mod interrupt_descriptor_table;
mod interrupt_routines;

use interrupt_descriptor_table::InterruptDescriptorTable;
use interrupt_routines::*;

use crate::memory::VirtualAddress;

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

#[repr(C)]
pub(super) struct InterruptStackFrame {
    instruction_pointer: VirtualAddress,
    code_segment: usize,
    cpu_flags: usize,
    stack_pointer: VirtualAddress,
    stack_segment: usize,
}

type Isr = extern "x86-interrupt" fn(InterruptStackFrame);
type IsrWithError = extern "x86-interrupt" fn(InterruptStackFrame, error: usize);
// type AbortingISR = extern "x86-interrupt" fn(InterruptStackFrame) -> !;
// type AbortingISRWithError = extern "x86-interrupt" fn(InterruptStackFrame, error: usize) -> !;

pub fn init() {
    let mut idt = InterruptDescriptorTable::new();
    idt.divide_error.set_handler(divide_error_handler);

    unsafe {
        IDT = idt;
        InterruptDescriptorTable::load(&raw const IDT);
    }
}
