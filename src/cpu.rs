use crate::memory::VirtualAddress;

pub mod interrupts;
pub mod segments;
pub mod registers;

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: VirtualAddress,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub(super) enum PrivilegeLevel {
    Ring0 = 0x00,
    Ring1 = 0x01,
    Ring2 = 0x02,
    Ring3 = 0x03,
}

impl TryFrom<u8> for PrivilegeLevel {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ring0),
            1 => Ok(Self::Ring1),
            2 => Ok(Self::Ring2),
            3 => Ok(Self::Ring3),
            _ => Err(()),
        }
    }
}
