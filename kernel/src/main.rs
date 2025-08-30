// src/main.rs
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
use log::{error, info, warn};
use proka_kernel::BASE_REVISION;
use proka_kernel::drivers::init_devices;
use proka_kernel::fs::vfs::VFS; // Added VFS import to check content
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

    // Load initrd
    if let Some(initrd_response) = proka_kernel::MODULE_REQUEST.get_response() {
        if let Some(inir) = initrd_response.modules().first() {
            unsafe {
                let slice: &[u8] = core::slice::from_raw_parts(inir.addr(), inir.size() as usize);
                match proka_kernel::libs::initrd::load_initrd(slice) {
                    Ok(_) => info!("Initrd loaded successfully."),
                    Err(e) => error!("Failed to load initrd: {:?}", e),
                }
            }
        } else {
            warn!("No initrd module found.");
        }
    } else {
        warn!("Initrd module request failed.");
    }
    let mut buf = [0; 1024];
    let content = VFS
        .lock()
        .open("/test.txt")
        .unwrap()
        .read(&mut buf)
        .unwrap();
    println!("{}", core::str::from_utf8(&buf[..content]).unwrap());

    success!("Kernel ready!");

    loop {
        x86_64::instructions::hlt();
    }
}
