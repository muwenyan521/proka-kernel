use crate::FRAMEBUFFER_REQUEST;
use crate::output::font8x16::FONT8X16;
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use spin::Mutex;

// Constants
const FONT_W: u64 = 8;
const FONT_H: u64 = 16;

// Some statics which is global
lazy_static! {
    pub static ref CONSOLE: Mutex<Console> = Mutex::new(Console::init());
}

pub struct Console {
    address: *mut u8,
    width: u64,  // For charactor
    height: u64, // For charactor
    pitch: u64,
    position: (u64, u64)    // (x, y)
}

unsafe impl Send for Console {}
unsafe impl Sync for Console {}

impl Console {
    pub fn init() -> Self {
        let framebuffer_response = FRAMEBUFFER_REQUEST.get_response().unwrap();
        let framebuffer = framebuffer_response.framebuffers().next().unwrap();
        Self {
            address: framebuffer.addr(),
            width: framebuffer.width() / FONT_W,
            height: framebuffer.height() / FONT_H,
            pitch: framebuffer.pitch(),
            position: (0, 0),
        }
    }

    fn print_char(&mut self, c: usize) {
        // Compute the current position
        // If over than self.width, switch to next line.
        if (self.position.0 + FONT_W) >= self.width {
            self.position.1 += FONT_H;  // todo handle stroll
            self.position.0 = 0;
        } else {
            self.position.0 += FONT_W;
        }

        // Write pixel
        let mut pixel_position = self.position;
        for line in 0..FONT_H {
            let line = FONT8X16[c][line as usize];
            for i in 0..FONT_W {
                let mask = 0x80 >> i;
                if line & mask != 0 {
                    // Write white pixel
                    // Compute current pixel offset
                    let x = pixel_position.0;
                    let y = pixel_position.1;
                    let pixel_offset = y * self.pitch + x * 4;

                    // Write
                    unsafe {
                    self.address
                        .add(pixel_offset as usize)
                        .cast::<u32>()
                        .write(0xFFFFFFFF);
                    }

                    // To next pixel
                    pixel_position.0 += 1;
                } else {
                    // No writing
                    pixel_position.0 += 1;
                }
            }
            pixel_position.1 += 1;
        }
    }

    pub fn print_string(&mut self, s: &str) {
        for c in s.bytes() {
            self.print_char(c as usize);
        }
    }
}

// Implement the [`Write`] trait to support formatting
impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::output::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    CONSOLE
        .lock()
        .write_fmt(args)
        .expect("Failed to write to console");
}
