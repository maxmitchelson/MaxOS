use core::cell::UnsafeCell;
use core::error;
use core::fmt;
use core::slice;

use spin::Once;

use crate::limine;
use crate::memory::*;
use crate::terminal::logger;

static ALLOCATOR_PTR: Once<AllocatorPtr> = Once::new();

const PAGE_SIZE: usize = 4096;

struct AllocatorPtr(UnsafeCell<BuddyAllocator>);
unsafe impl Send for AllocatorPtr {}
unsafe impl Sync for AllocatorPtr {}

pub fn init() {
    ALLOCATOR_PTR.call_once(|| {
        AllocatorPtr(UnsafeCell::new(
            BuddyAllocator::new_embedded(limine::acquire_memory_map().unwrap()).unwrap(),
        ))
    });
}

#[inline(always)]
pub fn allocate_exact(size: usize) -> PhysicalAddress {
    with_allocator(|a| a.allocate_exact(size))
}

#[inline(always)]
pub fn allocate(size: usize) -> PhysicalAddress {
    with_allocator(|a| a.allocate(size))
}

#[inline(always)]
pub fn free(address: PhysicalAddress) {
    with_allocator(|a| a.free(address))
}

#[inline]
pub fn with_allocator<F, R>(func: F) -> R
where
    F: Fn(&mut BuddyAllocator) -> R,
{
    let buddy = ALLOCATOR_PTR
        .get()
        .expect("CRITICAL [FR4]: Cannot opperate on uninitialized frame allocator");
    func(unsafe { &mut *buddy.0.get() })
}

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
pub enum InitializationError {
    NoUsableMemory,
    NotEnoughAvailableMemory,
    BadRange(PhysicalAddress, PhysicalAddress),
}

impl fmt::Display for InitializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoUsableMemory => write!(
                f,
                "Could not create the frame allocator because the provided memory map reports no usable memory"
            ),
            Self::NotEnoughAvailableMemory => write!(
                f,
                "Could not create the frame allocator because the provided memory map reports too few usable memory"
            ),
            Self::BadRange(start, end) => f.write_fmt(format_args!(
                "Could not reserve memory for range [{:?}..{:?}] because it is an invalid range",
                start, end
            )),
        }
    }
}

impl error::Error for InitializationError {}

#[derive(Debug)]
pub struct BuddyAllocator {
    region_start: PhysicalAddress,
    region_end: PhysicalAddress,
    max_order: u8,
    markers: *mut [usize],
    state_tree: *mut [BlockState],
}

impl BuddyAllocator {
    pub fn new_embedded(memory_map: limine::MemoryMap) -> Result<Self, InitializationError> {
        let (usable_start, usable_end) = Self::get_usable_region(memory_map)?;
        let max_order = Self::max_order_for_usable_region(usable_start, usable_end);

        let tree_size = Self::size_of_tree_for_order(max_order);
        let markers_size = Self::size_of_markers_for_order(max_order);
        let total_size = markers_size + tree_size;

        let data_start = Self::select_data_start(&memory_map, total_size)?;

        let markers_start = data_start;
        let tree_start = markers_start + markers_size;

        let state_tree = unsafe { Self::init_block_tree(tree_start, tree_size) };
        let markers = unsafe { Self::init_markers(markers_start, max_order as usize + 1) };

        let mut allocator = Self {
            region_start: align_up(tree_start + tree_size, PAGE_SIZE),
            region_end: usable_end,
            max_order,
            markers,
            state_tree,
        };

        allocator.set_reserved_from_mmap(memory_map)?;
        Ok(allocator)
    }

    fn get_usable_region(
        memory_map: limine::MemoryMap,
    ) -> Result<(PhysicalAddress, PhysicalAddress), InitializationError> {
        let mut usable = memory_map.usable_entries();

        let first = usable.next().ok_or(InitializationError::NoUsableMemory)?;
        let last = usable.next_back().unwrap_or(first);
        Ok((
            PhysicalAddress::from_u64(first.base),
            PhysicalAddress::from_u64(last.base + last.length),
        ))
    }

    unsafe fn init_markers(markers_start: PhysicalAddress, markers_size: usize) -> *mut [usize] {
        unsafe {
            let list = slice::from_raw_parts_mut(
                markers_start.to_virtual().to_ptr::<usize>(),
                markers_size,
            );

            for (i, ptr) in list.iter_mut().enumerate() {
                *ptr = 1 << i;
            }

            list
        }
    }

    #[inline]
    unsafe fn init_block_tree(tree_start: PhysicalAddress, tree_size: usize) -> *mut [BlockState] {
        unsafe {
            let block_tree = slice::from_raw_parts_mut(
                tree_start.to_virtual().to_ptr::<BlockState>(),
                tree_size,
            );

            block_tree
                .as_mut_ptr()
                .write_bytes(BlockState::Free as u8, tree_size);
            block_tree[0] = BlockState::Reserved;
            block_tree
        }
    }

