use crate::serial_println;
use x86_64::{
    VirtAddr,
    registers::control::Cr2,
    structures::idt::{InterruptStackFrame, PageFaultErrorCode},
};

macro_rules! exception_handler {
    ($name:ident, $msg:expr) => {
        pub extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame) {
            serial_println!("EXCEPTION: {}\n{:#?}", $msg, stack_frame);
            hlt_loop() // 需替换为恢复逻辑或panic处理
        }
    };
}

macro_rules! exception_handler_with_error_code {
    ($name:ident, $msg:expr) => {
        pub extern "x86-interrupt" fn $name(
            stack_frame: InterruptStackFrame,
            error_code: u64, // 统一使用u64接收错误码
        ) {
            serial_println!(
                "EXCEPTION: {} [ERR: {:#x}]\n{:#?}",
                $msg,
                error_code,
                stack_frame
            );
            hlt_loop()
        }
    };
}

// 无错误码异常 -------------------------------------------------
exception_handler!(divide_error_handler, "DIVIDE ERROR");
exception_handler!(debug_handler, "DEBUG");
exception_handler!(nmi_handler, "NON-MASKABLE INTERRUPT");
exception_handler!(overflow_handler, "OVERFLOW");
exception_handler!(bound_range_handler, "BOUND RANGE EXCEEDED");
exception_handler!(invalid_opcode_handler, "INVALID OPCODE");
exception_handler!(device_not_available_handler, "DEVICE NOT AVAILABLE");

// 有错误码异常 -------------------------------------------------
exception_handler_with_error_code!(invalid_tss_handler, "INVALID TSS");
exception_handler_with_error_code!(segment_not_present_handler, "SEGMENT NOT PRESENT");
exception_handler_with_error_code!(stack_segment_handler, "STACK-SEGMENT FAULT");
exception_handler_with_error_code!(general_protection_handler, "GENERAL PROTECTION FAULT");
exception_handler_with_error_code!(alignment_check_handler, "ALIGNMENT CHECK");

// 特殊处理异常 -------------------------------------------------
pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    // 必须标记为永不返回
    serial_println!(
        "CRITICAL: DOUBLE FAULT [ERR: {:#x}]\n{:#?}",
        error_code,
        stack_frame
    );
    panic!("SYSTEM HALT"); // 安全地停止系统
}

pub extern "x86-interrupt" fn pagefault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let fault_address = match Cr2::read() {
        Ok(addr) => addr,
        Err(_) => VirtAddr::zero(),
    };

    serial_println!(
        "EXCEPTION: PAGE FAULT at {:#x}\n \
         Cause: {:?}\n \
         Frame: {:#?}",
        fault_address,
        error_code,
        stack_frame
    );
    // 实际应执行页面分配回收逻辑
    hlt_loop()
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

#[inline(always)]
fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
