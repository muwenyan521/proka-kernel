//! Proka Kernel - A kernel for ProkaOS
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! Well, welcome to the main entry of Proka Kernel!!
//!
//! If you have jumped here successfully, that means your CPU
//! can satisfy our kernel's requirements.
//!
//! Now, let's enjoy the kernel written in Rust!!!!
//!
//! For more information, see https://github.com/RainSTR-Studio/proka-kernel

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(proka_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

/* Module imports */
#[cfg(not(test))]
use multiboot2::{BootInformation, BootInformationHeader};

#[macro_use]
extern crate proka_kernel;

/* C functions extern area */
extern_safe! {
    fn add(a: i32, b: i32) -> i32;
    fn sub(a: i32, b: i32) -> i32;
}

/* The Kernel main code */
// The normal one
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(mbi_ptr: *const BootInformationHeader) -> ! {
    // The magic number has checked in assmebly, so pass it.

    serial_println!("Hello, ProkaOS!");

    /* Get the multiboot2 information */
    let boot_info =
        unsafe { BootInformation::load(mbi_ptr).expect("Failed to load BootInformation") };

    /* Get the framebuffer tag */
    // In multiboot2 crate, the "info.framebuffer_tag()" will
    // return a Some(Ok(framebuffer)), so use "match" to handle.
    let framebuffer = match boot_info.framebuffer_tag() {
        Some(Ok(tag)) => tag,
        Some(Err(_)) => panic!("Unknown framebuffer type"),
        None => panic!("No framebuffer tag"),
    };
    serial_println!("Framebuffer initialized");

    /* Initialize the heap */
    proka_kernel::allocator::init_heap();
    serial_println!("Heap initialized");

    /* Initialize the mapper */
    proka_kernel::mapper::init_memory_mapper();
    serial_println!("Memory mapper initialized");

    /* Initialize the global renderer */
    crate::proka_kernel::output::framebuffer::init_global_render(&framebuffer);
    serial_println!("Framebuffer renderer initialized");

    // 新增屏幕输出代码
    if let Some(render) = proka_kernel::output::framebuffer::get_render()
        .lock()
        .as_mut()
    {
        render.draw_char('H');
    }

    loop {}
}

// The test kernel entry
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    test_main();
    loop {}
}
