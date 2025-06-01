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
#[macro_use]
extern crate proka_kernel;
extern crate alloc;

#[cfg(not(test))]
use multiboot2::{BootInformation, BootInformationHeader};
use x86_64::VirtAddr;

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
    let binding = &boot_info.framebuffer_tag();
    let framebuffer = match binding {
        Some(Ok(tag)) => tag,
        Some(Err(_)) => panic!("Unknown framebuffer type"),
        None => panic!("No framebuffer tag"),
    };

    /* Get the memory map tag */
    let memmap = &boot_info.memory_map_tag().unwrap();

    serial_println!("Boot info initialized");

    /* Enable the configured IDT */
    proka_kernel::interrupts::idt::init_idt();
    serial_println!("IDT initialized");

    /* Initialize the heap */
    proka_kernel::allocator::init_heap();
    serial_println!("Heap initialized");

    /* Initialize the mapper */
    // Initialize frame allocator first
    proka_kernel::mapper::init_frame_allocator(memmap);
    serial_println!("Frame allocator initialized");

    // Then initialize memory mapper with physical offset
    let physical_memory_offset = VirtAddr::new(0xFFFF_8000_0000_0000); // Common higher-half offset
    proka_kernel::mapper::init_memory_mapper(physical_memory_offset);
    serial_println!("Memory mapper initialized");

    /* Initialize the global renderer */
    crate::proka_kernel::output::framebuffer::init_global_render(&framebuffer);
    serial_println!("Framebuffer renderer initialized");

    // The output code for screen
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
