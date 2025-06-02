use crate::serial_println;
use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};

pub extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: DIVIDE ERROR\n{:#?}", stack_frame);
    loop {}
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
    loop {}
}

pub extern "x86-interrupt" fn pagefault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
    serial_println!("EXCEPTION: PAGE FAULT\n{:#?}", stack_frame);
    loop {}
}
