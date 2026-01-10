//! Interrupt and exception handlers
//!
//! This module contains the interrupt service routines (ISRs) and exception handlers
//! for the x86_64 architecture. It provides handlers for CPU exceptions, hardware
//! interrupts, and system calls.
//!
//! # Overview
//! The module defines handlers for:
//! - CPU exceptions (divide error, page fault, double fault, etc.)
//! - Hardware interrupts from the PIC (Programmable Interrupt Controller)
//! - Special handlers for critical system events
//!
//! # Macros
//! - [`exception_handler!`](macro.exception_handler.html) - Creates exception handlers without error codes
//! - [`exception_handler_with_error_code!`](macro.exception_handler_with_error_code.html) - Creates exception handlers with error codes
//! - [`pic_interrupt_handler!`](macro.pic_interrupt_handler.html) - Creates PIC interrupt handlers
//!
//! # Safety
//! This module contains interrupt handlers that run in a special context.
//! These functions must follow specific calling conventions and handle
//! system state carefully to avoid corruption.

#[allow(unused)]
use crate::interrupts::pic::{PICS, PIC_1_OFFSET};
use crate::serial_println;
use x86_64::{
    registers::control::Cr2,
    structures::idt::{InterruptStackFrame, PageFaultErrorCode},
    VirtAddr,
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
exception_handler!(x87_floating_point_handler, "x87 FLOATING POINT ERROR");

// 有错误码异常 -------------------------------------------------
exception_handler_with_error_code!(invalid_tss_handler, "INVALID TSS");
exception_handler_with_error_code!(segment_not_present_handler, "SEGMENT NOT PRESENT");
exception_handler_with_error_code!(stack_segment_handler, "STACK-SEGMENT FAULT");
exception_handler_with_error_code!(general_protection_handler, "GENERAL PROTECTION FAULT");
exception_handler_with_error_code!(alignment_check_handler, "ALIGNMENT CHECK");
exception_handler_with_error_code!(control_protection_handler, "CONTROL PROTECTION EXCEPTION");

// 特殊处理异常 -------------------------------------------------
/// Double fault exception handler
///
/// A double fault occurs when the CPU fails to handle an exception properly.
/// This is a critical system error that typically indicates a serious problem
/// such as stack overflow or corrupted system state.
///
/// # Arguments
/// * `stack_frame` - The interrupt stack frame at the time of the fault
/// * `error_code` - Error code provided by the CPU (usually 0 for double faults)
///
/// # Returns
/// This function never returns (`-> !`). It panics to halt the system.
///
/// # Safety
/// This handler runs in a special interrupt context and must not return.
/// It logs the error and halts the system to prevent further damage.
///
/// # Examples
/// This handler is called automatically by the CPU when a double fault occurs.
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

/// Page fault exception handler
///
/// A page fault occurs when the CPU attempts to access memory that is not
/// currently mapped or when access violates page protection rules.
///
/// # Arguments
/// * `stack_frame` - The interrupt stack frame at the time of the fault
/// * `error_code` - Page fault error code indicating the cause of the fault
///
/// # Behavior
/// 1. Reads the faulting address from the CR2 register
/// 2. Logs the fault address, error code, and stack frame
/// 3. Enters an infinite halt loop (should be replaced with proper page handling)
///
/// # Future Improvements
/// This handler should be extended to:
/// - Allocate new pages for demand paging
/// - Handle copy-on-write pages
/// - Implement page swapping
/// - Handle protection violations
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

/// Breakpoint exception handler
///
/// A breakpoint exception is triggered by the `int3` instruction.
/// This is commonly used for debugging purposes.
///
/// # Arguments
/// * `stack_frame` - The interrupt stack frame at the time of the breakpoint
///
/// # Behavior
/// Logs the breakpoint event and continues execution.
/// In a debugger, this would typically pause execution for inspection.
pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

/// Machine check exception handler
///
/// A machine check exception indicates a hardware error detected by the CPU.
/// This is a critical error that often indicates failing hardware.
///
/// # Arguments
/// * `stack_frame` - The interrupt stack frame at the time of the error
///
/// # Returns
/// This function never returns (`-> !`). It panics to halt the system.
///
/// # Safety
/// Machine check errors are typically unrecoverable and indicate serious
/// hardware problems. The system should be halted to prevent data corruption.
pub extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    serial_println!("CRITICAL: MACHINE CHECK\n{:#?}", stack_frame);
    panic!("SYSTEM HALT: MACHINE CHECK");
}

/// Infinite halt loop
///
/// This function enters an infinite loop that repeatedly executes the `hlt`
/// (halt) instruction. The `hlt` instruction stops the CPU until the next
/// interrupt occurs, reducing power consumption.
///
/// # Returns
/// This function never returns (`-> !`).
///
/// # Use Cases
/// - Placeholder for exception handlers that need to stop execution
/// - Idle loop when no tasks are available
/// - Emergency stop when unrecoverable errors occur
///
/// # Safety
/// This function is safe but will never return, effectively halting the CPU.
#[inline(always)]
fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

macro_rules! pic_interrupt_handler {
    ($name:ident, $irq_number:expr) => {
        #[allow(unused_variables)]
        pub extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame) {
            if $irq_number == 1 {
                let mut port = x86_64::instructions::port::Port::<u8>::new(0x60);
                let scancode = unsafe { port.read() };
                crate::drivers::input::keyboard::KEYBOARD.handle_scancode(scancode);
            } else {
                serial_println!("IRQ {} received!", $irq_number);
            }

            unsafe {
                PICS.lock()
                    .notify_end_of_interrupt(PIC_1_OFFSET + $irq_number);
            }
        }
    };
}
// 为所有 16 个 IRQ 定义处理函数
pic_interrupt_handler!(pic_interrupt_handler_0, 0); // 时钟中断 Timer
pic_interrupt_handler!(pic_interrupt_handler_1, 1); // 键盘中断 Keyboard
pic_interrupt_handler!(pic_interrupt_handler_2, 2); // 级联到 PIC2
pic_interrupt_handler!(pic_interrupt_handler_3, 3); // 串口 COM2
pic_interrupt_handler!(pic_interrupt_handler_4, 4); // 串口 COM1
pic_interrupt_handler!(pic_interrupt_handler_5, 5); // 并口 LPT2 / 声卡
pic_interrupt_handler!(pic_interrupt_handler_6, 6); // 软盘控制器 Floppy Disk
pic_interrupt_handler!(pic_interrupt_handler_7, 7); // 并口 LPT1 / 伪中断
pic_interrupt_handler!(pic_interrupt_handler_8, 8); // RTC Real Time Clock
pic_interrupt_handler!(pic_interrupt_handler_9, 9); // 重定向 IRQ2
pic_interrupt_handler!(pic_interrupt_handler_10, 10); // 空闲 / SCSI / 网卡
pic_interrupt_handler!(pic_interrupt_handler_11, 11); // 空闲 / SCSI / 网卡
pic_interrupt_handler!(pic_interrupt_handler_12, 12); // PS/2 鼠标
pic_interrupt_handler!(pic_interrupt_handler_13, 13); // FPU / 协处理器
pic_interrupt_handler!(pic_interrupt_handler_14, 14); // 主 IDE Primary IDE
pic_interrupt_handler!(pic_interrupt_handler_15, 15); // 次 IDE Secondary IDE
