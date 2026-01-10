//! 可编程中断控制器（PIC）模块
//!
//! 此模块提供对8259A可编程中断控制器（Programmable Interrupt Controller, PIC）的封装。
//! PIC是x86架构中用于管理硬件中断的传统设备，它将多个硬件中断源映射到CPU的单个中断引脚。
//!
//! ## 功能
//!
//! - 定义PIC中断向量偏移量
//! - 提供全局PIC实例的线程安全访问
//! - 初始化PIC并配置中断掩码
//! - 禁用PIC（用于切换到APIC模式）
//!
//! ## PIC架构
//!
//! 系统使用两个级联的8259A PIC芯片：
//! - **主PIC**（PIC1）：处理IRQ0-IRQ7，偏移量为32
//! - **从PIC**（PIC2）：处理IRQ8-IRQ15，偏移量为40
//!
//! 级联配置允许系统支持最多15个硬件中断（IRQ2用于级联）。
//!
//! ## 中断向量映射
//!
//! PIC中断被重新映射到CPU中断向量32-47：
//! - IRQ0 (时钟) → 向量32
//! - IRQ1 (键盘) → 向量33
//! - IRQ2 (级联) → 向量34
//! - IRQ3-7 → 向量35-39
//! - IRQ8-15 → 向量40-47
//!
//! ## 安全考虑
//!
//! - 使用`spin::Mutex`确保对PIC的线程安全访问
//! - 初始化函数包含不安全块，因为直接操作硬件
//! - 禁用函数用于安全切换到APIC模式
//!
//! ## 示例
//!
//! ```no_run
//! use kernel::interrupts::pic;
//!
//! // 初始化PIC
//! pic::init();
//!
//! // 禁用PIC（切换到APIC模式）
//! pic::disable();
//! ```

use pic8259::ChainedPics;
use spin;

/// 主PIC（PIC1）的中断向量偏移量
///
/// 这是IRQ0映射到的CPU中断向量号。PIC中断被重新映射到向量32-47，
/// 以避免与CPU异常（向量0-31）冲突。
pub const PIC_1_OFFSET: u8 = 32;

/// 从PIC（PIC2）的中断向量偏移量
///
/// 这是IRQ8映射到的CPU中断向量号。由于PIC2级联到PIC1的IRQ2，
/// 其偏移量比PIC1偏移量大8。
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// 全局可编程中断控制器实例
///
/// 此静态变量提供对系统PIC的线程安全访问。使用`spin::Mutex`确保
/// 在多核环境中的安全并发访问。
///
/// ## 注意
///
/// 初始化时使用不安全块，因为直接创建`ChainedPics`实例涉及硬件操作。
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// 初始化可编程中断控制器（PIC）
///
/// 此函数执行以下操作：
/// 1. 初始化主PIC和从PIC
/// 2. 配置中断掩码，默认启用时钟中断（IRQ0）和键盘中断（IRQ1）
/// 3. 使PIC准备好接收硬件中断
///
/// ## 中断掩码配置
///
/// 初始化后，中断掩码设置为：
/// - 主PIC掩码：0xFD（二进制11111101），启用IRQ0和IRQ1
/// - 从PIC掩码：0xFF（二进制11111111），禁用所有IRQ8-IRQ15
///
/// ## 安全
///
/// 此函数包含不安全块，因为它直接操作硬件寄存器。
/// 调用者应确保：
/// - 此函数只调用一次
/// - 在启用CPU中断之前调用
/// - 系统处于适当的特权级别（ring 0）
///
/// ## 示例
///
/// ```no_run
/// use kernel::interrupts::pic;
///
/// // 初始化PIC
/// pic::init();
/// ```
pub fn init() {
    unsafe {
        let mut pics = PICS.lock();
        pics.initialize();
        pics.write_masks(0xFD, 0xFF);
    }
}

/// 禁用可编程中断控制器（PIC）
///
/// 此函数禁用PIC的所有中断。通常在切换到APIC（高级可编程中断控制器）
/// 模式时调用，因为APIC是现代x86_64系统的首选中断控制器。
///
/// ## 使用场景
///
/// 1. 当系统检测到APIC可用并希望切换到APIC模式时
/// 2. 在系统关闭或重启过程中
/// 3. 在虚拟化环境中模拟中断控制器时
///
/// ## 安全
///
/// 此函数包含不安全块，因为它直接操作硬件寄存器。
/// 调用者应确保：
/// - 在禁用PIC之前已设置好替代的中断处理机制（如APIC）
/// - 没有正在进行的关键中断处理
///
/// ## 示例
///
/// ```no_run
/// use kernel::interrupts::pic;
///
/// // 禁用PIC以切换到APIC模式
/// pic::disable();
/// ```
pub fn disable() {
    unsafe { PICS.lock().disable() };
}
