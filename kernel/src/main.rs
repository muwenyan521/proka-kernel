#![no_std]
#![no_main]

use core::panic::PanicInfo;
use multiboot2::{BootInformation, BootInformationHeader};
#[macro_use]
extern crate proka_kernel;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    // x86_64::instructions::interrupts::int3();
    loop {}
}

/* C functions extern area */
extern_safe! {
    fn add(a: i32, b: i32) -> i32;
    fn sub(a: i32, b: i32) -> i32;
}

/* The Kernel main code */
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(mbi_ptr: *const BootInformationHeader) -> ! {
    // The magic number has checked in assmebly, so pass it.

    /* Get the multiboot2 information */
    let boot_info = unsafe { BootInformation::load(mbi_ptr).expect("Failed to load BootInformation") };

    /* Get the framebuffer tag */
    // In multiboot2 crate, the "info.framebuffer_tag()" will
    // return a Some(Ok(framebuffer)), so use "match" to handle. 
    let framebuffer = match boot_info.framebuffer_tag() {
        Some(Ok(tag)) => tag,
        Some(Err(_)) => panic!("Unknown framebuffer type"),
        None => panic!("No framebuffer tag"),
    };

    // let test = safe_add(3, 5);

    loop {}
}
