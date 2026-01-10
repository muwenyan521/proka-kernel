//! Serial output module for debugging and logging
//!
//! This module provides serial port output functionality for debugging purposes.
//! It includes both a device-managed serial output system (when devices are available)
//! and a fallback direct serial port access (when device management fails).
//!
//! # Features
//! - **Debug Mode**: Full serial output functionality when compiled with debug assertions
//! - **Release Mode**: Empty stubs when compiled without debug assertions (no overhead)
//! - **Device Integration**: Integrates with the kernel's device manager for proper serial device access
//! - **Fallback Support**: Direct serial port access as fallback when device manager fails
//!
//! # Macros
//! - [`serial_print!`](macro.serial_print.html): Outputs formatted text to serial port (debug mode only)
//! - [`serial_println!`](macro.serial_println.html): Outputs formatted text with newline to serial port (debug mode only)
//!
//! # Functions
//! - [`_print`]: Internal printing function (debug mode: actual output, release mode: empty)
//! - [`serial_fallback`]: Fallback function for direct serial port access
//!
//! # Examples
//! ```rust
//! use crate::serial_print;
//! use crate::serial_println;
//!
//! // Only works in debug mode
//! #[cfg(debug_assertions)]
//! {
//!     serial_print!("Debug: ");
//!     serial_println!("x = {}", 42);
//! }
//! ```
//!
//! # Configuration
//! The module's behavior depends on the compilation mode:
//! - **Debug mode** (`#[cfg(debug_assertions)]`): Full serial output functionality
//! - **Release mode** (`#[cfg(not(debug_assertions))]`): Empty stubs (no output, no overhead)
//!
//! # Safety
//! The fallback function uses unsafe direct port I/O (0x3F8) when device management fails.
//! This is safe in the kernel context but should only be used as a last resort.

extern crate alloc;
use crate::drivers::DEVICE_MANAGER;
use uart_16550::SerialPort;

/// Fallback function for direct serial port access when device management fails
///
/// This function provides a direct serial port output mechanism that bypasses
/// the device manager. It's used as a fallback when the proper serial device
/// cannot be accessed through the device manager.
///
/// # Arguments
/// * `args` - Formatted arguments to output to the serial port
///
/// # Behavior
/// 1. Initializes a direct serial port connection to COM1 (port 0x3F8)
/// 2. Outputs a warning message indicating device manager failure
/// 3. Outputs the provided formatted arguments
///
/// # Safety
/// This function uses `unsafe` to create a `SerialPort` instance with direct
/// hardware access. This is safe in the kernel context but should only be
/// used when the proper device-managed approach fails.
///
/// # Examples
/// ```rust
/// use crate::output::serial::serial_fallback;
/// use core::fmt::Arguments;
///
/// // Create formatted arguments
/// let args = format_args!("Error code: {}", 42);
/// serial_fallback(args);
/// ```
pub fn serial_fallback(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    let mut serial_port = unsafe { SerialPort::new(0x3F8) };
    serial_port.init();
    // 输出错误信息
    serial_port
        .write_str("WARNING: Could not initialize serial port device\n")
        .expect("Printing to serial failed");
    serial_port
        .write_fmt(args)
        .expect("Printing to serial failed");
}

/* The functions and macros in debug mode */
/// Internal printing function for serial output (debug mode only)
///
/// This function is marked as `#[doc(hidden)]` because it's an implementation
/// detail used by the [`serial_print!`] and [`serial_println!`] macros. It
/// provides the actual serial output functionality in debug mode.
///
/// # Arguments
/// * `args` - Formatted arguments to output to the serial port
///
/// # Behavior
/// 1. Acquires a read lock on the device manager
/// 2. Attempts to get the serial device with major/minor numbers (1, 0)
/// 3. If found and it's a character device, writes the formatted output to it
/// 4. If not found or not a character device, falls back to [`serial_fallback`]
///
/// # Safety
/// This function is safe to call as it properly handles device acquisition
/// and provides a fallback mechanism when devices are unavailable.
///
/// # Note
/// This function is only compiled in debug mode (`#[cfg(debug_assertions)]`).
/// In release mode, an empty stub is provided instead.
#[doc(hidden)]
#[cfg(debug_assertions)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;

    // 获取设备管理器锁
    let device_manager = DEVICE_MANAGER.read();

    // 尝试获取设备号为 (1, 0) 的字符设备
    match device_manager.get_device_by_major_minor(1, 0) {
        Some(device) => {
            // 尝试将设备转换为字符设备
            if let Some(char_device_arc) = device.as_char_device() {
                let mut buffer = alloc::string::String::new();
                buffer.write_fmt(args).expect("Failed to format string");

                char_device_arc
                    .write(buffer.as_bytes())
                    .expect("Printing to serial failed");
            } else {
                serial_fallback(args);
            }
        }
        None => {
            // 设备 (1,0) 未找到
            serial_fallback(args);
        }
    }
}

