use crate::cpu::interrupts::InterruptStackFrame;

// Cannot return never type from extern interrupt because of regression in current nightly build
pub(super) extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    crate::println!("interrupt {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error: usize) {}
pub(super) extern "x86-interrupt" fn coprocessor_segment_overrun_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error: usize) {}
pub(super) extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error: usize) {}
pub(super) extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error: usize) {}
pub(super) extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error: usize) {}
pub(super) extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error: usize) {}
pub(super) extern "x86-interrupt" fn x87_floating_point_exception_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn alignement_check_handler(stack_frame: InterruptStackFrame, error: usize) {}
pub(super) extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn virtualization_exception_handler(stack_frame: InterruptStackFrame) {}
pub(super) extern "x86-interrupt" fn control_protection_exception_handler(stack_frame: InterruptStackFrame, error: usize) {}

