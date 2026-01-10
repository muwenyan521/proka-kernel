//! 内核日志记录器模块
//!
//! 此模块提供内核的日志记录功能，包括：
//! - 自定义日志记录器实现（`KernelLogger`）
//! - 彩色日志输出支持
//! - 成功消息宏（`success!`）
//! - 日志系统初始化函数
//!
//! ## 功能特性
//!
//! - **彩色输出**: 不同日志级别使用不同颜色
//!   - 错误（Error）: 红色
//!   - 警告（Warn）: 黄色
//!   - 信息（Info）: 白色
//!   - 调试（Debug）: 蓝色
//!   - 跟踪（Trace）: 紫色
//!   - 成功（Success）: 绿色
//! - **双重输出**: 同时输出到控制台和串口（通过`dual_println!`宏）
//! - **条件编译**: 调试版本启用Trace级别，发布版本启用Info级别
//!
//! ## 使用示例
//!
//! ```rust
//! use log::{error, warn, info, debug, trace};
//! use kernel::libs::logger::{init_logger, success};
//!
//! // 初始化日志系统
//! init_logger();
//!
//! // 记录不同级别的日志
//! error!("这是一个错误消息");
//! warn!("这是一个警告消息");
//! info!("这是一个信息消息");
//! debug!("这是一个调试消息");
//! trace!("这是一个跟踪消息");
//!
//! // 使用成功宏
//! success!("操作成功完成");
//! ```
//!
//! ## 依赖
//!
//! 此模块依赖于`log` crate和内核的`dual_println`宏。

use crate::dual_println;
use log::{Log, Metadata, Record};

/// 自定义内核日志记录器
///
/// 实现`log::Log` trait，提供彩色日志输出功能。
/// 所有日志消息都会通过`dual_println!`宏输出，同时显示在控制台和串口。
///
/// ## 日志级别颜色映射
///
/// | 级别 | 颜色 | ANSI转义码 |
/// |------|------|------------|
/// | Error | 红色 | `\x1b[31m` |
/// | Warn  | 黄色 | `\x1b[33m` |
/// | Info  | 白色 | `\x1b[37m` |
/// | Debug | 蓝色 | `\x1b[34m` |
/// | Trace | 紫色 | `\x1b[35m` |
/// | Success | 绿色 | `\x1b[32m` |
///
/// ## 注意
///
/// 此记录器总是启用所有日志级别，实际过滤由`log` crate的全局级别控制。
pub struct KernelLogger;

impl Log for KernelLogger {
    /// 检查日志记录是否启用
    ///
    /// 此实现总是返回`true`，因为实际过滤由全局日志级别控制。
    ///
    /// # 参数
    ///
    /// * `_metadata` - 日志元数据（未使用）
    ///
    /// # 返回值
    ///
    /// 总是返回`true`
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    /// 记录日志消息
    ///
    /// 根据日志级别选择颜色，并通过`dual_println!`宏输出格式化消息。
    ///
    /// # 参数
    ///
    /// * `record` - 日志记录，包含级别、消息和元数据
    ///
    /// # 格式
    ///
    /// 输出格式为：`{颜色}[{级别}] {消息}\x1b[0m`
    /// 其中`\x1b[0m`重置颜色，防止影响后续输出。
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level();

            let color;
            match record.level() {
                log::Level::Error => {
                    color = "\x1b[31m";
                }
                log::Level::Warn => {
                    color = "\x1b[33m";
                }
                log::Level::Info => {
                    color = "\x1b[37m";
                }
                log::Level::Debug => {
                    color = "\x1b[34m";
                }
                log::Level::Trace => {
                    color = "\x1b[35m";
                }
            }

            let _ = dual_println!("{}[{}] {}\x1b[0m", color, level, record.args());
        }
    }

    /// 刷新日志输出
    ///
    /// 此实现为空操作，因为`dual_println!`宏立即输出。
    fn flush(&self) {}
}

/// 成功消息宏
///
/// 输出绿色格式化的成功消息，格式为：`\x1b[32m[SUCCESS] {消息}\x1b[0m`
///
/// # 参数
///
/// 与标准`format!`宏相同的参数。
///
/// # 示例
///
/// ```rust
/// use kernel::success;
///
/// success!("文件已成功加载");
/// success!("用户 {} 已登录", "alice");
/// success!("操作完成，耗时 {} 毫秒", 150);
/// ```
///
/// # 注意
///
/// 此宏通过`#[macro_export]`导出，可在整个内核中使用。
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {
         dual_println!("\x1b[32m[SUCCESS] {}\x1b[0m", format_args!($($arg)*))
    };
}

/// 初始化内核日志系统
///
/// 设置全局日志记录器为`KernelLogger`，并根据构建配置设置日志级别。
///
/// ## 日志级别配置
///
/// - **调试构建**（`debug_assertions`启用）: 启用所有级别（`LevelFilter::Trace`）
/// - **发布构建**（`debug_assertions`禁用）: 启用Info及以上级别（`LevelFilter::Info`）
///
/// # 错误处理
///
/// 如果设置日志记录器失败（例如已设置过），此函数会panic。
///
/// # 示例
///
/// ```rust
/// use kernel::libs::logger::init_logger;
///
/// // 在main函数开始时调用
/// init_logger();
/// ```
///
/// # 注意
///
/// 此函数应该在内核启动过程中尽早调用，通常是在`main`函数的开始处。
/// 多次调用会导致panic，因为`log::set_logger`只能调用一次。
pub fn init_logger() {
    static LOGGER: KernelLogger = KernelLogger;
    log::set_logger(&LOGGER).expect("Failed to set logger");
    #[cfg(debug_assertions)]
    log::set_max_level(log::LevelFilter::Trace);
    #[cfg(not(debug_assertions))]
    log::set_max_level(log::LevelFilter::Info);
}
