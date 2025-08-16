use core::arch::asm;
use core::fmt;

use crate::memory::PhysicalAddress;

pub struct Cr3;
impl Cr3 {
    pub fn read() -> (PhysicalAddress, Cr3Flags) {
        let content: usize;
        unsafe { asm!("mov {}, cr3", out(reg) content) }
        let pdbr = content & !((1 << 12) - 1);
        let flags = Cr3Flags::from_bits_truncate(content);
        (pdbr.into(), flags)
    }
}

bitflags::bitflags! {
    #[derive(PartialEq, Eq, Clone, Copy)]
    #[repr(transparent)]
    pub struct Cr3Flags: usize {
        /// (PWT) Page-level write-through. Not used if bit 17 of CR4 is 1.
        const WRITE_THROUGH = 1 << 3;
        /// (PCD) Page-level cache disable. Not used if bit 17 of CR4 is 1.
        const CACHE_DISABLE = 1 << 4;
    }
}

bitflags::bitflags! {
    #[derive(PartialEq, Eq, Clone, Copy)]
    #[repr(transparent)]
    pub struct RFlags: usize {
        /// (CF) Carry flag.
        const CARRY = 1 << 0;
        /// (PF) Parity flag.
        const PARITY = 1 << 2;
        /// (AF) Auxiliary carry flag.
        const AUXILIARY_CARRY = 1 << 4;
        /// (ZF) Zero flag.
        const ZERO = 1 << 6;
        /// (SF) Sign flag.
        const SIGN = 1 << 7;
        /// (TF) Trap flag.
        const TRAP = 1 << 8;
        /// (IF) Interrupt enable flag.
        const INTERRUPT_ENABLE = 1 << 9;
        /// (DF) Direction flag.
        const DIRECTION = 1 << 10;
        /// (OF) Overflow flag.
        const OVERFLOW = 1 << 11;
        /// (IOPL) I/O privilege level.
        const IO_PRIVILEGE = 0b11 << 12;
        /// (NT) Nested task.
        const NESTED_TASK = 1 << 14;
        /// (RF) Resume flag.
        const RESUME = 1 << 16;
        /// (VM) Virtual-8086 mode.
        const VIRTUAL_8086_MODE = 1 << 17;
        /// (AC) Alignment check / Access control.
        const AC = 1 << 18;
        /// (VIF) Virtual interrupt flag.
        const VIRTUAL_INTERRUPT = 1 << 19;
        /// (VIP) Virtual interrupt pending.
        const VIRTUAL_INTERRUPT_PENDING = 1 << 20;
        /// (ID) ID flag.
        const ID = 1 << 21;

        const _ = 1 << 1 | 1 << 3 | 1 << 5 | 1 << 15 | !((1<<22) - 1);
    }
}

impl fmt::Debug for RFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RFlags(")?;
        bitflags::parser::to_writer_strict(self, &mut *f)?;
        write!(f, ")")
    }
}
