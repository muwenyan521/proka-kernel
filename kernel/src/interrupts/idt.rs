//! 中断描述符表（IDT）模块
//!
//! 此模块提供中断描述符表（Interrupt Descriptor Table, IDT）的初始化和配置功能。
//! IDT是x86_64架构中用于处理中断和异常的关键数据结构，它定义了中断向量号到相应处理程序的映射。
//!
//! ## 功能
//!
//! - 定义全局中断描述符表（IDT）
//! - 配置所有CPU异常的处理程序
//! - 配置所有硬件中断（IRQ）的处理程序
//! - 提供IDT加载和初始化功能
//!
//! ## 异常类型
//!
//! 此模块处理以下类型的异常：
//!
//! 1. **无错误码异常**：除零错误、调试异常、不可屏蔽中断、断点异常等
//! 2. **有错误码异常**：无效TSS、段不存在、栈段错误、一般保护错误等
//! 3. **特殊异常**：页面故障、机器检查、双重故障
//!
//! ## 硬件中断
//!
//! 配置了16个PIC（可编程中断控制器）中断（IRQ0-IRQ15），包括：
//! - IRQ0：时钟中断
//! - IRQ1：键盘中断
//! - IRQ2-15：其他硬件设备中断
//!
//! ## 安全考虑
//!
//! - 双重故障处理程序使用独立的栈（IST）以防止栈溢出
//! - 所有中断处理程序都定义在`handler.rs`模块中
//! - 使用`lazy_static`确保IDT的线程安全初始化
//!
//! ## 示例
//!
//! ```no_run
//! use kernel::interrupts::idt;
//!
//! // 初始化并加载IDT
//! idt::init_idt();
//! ```

use crate::interrupts::gdt;
use crate::interrupts::handler;
use crate::interrupts::pic::{PIC_1_OFFSET, PIC_2_OFFSET};
use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

/// PIC相关的中断向量数量
///
/// 表示可编程中断控制器（PIC）支持的中断数量，即IRQ0到IRQ15，共16个中断。
#[allow(dead_code)]
pub const PICS_EVT_COUNT: u8 = 16;
lazy_static! {
    /// 全局中断描述符表（IDT）
    ///
    /// 此静态变量包含系统所有中断和异常的处理程序配置。
    /// 使用`lazy_static`宏确保线程安全的惰性初始化。
    ///
    /// ## 配置内容
    ///
    /// 1. **CPU异常处理程序**：所有x86_64架构定义的异常
    /// 2. **硬件中断处理程序**：16个PIC中断（IRQ0-IRQ15）
    /// 3. **特殊配置**：双重故障使用独立栈（IST）
    ///
    /// ## 异常分类
    ///
    /// - **无错误码异常**：发生时CPU不推送错误码到栈上
    /// - **有错误码异常**：发生时CPU推送错误码到栈上
    /// - **特殊异常**：需要特殊处理的异常（如页面故障、双重故障）
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        
        // 无错误码异常设置
        idt.divide_error.set_handler_fn(handler::divide_error_handler);
        idt.debug.set_handler_fn(handler::debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(handler::nmi_handler);
        idt.breakpoint.set_handler_fn(handler::breakpoint_handler);
        idt.overflow.set_handler_fn(handler::overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(handler::bound_range_handler);
        idt.invalid_opcode.set_handler_fn(handler::invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(handler::device_not_available_handler);
        idt.x87_floating_point.set_handler_fn(handler::x87_floating_point_handler);
        
        // 有错误码异常设置
        idt.invalid_tss.set_handler_fn(handler::invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(handler::segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(handler::stack_segment_handler);
        idt.general_protection_fault.set_handler_fn(handler::general_protection_handler);
        idt.alignment_check.set_handler_fn(handler::alignment_check_handler);
        idt.cp_protection_exception.set_handler_fn(handler::control_protection_handler);
        
        // 特殊异常设置
        idt.page_fault.set_handler_fn(handler::pagefault_handler);
        idt.machine_check.set_handler_fn(handler::machine_check_handler);
        
        // 双重故障需要特殊处理：使用独立栈（IST）
        unsafe {
            idt.double_fault
                .set_handler_fn(handler::double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        
        // PIC中断处理程序设置
        // IRQ0 - 时钟中断, IRQ1 - 键盘中断
        idt[PIC_1_OFFSET as u8].set_handler_fn(handler::pic_interrupt_handler_0); // IRQ0
        idt[PIC_1_OFFSET as u8 + 1].set_handler_fn(handler::pic_interrupt_handler_1); // IRQ1
        idt[PIC_1_OFFSET as u8 + 2].set_handler_fn(handler::pic_interrupt_handler_2); // IRQ2
        idt[PIC_1_OFFSET as u8 + 3].set_handler_fn(handler::pic_interrupt_handler_3); // IRQ3
        idt[PIC_1_OFFSET as u8 + 4].set_handler_fn(handler::pic_interrupt_handler_4); // IRQ4
        idt[PIC_1_OFFSET as u8 + 5].set_handler_fn(handler::pic_interrupt_handler_5); // IRQ5
        idt[PIC_1_OFFSET as u8 + 6].set_handler_fn(handler::pic_interrupt_handler_6); // IRQ6
        idt[PIC_1_OFFSET as u8 + 7].set_handler_fn(handler::pic_interrupt_handler_7); // IRQ7
        idt[PIC_2_OFFSET as u8].set_handler_fn(handler::pic_interrupt_handler_8);   // IRQ8
        idt[PIC_2_OFFSET as u8 + 1].set_handler_fn(handler::pic_interrupt_handler_9);   // IRQ9
        idt[PIC_2_OFFSET as u8 + 2].set_handler_fn(handler::pic_interrupt_handler_10);  // IRQ10
        idt[PIC_2_OFFSET as u8 + 3].set_handler_fn(handler::pic_interrupt_handler_11);  // IRQ11
        idt[PIC_2_OFFSET as u8 + 4].set_handler_fn(handler::pic_interrupt_handler_12);  // IRQ12
        idt[PIC_2_OFFSET as u8 + 5].set_handler_fn(handler::pic_interrupt_handler_13);  // IRQ13
        idt[PIC_2_OFFSET as u8 + 6].set_handler_fn(handler::pic_interrupt_handler_14);  // IRQ14
        idt[PIC_2_OFFSET as u8 + 7].set_handler_fn(handler::pic_interrupt_handler_15);  // IRQ15
        
        idt
    };
}
/// 初始化并加载中断描述符表（IDT）
///
/// 此函数加载全局IDT到CPU，使所有配置的中断和异常处理程序生效。
/// 在调用此函数后，系统将能够处理CPU异常和硬件中断。
///
/// ## 注意事项
///
/// - 此函数应在GDT和PIC初始化之后调用
/// - 调用此函数后，中断将被启用（如果之前被禁用）
/// - 此函数只应调用一次，重复调用没有额外效果但也没有危害
///
/// ## 示例
///
/// ```no_run
/// use kernel::interrupts::idt;
///
/// // 先初始化其他组件...
/// // 然后初始化IDT
/// idt::init_idt();
/// ```
pub fn init_idt() {
    IDT.load();
}
