//! 模型特定寄存器（MSR）模块
//!
//! 此模块定义内核使用的模型特定寄存器（Model-Specific Register, MSR）常量。
//! MSR是x86架构中特定于处理器模型的寄存器，用于控制CPU的各种功能。
//!
//! ## 关于MSR
//!
//! 模型特定寄存器是x86架构中用于控制处理器特定功能的寄存器，包括：
//! - 性能监控
//! - 电源管理
//! - 调试功能
//! - 系统配置
//!
//! MSR通过`RDMSR`（读取MSR）和`WRMSR`（写入MSR）指令访问。
//!
//! ## 当前定义的MSR
//!
//! - `IA32_APIC_BASE`: APIC基地址寄存器
//!
//! ## 参考
//!
//! - Intel 64 and IA-32 Architectures Software Developer's Manual
//! - Volume 4: Model-Specific Registers

/// APIC基地址寄存器（IA32_APIC_BASE）
///
/// 此寄存器控制本地APIC（高级可编程中断控制器）的基地址和状态。
///
/// ## 寄存器布局
///
/// | 位范围 | 字段名称 | 描述 |
/// |--------|----------|------|
/// | 0-7    | 保留     | 必须为0 |
/// | 8      | BSP      | 引导处理器标志（1=引导处理器） |
/// | 9      | 保留     | 必须为0 |
/// | 10     | EXTD     | 扩展目的地模式使能 |
/// | 11     | EN       | APIC全局使能（1=启用） |
/// | 12-35  | APIC基地址 | APIC基地址（4KB对齐） |
/// | 36-63  | 保留     | 必须为0 |
///
/// ## 功能
///
/// 1. **APIC基地址**: 指定本地APIC寄存器在物理内存中的基地址
/// 2. **APIC使能**: 控制APIC功能的全局启用/禁用
/// 3. **引导处理器标识**: 标识当前CPU是否为引导处理器
///
/// ## 使用示例
///
/// ```rust
/// use kernel::libs::msr::IA32_APIC_BASE;
///
/// // 读取APIC基地址寄存器
/// let apic_base = unsafe { x86::msr::rdmsr(IA32_APIC_BASE) };
/// println!("APIC基地址: 0x{:x}", apic_base);
/// ```
///
/// ## 参考
///
/// - Intel SDM Volume 3, Section 10.4.4: "Local APIC Status and Location"
/// - Intel SDM Volume 4, Table 35-2: "IA32_APIC_BASE MSR"
pub const IA32_APIC_BASE: u32 = 0x1b;
