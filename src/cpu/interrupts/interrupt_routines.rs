use crate::cpu::interrupts::InterruptStackFrame;

// Cannot return never type from extern interrupt because of regression in current nightly build
pub(super) extern "x86-interrupt" fn divide_error_handler(_stack_frame: InterruptStackFrame) {
    crate::println!("interrupt");
}
