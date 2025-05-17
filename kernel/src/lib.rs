//! # Proka Kernel - A kernel for ProkaOS
//! Copyright (C) RainSTR Studio 2025, All rights reserved.
//!
//! This provides the public functions, and they will help you
//! to use the kernel functions easily.

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
use core::panic::PanicInfo;
pub mod output;

/// This will extern the C function and make it to safe.
///
/// # Example
/// ```rust
/// use proka_kernel::extern_safe;
///
/// // Make sure that the C function was defined and linked currectly.
/// extern_safe! {
///     fn add(a: i32, b: i32) -> i32;
/// }
///
/// // Then use it, with the header "safe_".
/// let result = safe_add(1, 2);
/// assert_eq!(result, 3);
/// ```
#[macro_export]
macro_rules! extern_safe {
    (
        $(
            $(#[$meta:meta])*
            fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty;
        )*
    ) => {
        unsafe extern "C" {
            $(
                $(#[$meta])*
                fn $name($($arg: $ty),*) -> $ret;
            )*
        }

       $(
           paste::paste! {
                pub fn [<safe_ $name>]($($arg: $ty),*) -> $ret {
                    unsafe { $name($($arg),*) }
                }
            }
        )*
    };
}

/* The tests define part */
/// The trait that assign the function is testable.
pub trait Testable {
    /// The things will run
    fn run(&self) -> ();
}

// This is the default implementation of this trait
impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("Testing {}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// This is the test runner, which will run if the test begins.
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

/// This is the QEMU exit code
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// The function to quit the QEMU
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

// The kernel entry, which will start uo the test
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    test_main();
    loop {}
}

/* The test functions */

/* Panics */
// This is the default panic handler
#[cfg(not(test))]
#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    //x86_64::instructions::interrupts::int3();
    loop {}
}

// And this is for test
#[cfg(test)]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    // Call the default test panic handler
    panic_for_test(info)
}

// This is the panic handler for all testing function
#[cfg(test)]
pub fn panic_for_test(info: &PanicInfo) -> ! {
    serial_println!("failed");
    serial_println!("Caused by:\n\t{}", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}     // Unreachable, but must write this
}