    #[inline]
    /// Reserves memory using the limine-provided mmap.
    ///
    /// Does not assume that all unusable memory is contained in the memory map and uses the holes
    /// between [`USABLE`](limine::memory_map::EntryType::USABLE) entries for safety.
    fn set_reserved_from_mmap(
        &mut self,
        memory_map: limine::MemoryMap,
    ) -> Result<(), InitializationError> {
        let mut usable = memory_map.usable_entries();

        let first = usable.next().unwrap();
        let mut previous_end = PhysicalAddress::from_u64(first.base + first.length);

        for entry in usable {
            let current_start = PhysicalAddress::from_u64(entry.base);
            self.reserve_range(previous_end, current_start)?;
            previous_end = PhysicalAddress::from_u64(entry.base + entry.length);
        }

        self.reserve_all_after(previous_end);
        Ok(())
    }

    #[inline(always)]
    fn clamp_addr(&self, address: PhysicalAddress) -> PhysicalAddress {
        address.clamp(self.region_start, self.region_end)
    }

    #[inline]
    pub fn reserve_range(
        &mut self,
        start: PhysicalAddress,
        end: PhysicalAddress,
    ) -> Result<(), InitializationError> {
        if end <= start {
            return Err(InitializationError::BadRange(start, end));
        }

        let first_block = self.page_block_from(self.clamp_addr(align_down(start, PAGE_SIZE)));
        let last_block = self.page_block_from(self.clamp_addr(align_up(end, PAGE_SIZE)));
        let offset = Self::offset_for_order(self.max_order);

        for block in first_block + offset..last_block + offset {
            self.set_state(block, BlockState::Reserved);
            self.update_ancestors(block);
        }
        Ok(())
    }

