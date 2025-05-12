#![no_std]
#![no_main]

use core::panic::PanicInfo;
use multiboot2::{BootInformation, BootInformationHeader};
use proka_kernel::extern_safe;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("int 3");
    }
    loop {}
}

/* The Kernel main code */
extern_safe! {
    fn add(a: i32, b: i32) -> i32;
    fn sub(a: i32, b: i32) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(mbi_ptr: u32) -> ! {
    // Get the multiboot2 information
    let boot_info =
        unsafe { BootInformation::load(mbi_ptr as *const BootInformationHeader).unwrap() };

    // Get the framebuffer tag.
    // In multiboot2 crate, the "info.framebuffer_tag()" will
    // return a Some(Ok(framebuffer)), so use "match" to handle.
    let framebuffer = match boot_info.framebuffer_tag() {
        Some(Ok(tag)) => tag,
        Some(Err(_)) => panic!("Unknown framebuffer type"),
        None => panic!("No framebuffer tag"),
    };

    loop {}
}
