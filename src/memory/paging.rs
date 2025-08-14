use core::{
    arch::asm,
    fmt::{self},
    ops::{Index, IndexMut},
};

use crate::cpu;

use super::PhysicalAddress;

/// A 64-bit page table.
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; Self::ENTRY_COUNT],
}

impl PageTable {
    const ENTRY_COUNT: usize = 512;

    #[inline]
    pub const fn new() -> Self {
        const EMPTY_ENTRY: PageTableEntry = PageTableEntry::new();
        Self {
            entries: [EMPTY_ENTRY; Self::ENTRY_COUNT],
        }
    }

    pub fn clear(&mut self) {
        self.entries.fill(PageTableEntry::new());
    }

    pub fn entries(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

/// A 64-bit page table entry.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    const ADDRESS_MASK: usize = 0x000F_FFFF_FFFF_F000;

    #[inline(always)]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress::from(self.0 & Self::ADDRESS_MASK)
    }

    #[inline]
    pub fn flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::from_bits_truncate(self.0)
    }

    #[inline]
    pub fn set_flags(&mut self, flags: PageTableEntryFlags) {
        self.0 = self.address().value() | flags.bits();
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageTableEntry")
            .field("Address", &self.address())
            .field("Flags", &self.flags().0)
            .finish()
    }
}

bitflags::bitflags! {
    /// Bit flags for page table entries.
    #[derive(PartialEq, Eq, Debug, Clone, Copy)]
    pub struct PageTableEntryFlags: usize {
        /// (P) Indicates whether the page is loaded in physical memory.
        const PRESENT = 1;
        /// (RW) Determines if write access to frames mapped by this page or children ones is permitted.
        /// When this bit is cleared the corresponding frames are read-only.
        const WRITABLE = 1 << 1;
        /// (US) Controls access to the page based on privilege level. When cleared, access is denied in
        /// ring 3 contexts.
        const USER_ACCESSIBLE = 1 << 2;
        /// (PWT) When this bit is set, use a `write-back` cahing policy, otherwise use a `write-through`
        /// caching policy.
        const CACHING_POLICY = 1 << 3;
        /// (PCD) Controls whether the page is cached or not.
        const DISABLE_CACHING = 1 << 4;
        /// (A) Set by the CPU when an entry is accessed. If needed, the responsability to clear it
        /// falls on the OS.
        const ACCESSED = 1 << 5;
        /// (D) Set by the CPU when a page has been written to.
        const DIRTY = 1 << 6;
        /// (PS) Indicates that the entry is a huge page and the lowest level of the page-translation hierarchy
        const HUGE_PAGE = 1 << 7;
        /// (G) Indicates the page is a global page and prevents it from getting invalidated when
        /// switching address space.
        const GLOBAL_PAGE = 1 << 8;

        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_1 = 1 << 9;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_2 = 1 << 10;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_3 = 1 << 11;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_4 = 1 << 52;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_5 = 1 << 53;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_6 = 1 << 54;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_7 = 1 << 55;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_8 = 1 << 56;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_9 = 1 << 57;
        /// (AVL) This bit is not interpreted by the CPU and is available to use if needed.
        const __AVAILABLE_10 = 1 << 58;

        /// (AVL/MPK) If memory protection keys are enabled (CR4.PKE=1), this bit is reserved, otherwise it's
        /// available to use.
        const __MPK_OR_AVL_1 = 1 << 59;
        /// (AVL/MPK) If memory protection keys are enabled (CR4.PKE=1), this bit is reserved, otherwise it's
        /// available to use.
        const __MPK_OR_AVL_2 = 1 << 60;
        /// (AVL/MPK) If memory protection keys are enabled (CR4.PKE=1), this bit is reserved, otherwise it's
        /// available to use.
        const __MPK_OR_AVL_3 = 1 << 61;
        /// (AVL/MPK) If memory protection keys are enabled (CR4.PKE=1), this bit is reserved, otherwise it's
        /// available to use.
        const __MPK_OR_AVL_4 = 1 << 62;

        /// (NX) When no-execute page-protection is enabled, this bit controls the ability to execute
        /// code from all pages mapped by this table entry. Otherwise, it should be set to 0.
        const NO_EXECUTE = 1 << 63;
    }
}

pub fn get_active_level_4_table(offset: usize) -> &'static mut PageTable {
    let (physical, _) = cpu::registers::Cr3::read();
    let page_table_ptr = physical.to_virtual().to_ptr::<PageTable>();

    unsafe { &mut *page_table_ptr }
}
