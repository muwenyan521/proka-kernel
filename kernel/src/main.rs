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
use ab_glyph::{Font, FontRef, Glyph, point};
use limine::{BaseRevision, request::FramebufferRequest};
use proka_kernel::{
    graphics::{Pixel, Renderer, color},
    memory::talcalloc,
    output::console::Console,
};

/* The section data define area */
#[unsafe(link_section = ".requests")]
#[used]
/// The base revision of the kernel.
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[unsafe(link_section = ".requests")]
#[used]
/// The framebuffer request of the kernel.
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

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

    //allocator::init_heap();

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            let mut render = Renderer::new(framebuffer);
            render.set_clear_color(color::BLACK);
            render.clear();
            render.draw_line(
                Pixel::new(0, 0),
                Pixel::new(800, 600),
                color::Color::new(128, 128, 128),
            );

            render.fill_triangle(
                Pixel::new(456, 12),
                Pixel::new(356, 122),
                Pixel::new(221, 86),
                color::YELLOW,
            );

            let font = FontRef::try_from_slice(include_bytes!("../fonts/maple-mono.ttf")).unwrap();
            let mut c = Console::new(render, font);
            c.draw_string("12121_-+*^%&/()=?'!@#");
        }
    }

    loop {}
}
