pub mod interrupts;
pub mod segments;
pub mod registers;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub(super) enum PrivilegeLevel {
    Ring0 = 0x00,
    Ring1 = 0x01,
    Ring2 = 0x02,
    Ring3 = 0x03,
}
