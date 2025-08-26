use crate::output::console::_print as console_print;
use crate::output::serial::_print as serial_print;

/// 双重打印宏：同时输出到控制台和串口
#[macro_export]
macro_rules! dual_print {
    ($($arg:tt)*) => {
        $crate::output::dual::_dual_print_serial(format_args!($($arg)*));
        // 总是输出到控制台
        $crate::output::dual::_dual_print_console(format_args!($($arg)*));
    };
}

/// 双重打印宏（带换行）
#[macro_export]
macro_rules! dual_println {
    () => {
        $crate::dual_print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::dual_print!("{}\n", format_args!($($arg)*))
    };
}

// 内部函数：处理控制台打印
#[doc(hidden)]
pub fn _dual_print_console(args: core::fmt::Arguments) {
    console_print(args);
}

// 内部函数：处理串口打印
#[doc(hidden)]
pub fn _dual_print_serial(args: core::fmt::Arguments) {
    serial_print(args);
}