/// Prints to the host through the serial interface (debug mode only)
///
/// This macro formats its arguments using the standard Rust formatting syntax
/// and sends the output to the serial port. It's only available in debug mode
/// (`#[cfg(debug_assertions)]`).
///
/// # Arguments
/// * `$($arg:tt)*` - Format string and arguments using Rust's formatting syntax
///
/// # Behavior
/// Forwards the formatted arguments to the internal [`_print`] function which
/// handles device-managed serial output with fallback support.
///
/// # Examples
/// ```rust
/// use crate::serial_print;
///
/// // Only works in debug mode
/// #[cfg(debug_assertions)]
/// {
///     serial_print!("Debug message: ");
///     serial_print!("x = {}, ", 10);
///     serial_print!("y = {}", 20);
/// }
/// ```
///
/// # Note
/// This macro is a no-op in release mode (compiled out completely).
/// Use it for debugging output that shouldn't appear in production builds.
#[macro_export]
#[cfg(debug_assertions)]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::output::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface with newline (debug mode only)
///
/// This macro works like [`serial_print!`](macro.serial_print.html) but automatically appends a newline
/// character to the output. It's only available in debug mode.
///
/// # Arguments
/// * `$($arg:tt)*` - Format string and arguments using Rust's formatting syntax
///
/// # Behavior
/// - When called without arguments: outputs just a newline character
/// - When called with arguments: formats the arguments and appends a newline
///
/// # Examples
/// ```rust
/// use crate::serial_println;
///
/// // Only works in debug mode
/// #[cfg(debug_assertions)]
/// {
///     // Empty line
///     serial_println!();
///
///     // With message
///     serial_println!("Debug message");
///
///     // With formatting
///     serial_println!("x = {}, y = {}", 10, 20);
/// }
/// ```
///
/// # Note
/// This is the preferred macro for most serial debugging output as it ensures
/// each message is on its own line, making serial logs easier to read.
#[macro_export]
#[cfg(debug_assertions)]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

/* The macros and function not in debug mode (empty) */
/// Internal printing function stub (release mode only)
///
/// This function is marked as `#[doc(hidden)]` because it's an implementation
/// detail. In release mode (`#[cfg(not(debug_assertions))]`), this is an empty
/// stub that does nothing, eliminating serial output overhead in production.
///
/// # Arguments
/// * `args` - Formatted arguments (ignored in release mode)
///
/// # Note
/// This empty implementation ensures that serial output calls are completely
/// eliminated in release builds, providing zero runtime overhead.
#[doc(hidden)]
#[cfg(not(debug_assertions))]
pub fn _print(args: ::core::fmt::Arguments) {}

/// Serial print macro stub (release mode only)
///
/// This macro is an empty stub in release mode (`#[cfg(not(debug_assertions))]`).
/// It expands to nothing, completely eliminating serial output overhead
/// in production builds.
///
/// # Arguments
/// * `$($arg:tt)*` - Format string and arguments (ignored in release mode)
///
/// # Note
/// Using this macro in release mode has zero runtime cost - it's completely
/// compiled out.
#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! serial_print {
    ($($arg:tt)*) => {};
}

/// Serial println macro stub (release mode only)
///
/// This macro is an empty stub in release mode (`#[cfg(not(debug_assertions))]`).
/// It expands to nothing, completely eliminating serial output overhead
/// in production builds.
///
/// # Arguments
/// * `$($arg:tt)*` - Format string and arguments (ignored in release mode)
///
/// # Note
/// Using this macro in release mode has zero runtime cost - it's completely
/// compiled out. All variants (empty, single argument, formatted arguments)
/// expand to empty blocks.
#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! serial_println {
    () => {};
    ($fmt:expr) => {};
    ($fmt:expr, $($arg:tt)*) => {};
}
