mod interrupt_descriptor_table;
mod interrupt_routines;

use core::fmt;

use interrupt_descriptor_table::InterruptDescriptorTable;
use interrupt_routines::*;

use crate::cpu::interrupts::interrupt_descriptor_table::GateType;
use crate::cpu::{PrivilegeLevel, registers::RFlags, segments::SegmentSelector};
use crate::memory::VirtualAddress;

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

#[repr(C)]
#[derive(Debug)]
pub struct InterruptStackFrame {
    instruction_pointer: VirtualAddress,
    code_segment: SegmentSelector,
    cpu_flags: RFlags,
    stack_pointer: VirtualAddress,
    stack_segment: SegmentSelector,
}

bitflags::bitflags! {
    #[derive(Copy, Clone)]
    #[repr(transparent)]
    struct PageFaultError : usize {
        /// (P) Present. When cleared, indicates the fault was caused by a non-present page. When
        /// set, the page fault was caused by a page-protection violation.
        const PRESENT = 1 << 0;
        /// (W) Write. Indicates the page fault was caused by a write access. When cleared,
        /// indicates that the fault was caused by a read access.
        const WRITE = 1 << 1;
        /// (U) User. Indicates the page fault was caused while CPL = 3. Not necessarily indicative
        /// of a privilege violation.
        const USER = 1 << 2;
        /// (R) Reserved write. Indicates one or more page directory entries contain reserved bits
        /// which are set to 1. This only applies when the PSE or PAE flags in CR4 are set to 1.
        const RESERVED_WRITE = 1 << 3;
        /// (I) Instructio fetch. Indicates the page fault was caused by an instruction fetch. This
        /// only applies when the No-Execute bit is enabled and supported.
        const INSTRUCTION_FETCH = 1 << 4;
        /// (PK) Protection key. Indicates the page fault was caused by a protection-key
        /// violation. The PKRU register or PKRS MSR (for user-mode and supervisor mode accesses
        /// respectively) specify the protection key rights.
        const PROTECTION_KEY = 1 << 5;
        /// (SS) Shadow stack. Indicates the page fault was caused by a shadow stack access.
        const SHADOW_STACK = 1 << 6;
        /// (SGX) Software guard extensions. Indicates the fault was caused by an SGX violation and
        /// is unrelated to ordinary paging.
        const SOFTWARE_GUARD_EXTENSIONS = 1 << 15;
    }
}

impl fmt::Debug for PageFaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageFaultError(")?;
        if !self.contains(Self::PRESENT) {
            write!(f, "ABSENT")?;
            if ! self.is_empty() {
                write!(f, " | ")?;
            }
        }
        bitflags::parser::to_writer_strict(self, &mut *f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
enum SelectorErrorTable {
    GDT,
    IDT,
    LDT,
}

impl TryFrom<usize> for SelectorErrorTable {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0b00 => Ok(Self::GDT),
            0b01 | 0b11 => Ok(Self::IDT),
            0b10 => Ok(Self::LDT),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct SegmentSelectorError(usize);

impl SegmentSelectorError {
    /// Indicates whether the exception orginiated externally to the processor.
    pub const fn external(&self) -> bool {
        self.0 & 1 == 1
    }

    pub fn table(&self) -> SelectorErrorTable {
        (self.0 >> 1 & 0b11).try_into().unwrap()
    }

    pub const fn index(&self) -> u16 {
        (self.0 & 0xFFF8) as u16
    }

    pub const fn is_present(&self) -> bool {
        self.0 != 0
    }
}

impl fmt::Debug for SegmentSelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_present() {
            f.write_str("SegmentSelectorError::None")
        } else {
            f.debug_struct("SegmentSelectorError")
                .field("External", &self.external())
                .field("Table", &self.table())
                .field("Index", &self.index())
                .finish()
        }
    }
}

type Handler = extern "x86-interrupt" fn(InterruptStackFrame);
type HandlerWithError<T> = extern "x86-interrupt" fn(InterruptStackFrame, error: T);
type DivergingHandler = extern "x86-interrupt" fn(InterruptStackFrame) -> !;
type DivergingHandlerWithError<T> = extern "x86-interrupt" fn(InterruptStackFrame, error: T) -> !;

pub fn init() {
    let mut idt = InterruptDescriptorTable::new();

    idt.divide_error //
        .set_handler(divide_error_handler);

    idt.debug
        .set_handler(debug_handler)
        .set_gate_type(GateType::Trap);

    idt.non_maskable_interrupt
        .set_handler(non_maskable_interrupt_handler);

    idt.breakpoint
        .set_handler(breakpoint_handler)
        .set_gate_type(GateType::Trap)
        .set_privilege_level(PrivilegeLevel::Ring3);

    idt.overflow
        .set_handler(overflow_handler)
        .set_gate_type(GateType::Trap)
        .set_privilege_level(PrivilegeLevel::Ring3);

    idt.bound_range_exceeded
        .set_handler(bound_range_exceeded_handler);

    idt.invalid_opcode //
        .set_handler(invalid_opcode_handler);

    idt.device_not_available
        .set_handler(device_not_available_handler);

    idt.double_fault //
        .set_handler(double_fault_handler);

    idt.invalid_tss.set_handler(invalid_tss_handler);

    idt.segment_not_present
        .set_handler(segment_not_present_handler);

    idt.stack_segment_fault
        .set_handler(stack_segment_fault_handler);

    idt.general_protection_fault
        .set_handler(general_protx_fault_handler);

    idt.page_fault //
        .set_handler(page_fault_handler);

    idt.x87_floating_point_exception
        .set_handler(x87_floating_point_exception_handler);

    idt.alignment_check
        .set_handler(alignement_check_handler)
        .set_privilege_level(PrivilegeLevel::Ring3);

    idt.machine_check //
        .set_handler(machine_check_handler);

    idt.simd_floating_point
        .set_handler(simd_floating_point_handler);

    idt.virtualization_exception
        .set_handler(virtualization_exception_handler);

    idt.control_protection_exception
        .set_handler(ctrl_protx_exception_handler);

    unsafe {
        IDT = idt;
        InterruptDescriptorTable::load(&raw const IDT);
    }
}
