// src/libs/logger.rs
use crate::dual_println;
use log::{Log, Metadata, Record};

/// 自定义日志记录器
pub struct KernelLogger;

impl Log for KernelLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level();
            let target = if !record.target().is_empty() {
                record.target()
            } else {
                ""
            };

            let _ = dual_println!("[{}] {}: {}\n", level, target, record.args());
        }
    }

    fn flush(&self) {}
}

/// 初始化日志系统
pub fn init_logger() {
    static LOGGER: KernelLogger = KernelLogger;
    log::set_logger(&LOGGER).expect("Failed to set logger");
    log::set_max_level(log::LevelFilter::Trace);
}
