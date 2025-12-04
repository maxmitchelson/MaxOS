use crate::{
    cpu::interrupts::{InterruptStackFrame as ISF, PageFaultError, SegmentSelectorError as SSErr},
    terminal::logger,
};

pub(super) extern "x86-interrupt" fn divide_error_handler(stack_frame: ISF) {
    panic!("DIVIDE ERROR INTERRUPT stack_frame: {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn debug_handler(stack_frame: ISF) {
    logger::warning!("DEBUG TRAP stack_frame: {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: ISF) {
    panic!("NON-MASKABLE INTERRUPT stack_frame: {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn breakpoint_handler(stack_frame: ISF) {
    logger::warning!("BREAKPOINT TRAP stack_frame: {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn overflow_handler(stack_frame: ISF) {
    logger::warning!("OVERFLOW TRAP stack_frame: {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: ISF) {
    panic!("BOUND RANGE INTERRUPT stack_frame: {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: ISF) {
    panic!("INVALID OPCODE INTERRUPT stack_frame: {:#?}", stack_frame);
}

pub(super) extern "x86-interrupt" fn device_not_available_handler(stack_frame: ISF) {
    panic!(
        "DEVICE NOT AVAILABLE INTERRUPT stack_frame: {:#?}",
        stack_frame
    );
}

pub(super) extern "x86-interrupt" fn double_fault_handler(stack_frame: ISF, error: usize) -> ! {
    panic!(
        "DOUBLE FAULT INTERRUPT stack_frame: {:#?}, error: {}",
        stack_frame, error
    );
}

pub(super) extern "x86-interrupt" fn invalid_tss_handler(stack_frame: ISF, error: SSErr) {
    panic!(
        "INVALID TSS INTERRUPT stack_frame: {:#?}, error: {:?}",
        stack_frame, error
    );
}

pub(super) extern "x86-interrupt" fn segment_not_present_handler(stack_frame: ISF, error: SSErr) {
    panic!(
        "SEGMENT NOT PRESENT INTERRUPT stack_frame: {:#?}, error: {:?}",
        stack_frame, error
    );
}

pub(super) extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: ISF, error: SSErr) {
    panic!(
        "STACK SEGMENT FAULT INTERRUPT stack_frame: {:#?}, error: {:?}",
        stack_frame, error
    );
}

pub(super) extern "x86-interrupt" fn general_protx_fault_handler(stack_frame: ISF, error: SSErr) {
    panic!(
        "GENERAL PROTECTION FAULT INTERRUPT stack_frame: {:#?}, error: {:?}",
        stack_frame, error
    );
}

pub(super) extern "x86-interrupt" fn page_fault_handler(stack_frame: ISF, error: PageFaultError) {
    panic!(
        "PAGE FAULT INTERRUPT stack_frame: {:#?}, error: {:?}",
        stack_frame, error
    );
}

pub(super) extern "x86-interrupt" fn x87_floating_point_exception_handler(stack_frame: ISF) {
    panic!(
        "x87 FLOATING POINT EXCEPTION INTERRUPT stack_frame: {:#?}",
        stack_frame
    );
}

pub(super) extern "x86-interrupt" fn alignement_check_handler(stack_frame: ISF, error: usize) {
    panic!(
        "ALIGNMENT CHECK INTERRUPT stack_frame: {:#?}, error: {}",
        stack_frame, error
    );
}

pub(super) extern "x86-interrupt" fn machine_check_handler(stack_frame: ISF) -> ! {
    panic!("MACHINE CHECK INTERRUPT stack_frame: {:#?}", stack_frame,);
}

pub(super) extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: ISF) {
    panic!(
        "SIMD FLOATING POINT INTERRUPT stack_frame: {:#?}",
        stack_frame,
    );
}

pub(super) extern "x86-interrupt" fn virtualization_exception_handler(stack_frame: ISF) {
    panic!(
        "VIRTUALIZATION EXCEPTION INTERRUPT stack_frame: {:#?}",
        stack_frame,
    );
}

pub(super) extern "x86-interrupt" fn ctrl_protx_exception_handler(stack_frame: ISF, error: usize) {
    panic!(
        "CONTROL PROTECTION EXCEPTION INTERRUPT stack_frame: {:#?}, error: {}",
        stack_frame, error
    );
}
