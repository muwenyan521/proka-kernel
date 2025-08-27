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
#![test_runner(proka_kernel::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

/* Module imports */
#[macro_use]
extern crate proka_kernel;
extern crate alloc;

use limine::memory_map::EntryType;
use proka_kernel::BASE_REVISION;

/* C functions extern area */
extern_safe! {
    fn add(a: i32, b: i32) -> i32;
    fn sub(a: i32, b: i32) -> i32;
}

/* The Kernel main code */
// The normal one
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    // Check is limine version supported
    assert!(BASE_REVISION.is_supported(), "Limine version not supported");

    println!("ProkaOS {}", env!("CARGO_PKG_VERSION"));
    println!("• Hello, World!");

    proka_kernel::output::console::CONSOLE
        .lock()
        .cursor_hidden();

    proka_kernel::interrupts::gdt::init();
    println!("• GDT Initialized");

    proka_kernel::interrupts::idt::init_idt();
    println!("• IDT initialized");

    proka_kernel::interrupts::apic::init();

    let memory_map = proka_kernel::MEMORY_MAP_REQUEST.get_response().unwrap();
    serial_println!("All of memory map:");
    for entry in memory_map.entries().iter() {
        serial_println!(
            "base: {}, lenght: {}, USABLE: {}",
            entry.base,
            entry.length,
            entry.entry_type == EntryType::USABLE
        );
    }

    println!("• Kernel ready");

    let mut device_manager = proka_kernel::drivers::DEVICE_MANAGER.lock();

    device_manager.register_device(proka_kernel::drivers::block::RamFSDevice::create_device(
        0, 10240,
    ));

    let mem_device = device_manager.get_device("ramfs-0").unwrap();

    mem_device.ops.write(1, &[0x42]).unwrap();

    let mut buf = [0x41];
    mem_device.ops.read(1, &mut buf).unwrap();

    dual_print!("{}", buf[0] as char);

    loop {
        x86_64::instructions::hlt();
    }
}
