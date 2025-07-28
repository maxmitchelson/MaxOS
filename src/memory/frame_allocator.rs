use crate::{limine, memory::PhysicalAddress};
use ::limine::memory_map;

const MAX_ORDER: usize = 10;

pub struct BuddyAllocator<'a> {
    region: &'a mut [u8],
}

impl<'a> BuddyAllocator<'a> {
    pub fn new_embedded(region_start: PhysicalAddress, region_end: PhysicalAddress) {
        let memory_map = *limine::BOOT_MEMORY_MAP;
    }
}

pub struct FrameAllocator {
    memory_map: &'static [&'static memory_map::Entry],
    current_entry_idx: usize,
    curent_offset: usize,
}

impl FrameAllocator {
    pub fn new() -> Self {
        Self {
            memory_map: *limine::BOOT_MEMORY_MAP,
            current_entry_idx: 0,
            curent_offset: 0,
        }
    }

    // FIXME: Causes obvious external fragmentation
    pub fn allocate(&mut self) -> PhysicalAddress {
        loop {
            if self.current_entry_idx >= self.memory_map.len() {
                panic!("Not enough memory remaining to allocate the required frame");
            }

            let current_entry = self.memory_map[self.current_entry_idx];
            if current_entry.entry_type == memory_map::EntryType::USABLE
                && current_entry.length as usize + self.curent_offset >= 4096
            {
                self.curent_offset += 4096;
                return PhysicalAddress::from(current_entry.base as usize + self.curent_offset);
            }

            self.current_entry_idx += 1;
            self.curent_offset = 0;
        }
    }
}
