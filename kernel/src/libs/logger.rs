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

    fn flush(&self) {}
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {
         dual_println!("\x1b[32m[SUCCESS] {}\x1b[0m", format_args!($($arg)*))
    };
}

/// 初始化日志系统
pub fn init_logger() {
    static LOGGER: KernelLogger = KernelLogger;
    log::set_logger(&LOGGER).expect("Failed to set logger");
    #[cfg(debug_assertions)]
    log::set_max_level(log::LevelFilter::Trace);
    #[cfg(not(debug_assertions))]
    log::set_max_level(log::LevelFilter::Info);
}