    #[inline]
    pub fn reserve_all_after(&mut self, address: PhysicalAddress) {
        let block = self.page_block_from(self.clamp_addr(align_down(address + 1, PAGE_SIZE)));
        let offset = Self::offset_for_order(self.max_order);

        for block in block + offset..self.state_tree.len() {
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
                if self.state(block).is_free() && self.state(buddy).is_free() {
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

    #[inline]
    fn mark_subtree(&mut self, block: usize, state: BlockState) {
        let mut level_size = 1;
        let mut level_start = block;

        while level_start < self.state_tree.len() {
            for i in level_start..level_start + level_size {
                if self.state(i) != BlockState::Reserved {
                    self.set_state(i, state);
                }
            }
            level_start <<= 1;
            level_size <<= 1;
        }
    }

    #[inline(always)]
    fn markers(&self) -> &[usize] {
        unsafe { &*self.markers }
    }

    #[inline(always)]
    fn markers_mut(&mut self) -> &mut [usize] {
        unsafe { &mut *self.markers }
    }

    #[inline(always)]
    fn marker_for(&self, order: u8) -> usize {
        usize::max(1 << order, self.markers()[order as usize])
    }

    fn set_marker_min(&mut self, order: u8, min: usize) {
        let order = order as usize;
        unsafe {
            let markers = &mut *self.markers;
            if markers[order] > min {
                markers[order] = min;
            }
        }
    }

    #[inline(always)]
    fn state_tree(&self) -> &[BlockState] {
        unsafe { &*self.state_tree }
    }

    #[inline(always)]
    fn state_tree_mut(&mut self) -> &mut [BlockState] {
        unsafe { &mut *self.state_tree }
    }

    #[inline(always)]
    fn state(&self, block: usize) -> BlockState {
        self.state_tree()[block]
    }

    #[inline(always)]
    fn set_state(&mut self, block: usize, state: BlockState) {
        self.state_tree_mut()[block] = state;
    }

    #[inline(always)]
    pub fn allocate_exact(&mut self, size: usize) -> PhysicalAddress {
        self.allocate_order(self.order_for_size(size).unwrap())
    }

    #[inline(always)]
    pub fn allocate(&mut self, size: usize) -> PhysicalAddress {
        assert!(size != 0);
        if is_aligned(size, PAGE_SIZE) && is_power_of_two(size) {
            self.allocate_exact(size)
        } else {
            let mut reverse_order = 0;
            while PAGE_SIZE << reverse_order < size {
                reverse_order += 1;

                if reverse_order > self.max_order {
                    panic!(
                        "Unsupported allocation for size {}, max supported size is {}",
                        size,
                        PAGE_SIZE << self.max_order
                    );
                }
            }

            self.allocate_order(self.max_order - reverse_order)
        }
    }

    #[inline]
    fn allocate_block(&mut self, block: usize, order: u8) -> PhysicalAddress {
        self.mark_subtree(block, BlockState::Allocated);
        self.update_ancestors(block);
        self.region_start + self.size_for_order(order) * (block - (1 << order))
    }

    #[inline]
    pub fn allocate_order(&mut self, order: u8) -> PhysicalAddress {
        let first = self.marker_for(order);
        let last = 2 << order;

        for block in first..last {
            if self.state(block).is_free() {
                self.markers_mut()[order as usize] = block + 1;
                return self.allocate_block(block, order);
            }
        }
        panic!("[FR0]: No free block for order size {order} in frame_allocator");
    }

    #[inline]
    pub fn free(&mut self, address: PhysicalAddress) {
        let val = (address - self.region_start).value() >> PAGE_SIZE.trailing_zeros();

        let rev_order = if val == 0 {
            self.max_order
        } else {
            val.trailing_zeros() as u8
        };

        let order_offset = Self::offset_for_order(self.max_order - rev_order);
        let block_offset = val >> rev_order;

        let mut block = order_offset + block_offset;
        let mut order = self.max_order - rev_order;

        while !matches!(self.state(block), BlockState::Allocated) {
            block <<= 1;
            order += 1;

            if block >= self.state_tree.len() {
                panic!(
                    "[FR1]: Could not free because no allocated block was found for address: {address:?}"
                );
            }
        }

        self.set_marker_min(order, block);
        self.mark_subtree(block, BlockState::Free);
        self.update_ancestors(block);
    }

    #[inline(always)]
    fn size_for_order(&self, order: u8) -> usize {
        PAGE_SIZE << (self.max_order - order)
    }

    #[inline(always)]
    fn order_for_size(&self, size: usize) -> Option<u8> {
        if is_aligned(size, PAGE_SIZE) && is_power_of_two(size) {
            return Some(
                self.max_order - (size.trailing_zeros() - PAGE_SIZE.trailing_zeros()) as u8,
            );
        }
        None
    }

    #[inline(always)]
    fn max_order_for_usable_region(
        usable_start: PhysicalAddress,
        usable_end: PhysicalAddress,
    ) -> u8 {
        let length = (usable_end - usable_start).value();
        let mut order = 0;
        let mut block_factor = (length - 1) >> PAGE_SIZE.trailing_zeros();

        while block_factor != 0 {
            block_factor >>= 1;
            order += 1;
        }

        order
    }

    fn size_of_tree_for_order(order: u8) -> usize {
        1 << (order + 1)
    }

    fn size_of_markers_for_order(order: u8) -> usize {
        (order as usize + 1) * size_of::<usize>()
    }

    fn select_data_start(
        memory_map: &limine::MemoryMap,
        data_size: usize,
    ) -> Result<PhysicalAddress, InitializationError> {
        let mut possible_data_pos = memory_map
            .usable_entries()
            .filter(|e| e.length as usize >= data_size);

        match possible_data_pos.next() {
            Some(region) => Ok(PhysicalAddress::from_u64(region.base)),
            None => Err(InitializationError::NotEnoughAvailableMemory),
        }
    }

    // TEST: should be REDESIGNED and marked as test when #4 is implemented
    pub fn stress(&mut self) {
        let offset = 1 << self.max_order;
        let mut count = 0usize;

        for i in offset..offset + offset {
            if self.state(i) == BlockState::Free {
                count += 1;
            }
        }

        logger::debug!(
            "Initial free {}KiB block count: {}",
            PAGE_SIZE / 1024,
            count
        );

        logger::debug!("Allocating all blocks");
        for i in 0..count - 1 {
            let frame = self.allocate_exact(PAGE_SIZE);
            let frame = unsafe {
                let ptr = frame.to_virtual().to_ptr::<u8>();
                ptr.write_bytes((i & 0xFF) as u8, PAGE_SIZE);
                slice::from_raw_parts_mut(ptr, PAGE_SIZE)
            };

            if i % 100000 == 0 {
                logger::debug!("i: {}", i);
            }

            if frame.iter().any(|&b| b != (i & 0xFF) as u8) {
                logger::error!("invalid read/write for frame {}", i);
            }
        }

        let frame = self.allocate_exact(PAGE_SIZE);

        let mut count = 0;
        for i in offset..offset + offset {
            if self.state(i) == BlockState::Free {
                count += 1;
            }
        }

        logger::debug!("Allocator fill success status: [{}]", count == 0);
        logger::debug!("Address for last allocated frame: {:?}", frame);
        logger::debug!("Freeing and reallocating last allocated frame");
        self.free(frame);
        let new_frame = self.allocate_exact(PAGE_SIZE);
        if new_frame == frame {
            logger::debug!("Same frame recieved");
        } else {
            logger::error!("Different frame received");
        }

        logger::debug!(
            "Freeing all allocated blocks (includes blocks allocated outside this test)"
        );
        let mut addr = self.region_start;
        for i in offset..offset + offset {
            if self.state(i) == BlockState::Allocated {
                self.free(addr);
                assert!(self.state(i) == BlockState::Free)
            }
            addr += PAGE_SIZE;
        }

        logger::debug!("Freed all possible blocks")
    }
}
