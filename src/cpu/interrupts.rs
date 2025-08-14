mod interrupt_descriptor_table;
mod interrupt_routines;

use interrupt_descriptor_table::InterruptDescriptorTable;
use interrupt_routines::*;

use crate::{cpu::{registers::RFlags, segments::SegmentSelector}, memory::VirtualAddress};

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

#[repr(C)]
#[derive(Debug)]
pub(super) struct InterruptStackFrame {
    instruction_pointer: VirtualAddress,
    code_segment: SegmentSelector,
    cpu_flags: RFlags,
    stack_pointer: VirtualAddress,
    stack_segment: SegmentSelector,
}

type Isr = extern "x86-interrupt" fn(InterruptStackFrame);
type IsrWithError = extern "x86-interrupt" fn(InterruptStackFrame, error: usize);
// type AbortingISR = extern "x86-interrupt" fn(InterruptStackFrame) -> !;
// type AbortingISRWithError = extern "x86-interrupt" fn(InterruptStackFrame, error: usize) -> !;

pub fn init() {
    let mut idt = InterruptDescriptorTable::new();

    idt.divide_error.set_handler(divide_error_handler);
    idt.non_maskable_interrupt.set_handler(non_maskable_interrupt_handler);
    idt.breakpoint.set_handler(breakpoint_handler);
    idt.overflow.set_handler(overflow_handler);
    idt.bound_range_exceeded.set_handler(overflow_handler);
    idt.invalid_opcode.set_handler(invalid_opcode_handler);
    idt.device_not_available.set_handler(device_not_available_handler);
    idt.double_fault.set_handler(double_fault_handler);
    idt.coprocessor_segment_overrun.set_handler(coprocessor_segment_overrun_handler);
    idt.invalid_tss.set_handler(invalid_tss_handler);
    idt.segment_not_present.set_handler(segment_not_present_handler);
    idt.stack_segment_fault.set_handler(stack_segment_fault_handler);
    idt.general_protection_fault.set_handler(general_protection_fault_handler);
    idt.page_fault.set_handler(page_fault_handler);
    idt.x87_floating_point_exception.set_handler(x87_floating_point_exception_handler);
    idt.alignment_check.set_handler(alignement_check_handler);
    idt.machine_check.set_handler(machine_check_handler);
    idt.simd_floating_point.set_handler(simd_floating_point_handler);
    idt.virtualization_exception.set_handler(virtualization_exception_handler);
    idt.control_protection_exception.set_handler(control_protection_exception_handler);

    unsafe {
        IDT = idt;
        InterruptDescriptorTable::load(&raw const IDT);
    }
}
