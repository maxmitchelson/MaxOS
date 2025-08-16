use core::fmt::Debug;

use crate::cpu::PrivilegeLevel;

pub mod selectors {
    use super::*;

    pub const CODE: SegmentSelector =
        SegmentSelector::new(5, DescriptorTable::GDT, PrivilegeLevel::Ring0);
    pub const DATA: SegmentSelector =
        SegmentSelector::new(6, DescriptorTable::GDT, PrivilegeLevel::Ring0);
}

#[derive(Clone, Copy, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum DescriptorTable {
    GDT = 0,
    LDT = 1,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    pub const fn new(
        index: u16,
        descriptor_table: DescriptorTable,
        privilege_level: PrivilegeLevel,
    ) -> Self {
        Self(index << 3 | ((descriptor_table as u16) << 2) | privilege_level as u16)
    }

    fn index(&self) -> u16 {
        self.0 >> 3
    }

    fn descriptor_table(&self) -> DescriptorTable {
        match self.0 & 1 {
            0 => DescriptorTable::GDT,
            1 => DescriptorTable::LDT,
            _ => panic!("Bad descriptor table"),
        }
    }
}

impl Debug for SegmentSelector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SegmentSelector")
            .field("index", &self.index())
            .field("table", &self.descriptor_table())
            .finish()
    }
}
