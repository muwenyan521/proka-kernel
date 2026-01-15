extern crate alloc;
use crate::graphics::{color, Color};
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
    fg_color: Color,
    bg_color: Color,
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
            fg_color: color::WHITE,
            bg_color: color::BLACK,
        }
    }

    #[inline(always)]
    fn scroll_up(&mut self) {
        let st = crate::libs::time::time_since_boot();

        unsafe {
            let base_ptr = self.address as *mut u8;
            let scroll_bytes = (FONT_H as usize) * self.pitch as usize;
            let total_bytes = (self.height as usize) * self.pitch as usize;

            // Use memmove
            // Move all buffer upper scroll_bytes
            core::ptr::copy(
                base_ptr.add(scroll_bytes),
                base_ptr,
                total_bytes - scroll_bytes,
            );

            // Clear last scroll_bytes area
            core::ptr::write_bytes(base_ptr.add(total_bytes - scroll_bytes), 0, scroll_bytes);
        }

        let et = crate::libs::time::time_since_boot();
        use crate::serial_println;
        serial_println!("Scroll up used time: {} ms", (et - st) * 1000.0);
    }

    /// Print a char to framebuffer console.
    /// 
    /// Note that the character must discovorable in ASCII, otherwise we don't know what
    /// unexpected thing is being happened.
    pub fn print_char(&mut self, c: usize) {
        if (self.height - self.position.1) < FONT_H {
            self.scroll_up();
            self.position.1 = self.height - FONT_H;
        }

        // If character is "\n", just switch to next line.
        if c == ('\n' as usize) {
            self.position.0 = 0;
            self.position.1 += FONT_H;
            return;
        }

        // If over than self.width, switch to next line.
        if self.position.0 + FONT_W > self.width {
            self.position.0 = 0;
            self.position.1 += FONT_H;
        }

        // Compute the current position
        let start_x = self.position.0;
        let start_y = self.position.1;

        // Write pixel
        for line in 0..FONT_H {
            for i in 0..FONT_W {
                let mask = 0x80 >> i;

                // Write white pixel
                // Compute current pixel offset
                let x = start_x + i;
                let y = start_y + line;
                if x < self.width && y < self.height {
                    let pixel_offset = y * self.pitch + x * 4;
                    if FONT8X16[c][line as usize] & mask != 0 {
                        // Write
                        unsafe {
                            self.address
                                .add(pixel_offset as usize)
                                .cast::<u32>()
                                .write(self.fg_color.to_u32(true));
                        }
                    } else {
                        unsafe {
                            self.address
                                .add(pixel_offset as usize)
                                .cast::<u32>()
                                .write(self.bg_color.to_u32(true));
                        }
                    }
                }
            }
        }

        self.position.0 += FONT_W;
    }

    /// Print a string to console.
    pub fn print_string(&mut self, s: &str) {
        for c in s.bytes() {
            self.print_char(c as usize);
        }
    }

    /* Settings methods */
    /// Set up the color of foreground character.
    pub fn set_fg_color(&mut self, color: Color) {
        self.fg_color = color
    }

    /// Set up the color of background character.
    pub fn set_bg_color(&mut self, color: Color) {
        self.bg_color = color
    }

    /// Get the color of foreground character.
    pub fn get_fg_color(&self) -> Color {
        self.fg_color
    }

    /// Get the color of background character.
    pub fn get_bg_color(&self) -> Color {
        self.bg_color
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
