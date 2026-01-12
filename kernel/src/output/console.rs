extern crate alloc;
use crate::output::font8x16::FONT8X16;
use crate::FRAMEBUFFER_REQUEST;
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

/// The object of console.
pub struct Console {
    address: *mut u8,
    width: u64,
    height: u64,
    pitch: u64,
    position: (u64, u64), // (x, y)
}

// We have to do it, so that it can be contained by Mutex.
unsafe impl Send for Console {}
unsafe impl Sync for Console {}

impl Console {
    pub fn init() -> Self {
        let framebuffer_response = FRAMEBUFFER_REQUEST.get_response().unwrap();
        let framebuffer = framebuffer_response.framebuffers().next().unwrap();
        Self {
            address: framebuffer.addr(),
            width: framebuffer.width(),
            height: framebuffer.height(),
            pitch: framebuffer.pitch(),
            position: (0, 0),
        }
    }

    fn scroll_up(&mut self) {
        let st = crate::libs::time::time_since_boot();
        // First, clear first line
        for y in 0..FONT_H {
            for x in 0..self.width {
                let offset = y * self.pitch + x * 4;
                unsafe {
                    self.address
                        .add(offset as usize)
                        .cast::<u32>()
                        .write(0x00000000);
                }
            }
        }

        // Second, move each line up.
        for y in FONT_H..self.height {
            for x in 0..self.width {
                let src_offset = y * self.pitch + x * 4;
                let dst_offset = (y - FONT_H) * self.pitch + x * 4;
                unsafe {
                    let pixel = self.address.add(src_offset as usize).cast::<u32>().read();
                    self.address
                        .add(dst_offset as usize)
                        .cast::<u32>()
                        .write(pixel);
                }
            }
        }

        // Third, clear last line
        for y in (self.height - FONT_H)..self.height {
            for x in 0..self.width {
                let offset = y * self.pitch + x * 4;
                unsafe {
                    self.address
                        .add(offset as usize)
                        .cast::<u32>()
                        .write(0x00000000);
                }
            }
        }
        let et = crate::libs::time::time_since_boot();
        use crate::serial_println;
        serial_println!("{}", et - st);
    }

    fn print_char(&mut self, c: usize) {
        // If character is "\n", just switch to next line.
        if c == ('\n' as usize) {
            self.position.0 = 0;
            if (self.position.1 + FONT_H) >= self.height {
                self.scroll_up();
                self.position.1 -= FONT_H;
            } else {
                self.position.1 += FONT_H;
            }
            return;
        }

        // If over than self.width, switch to next line.
        if self.position.0 + FONT_W > self.width {
            self.position.0 = 0;
            if (self.position.1 + FONT_H) >= self.height {
                self.scroll_up();
                self.position.1 -= FONT_H;
            } else {
                self.position.1 += FONT_H;
            }
        }

        // Compute the current position
        let start_x = self.position.0;
        let start_y = self.position.1;

        // Write pixel
        for line in 0..FONT_H {
            for i in 0..FONT_W {
                let mask = 0x80 >> i;

                if FONT8X16[c][line as usize] & mask != 0 {
                    // Write white pixel
                    // Compute current pixel offset
                    let x = start_x + i;
                    let y = start_y + line;
                    if x < self.width && y < self.height {
                        let pixel_offset = y * self.pitch + x * 4;

                        // Write
                        unsafe {
                            self.address
                                .add(pixel_offset as usize)
                                .cast::<u32>()
                                .write(0xFFFFFFFF);
                        }
                    }
                }
            }
        }

        self.position.0 += FONT_W;
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
