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
use alloc::string::String;
use log::{debug, info};
use proka_kernel::BASE_REVISION;
use proka_kernel::drivers::init_devices;
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
    init_devices();
    proka_kernel::libs::logger::init_logger(); // Init log system

    proka_kernel::output::console::CONSOLE
        .lock()
        .cursor_hidden();

    println!("Starting ProkaOS v{}...", env!("CARGO_PKG_VERSION")); // Print welcome message

    // 初始化各个模块
    proka_kernel::interrupts::gdt::init();
    info!("GDT Initialized");
    proka_kernel::interrupts::idt::init_idt();
    info!("IDT initialized");
    proka_kernel::interrupts::apic::init();
    info!("APIC initialized");

    //proka_kernel::memory::paging::table::init_page_table();

    println!("Device list:");
    for device in proka_kernel::drivers::DEVICE_MANAGER
        .lock()
        .list_devices()
        .iter()
    {
        println!("{:?}", device);
    }

    success!("Kernel ready!");
    if let Some(initrd) = proka_kernel::MODULE_REQUEST.get_response() {
        let inir = initrd.modules()[0];
        unsafe {
            let slice: &[u8] = core::slice::from_raw_parts(inir.addr(), inir.size() as usize);
            let raw_reader = proka_kernel::libs::initrd::CpioNewcReader::new(slice);
            for obj_result in raw_reader {
                match obj_result {
                    Ok(obj) => {
                        println!("Found object:");
                        println!("  Name: {}", obj.name);
                        println!(
                            "  Data: {:?}",
                            core::str::from_utf8(obj.data).unwrap_or("<binary data>")
                        );
                        println!("  Metadata: {:?}", obj.metadata);
                    }
                    Err(_) => {
                        println!("Error reading object.");
                    }
                }
            }
            println!("--- Finished RawCpioNewcReader ---");
        }
    }

    loop {
        x86_64::instructions::hlt();
    }
}
