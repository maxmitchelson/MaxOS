use core::slice;

use crate::{
    limine::BootMemoryMap,
    memory::*,
};

const PAGE_SIZE: usize = 4096;
const PAGE_SIZE_ORDER: usize = 12;

type MemoryRegion = (PhysicalAddress, PhysicalAddress);

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockState {
    Free = 0x00,
    Allocated = 0x01,
    Split = 0x02,
    Reserved = 0xE7,
}

#[derive(Debug)]
pub enum FrallocError {
    NoUsableMemory,
    NotEnoughAvailableMemory,
}

#[derive(Debug)]
#[repr(align(4096))]
pub struct BuddyAllocator<'a> {
    region_start: PhysicalAddress,
    region_length: usize,
    max_order: u8,
    block_tree: &'a mut [BlockState],
}

// InFO: Managing max_order 25 = 128 GiB (2^25*4096 B) of memory results in 2^25 B = 32 MiB
// of allocator metadata. Setting an aribitrary limit of 128 GiB of physical memory would
// allow reserving a fixed 32 MiB of space for the allocator, reducing the complexity of reserving
// space for the allocator itself.
impl<'a> BuddyAllocator<'a> {
    pub fn new_embedded(memory_map: BootMemoryMap) -> Result<&'a mut Self, FrallocError> {
        let (region_start, region_end) = Self::get_usable_region(memory_map)?;

        let length = (region_end - region_start).value();
        let max_order = Self::max_order_for_length(length);
        let tree_size = Self::size_of_tree_for_order(max_order);
        let allocator_size = align_up(tree_size + size_of::<BuddyAllocator>(), PAGE_SIZE);

        let mut possible_allocator_pos = memory_map
            .usable_entries()
            .filter(|e| e.length as usize >= allocator_size);

        let allocator_start = match possible_allocator_pos.next() {
            Some(region) => PhysicalAddress::from(region.base as usize),
            None => return Err(FrallocError::NotEnoughAvailableMemory),
        };

        let allocator = unsafe {
            Self::init(
                allocator_start,
                align_up(allocator_start + allocator_size, PAGE_SIZE),
                region_end,
                max_order,
                allocator_size,
                tree_size,
            )
        };

        allocator.set_reserved_from_mmap(memory_map);

        Ok(allocator)
    }

    fn get_usable_region(memory_map: BootMemoryMap) -> Result<MemoryRegion, FrallocError> {
        let mut usable = memory_map.usable_entries();

        let first = usable.next().ok_or(FrallocError::NoUsableMemory)?;
        let last = usable.next_back().unwrap_or(first);

        Ok((
            PhysicalAddress::from(first.base as usize),
            PhysicalAddress::from((last.base + last.length) as usize),
        ))
    }

    #[inline]
    unsafe fn init(
        allocator_start: PhysicalAddress,
        region_start: PhysicalAddress,
        region_end: PhysicalAddress,
        max_order: u8,
        allocator_size: usize,
        tree_size: usize,
    ) -> &'a mut Self {
        unsafe {
            let allocator_space = slice::from_raw_parts_mut(
                allocator_start.to_virtual().to_ptr::<u8>(),
                allocator_size,
            );
            allocator_space.fill(core::mem::zeroed());

            let allocator = &mut *allocator_start.to_virtual().to_ptr::<BuddyAllocator>();
            let tree = &mut *slice::from_raw_parts_mut(
                (allocator_start + size_of::<BuddyAllocator>())
                    .to_virtual()
                    .to_ptr::<BlockState>(),
                tree_size,
            );

            tree.fill(BlockState::Free);
            tree[0] = BlockState::Reserved;

            allocator.region_start = region_start;
            allocator.region_length = (region_end - region_start).value();
            allocator.max_order = max_order;
            allocator.block_tree = tree;

            allocator
        }
    }

    #[inline]
    pub fn set_reserved_from_mmap(&mut self, memory_map: BootMemoryMap) {
        let mut usable = memory_map.usable_entries();

        let first = usable.next().unwrap();
        let mut start = PhysicalAddress::from((first.base + first.length) as usize);

        for entry in usable {
            self.reserve_range(start, PhysicalAddress::from(entry.base as usize));
            start = PhysicalAddress::from((entry.base + entry.length) as usize);
        }
    }

    #[inline]
    pub fn reserve_range(
        &mut self,
        start_address: PhysicalAddress,
        end_address: PhysicalAddress,
    ) {
        if end_address <= start_address {
            panic!("Cannot reserve memory: bad range ({start_address:?}, {end_address:?})");
        }

        let first_block =
            (align_down(start_address, PAGE_SIZE) - self.region_start).value() / PAGE_SIZE;
        let last_block = (align_up(end_address, PAGE_SIZE) - self.region_start).value() / PAGE_SIZE;

        let offset = 1 << self.max_order;

        for i in first_block + offset..last_block + offset {
            self.block_tree[i] = BlockState::Allocated;
            self.split_parent(i);
        }
    }

    #[inline(always)]
    fn buddy(idx: usize) -> Option<usize> {
        if idx == 1 { None } else { Some(idx ^ 1) }
    }

    #[inline(always)]
    fn parent(idx: usize) -> Option<usize> {
        if idx == 1 { None } else { Some(idx >> 1) }
    }

    #[inline]
    fn split_parent(&mut self, block: usize) {
        let mut block = block;
        while let Some(parent) = Self::parent(block) {
            if self.block_tree[parent] == BlockState::Split {
                break;
            } else {
                self.block_tree[parent] = BlockState::Split;
            }

            block = parent;
        }
    }

    #[inline(always)]
    pub fn allocate(&mut self, size: usize) -> PhysicalAddress {
        self.allocate_order(self.order_for_size(size).unwrap())
    }

    #[inline(always)]
    pub fn allocate_order(&mut self, order: u8) -> PhysicalAddress {
        let offset = 1 << order;
        let order_block_count = 1 << order;

        for i in offset..order_block_count + offset {
            if self.block_tree[i] == BlockState::Free {
                self.split_parent(i);
                return self.region_start + self.size_for_order(order) * (i-offset);
            }
        }

        panic!("No free block for order size {order}");
    }

    #[inline(always)]
    fn size_for_order(&self, order: u8) -> usize {
        PAGE_SIZE << (self.max_order-order)
    }

    #[inline(always)]
    fn order_for_size(&self, size: usize) -> Option<u8> {
        if is_aligned(size, PAGE_SIZE) && is_power_of_two(size) {
            return Some(self.max_order - (size >> 12).trailing_zeros() as u8);
        }
        None
    }

    #[inline(always)]
    fn max_order_for_length(length: usize) -> u8 {
        let mut order = 0;
        let mut block_factor = (length - 1) >> PAGE_SIZE_ORDER;

        while block_factor != 0 {
            block_factor >>= 1;
            order += 1;
        }

        order
    }

    #[inline(always)]
    fn size_of_tree_for_order(order: u8) -> usize {
        1 << (order + 1)
    }
}
