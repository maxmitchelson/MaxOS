use core::slice;

use crate::limine::BootMemoryMap;
use crate::{memory::*, println};

const PAGE_SIZE: usize = 4096;
const PAGE_SIZE_ORDER: usize = 12;

type MemoryRegion = (PhysicalAddress, PhysicalAddress);

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockState {
    Free = 0b00,
    Allocated = 0b01,
    Split = 0b10,
    Full = 0b11,
    Reserved = 0xE7,
}

impl BlockState {
    #[inline(always)]
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Free | Self::Split)
    }

    #[inline(always)]
    pub const fn is_free(&self) -> bool {
        matches!(self, Self::Free)
    }
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
            Some(region) => PhysicalAddress::from_u64(region.base),
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
            PhysicalAddress::from_u64(first.base),
            PhysicalAddress::from_u64(last.base + last.length),
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
    /// Reserves memory using the limine-provided mmap.
    ///
    /// Does not assume that all unusable memory is contained in the memory map and uses the holes
    /// between [`USABLE`](limine::memory_map::EntryType::USABLE) entries for safety.
    pub fn set_reserved_from_mmap(&mut self, memory_map: BootMemoryMap) {
        let mut usable = memory_map.usable_entries();

        let first = usable.next().unwrap();
        let mut previous_end = PhysicalAddress::from_u64(first.base + first.length);

        for entry in usable {
            let current_start = PhysicalAddress::from_u64(entry.base);
            self.reserve_range(previous_end, current_start);
            previous_end = PhysicalAddress::from_u64(entry.base + entry.length);
        }

        self.reserve_all_after(previous_end);
    }

    #[inline]
    pub fn reserve_range(&mut self, start: PhysicalAddress, end: PhysicalAddress) {
        if end <= start {
            panic!("Cannot reserve memory: bad range ({start:?}, {end:?})");
        }

        let first_block = self.page_block_from(align_down(start, PAGE_SIZE));
        let last_block = self.page_block_from(align_up(end, PAGE_SIZE));
        let offset = Self::offset_for_order(self.max_order);

        for block in first_block + offset..last_block + offset {
            self.set_state(block, BlockState::Reserved);
            self.update_ancestors(block);
        }
    }

    #[inline]
    pub fn reserve_all_after(&mut self, address: PhysicalAddress) {
        let block = self.page_block_from(align_down(address + 1, PAGE_SIZE));
        let offset = Self::offset_for_order(self.max_order);

        for block in block + offset..self.block_tree.len() {
            self.set_state(block, BlockState::Reserved);
            self.update_ancestors(block);
        }
    }

    #[inline(always)]
    fn page_block_from(&self, address: PhysicalAddress) -> usize {
        assert!(is_aligned(address, PAGE_SIZE));
        (address - self.region_start).value() / PAGE_SIZE
    }

    #[inline(always)]
    fn offset_for_order(order: u8) -> usize {
        1 << order
    }

    #[inline(always)]
    fn buddy(idx: usize) -> Option<usize> {
        if idx > 1 { Some(idx ^ 1) } else { None }
    }

    #[inline(always)]
    fn parent(idx: usize) -> Option<usize> {
        if idx > 1 { Some(idx >> 1) } else { None }
    }

    #[inline]
    fn update_ancestors(&mut self, block: usize) {
        let mut block = block;
        while let Some(parent) = Self::parent(block) {
            if let Some(buddy) = Self::buddy(block) {
                if self.state(block).is_usable() && self.state(buddy).is_usable() {
                    self.set_state(parent, BlockState::Free);
                } else if self.state(block).is_usable() || self.state(buddy).is_usable() {
                    self.set_state(parent, BlockState::Split);
                } else {
                    self.set_state(parent, BlockState::Full);
                }
            }

            block = parent;
        }
    }

    #[inline(always)]
    fn state(&self, block: usize) -> BlockState {
        self.block_tree[block]
    }

    #[inline(always)]
    fn set_state(&mut self, block: usize, state: BlockState) {
        self.block_tree[block] = state;
    }

    #[inline(always)]
    pub fn allocate(&mut self, size: usize) -> PhysicalAddress {
        self.allocate_order(self.order_for_size(size).unwrap())
    }

    #[inline]
    pub fn allocate_block(&mut self, block: usize, order: u8) -> PhysicalAddress {
        self.block_tree[block] = BlockState::Allocated;
        self.update_ancestors(block);
        self.region_start + self.size_for_order(order) * (block - (1 << order))
    }

    #[inline]
    pub fn allocate_order(&mut self, order: u8) -> PhysicalAddress {
        let mut current_order = 0;
        let mut current_block = 1;
        loop {
            if current_order == order {
                if self.state(current_block).is_free() {
                    return self.allocate_block(current_block, order);
                } else if let Some(buddy) = Self::buddy(current_block) {
                    if self.state(buddy).is_free() {
                        return self.allocate_block(buddy, order);
                    }
                }

                // Should be caught earlier unless allocating order = 0
                panic!("CRITICAL [FR0]: No free block for order size {order} in frame allocator");
            }

            if self.state(current_block).is_usable() {
                current_block <<= 1;
                current_order += 1;
                continue;
            } else if let Some(buddy) = Self::buddy(current_block) {
                if self.state(buddy).is_usable() {
                    current_block = buddy << 1;
                    current_order += 1;
                    continue;
                }
            }

            panic!("CRITICAL [FR1]: No free block for order size {order} in frame allocator");
        }
    }

    #[inline]
    pub fn free(&mut self, address: PhysicalAddress) {
        let val = (address - self.region_start).value() >> PAGE_SIZE_ORDER;

        let rev_order = if val == 0 {
            self.max_order
        } else {
            val.trailing_zeros() as u8
        };

        let order_offset = Self::offset_for_order(self.max_order - rev_order);
        let block_offset = val >> rev_order;

        let mut block = order_offset + block_offset;

        while !matches!(self.state(block), BlockState::Allocated) {
            block <<= 1;

            if block >= self.block_tree.len() {
                panic!(
                    "CRITICAL [FR2]: Could not free because no allocated block was found for address: {address:?}"
                );
            }
        }

        self.set_state(block, BlockState::Free);
        self.update_ancestors(block);
    }

    #[inline(always)]
    fn size_for_order(&self, order: u8) -> usize {
        PAGE_SIZE << (self.max_order - order)
    }

    #[inline(always)]
    fn order_for_size(&self, size: usize) -> Option<u8> {
        if is_aligned(size, PAGE_SIZE) && is_power_of_two(size) {
            return Some(self.max_order - (size >> PAGE_SIZE_ORDER).trailing_zeros() as u8);
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

    // TEST: should be marked as test when #4 is implemented
    pub fn stress(&mut self) {
        let offset = 1 << self.max_order;
        let mut count = 0usize;

        for i in offset..offset + offset {
            if self.block_tree[i] == BlockState::Free {
                count += 1;
            }
        }

        crate::println!("{} remaining free blocks", count);

        for i in 0..count - 1 {
            let frame = self.allocate(PAGE_SIZE);
            let frame =
                unsafe { slice::from_raw_parts_mut(frame.to_virtual().to_ptr::<u8>(), PAGE_SIZE) };

            frame.fill((i & 0xFF) as u8);

            if frame.iter().any(|&b| b != (i & 0xFF) as u8) {
                println!("ERROR: invalid read/write for frame {}", i);
            }
        }

        let frame = self.allocate(PAGE_SIZE);
        crate::println!("Allocator fill completed");

        let mut count = 0;
        for i in offset..offset + offset {
            if self.block_tree[i] == BlockState::Free {
                count += 1;
            }
        }
        crate::println!("{} remaining free blocks", count);
        crate::println!("last frame: {:?}", frame);

        self.free(frame);
        crate::println!("Last block freed");
        self.allocate(PAGE_SIZE);
        crate::println!("Last block reallocated");

        // self.allocate(PAGE_SIZE);
        // crate::println!("Should have paniced");

        let mut addr = self.region_start;
        for i in offset..offset + offset {
            if self.block_tree[i] == BlockState::Allocated {
                self.free(addr);
                assert!(self.block_tree[i] == BlockState::Free)
            }
            addr += PAGE_SIZE;
        }

        println!("Freed all possible blocks")
    }
}
