use core::{fmt::Write, ptr};
use multiboot2::FramebufferTag;
use crate::output::bmf::{DEFAULT_FONT, BMFParser};

// Define Framebuffer info struct
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    addr: *mut u32,
    width: u32,
    height: u32,
    pitch: u32,
    bpp: u8,
    red_shift: u8,
    green_shift: u8,
    blue_shift: u8,
}

impl FramebufferInfo {
    /// Parse multiboot2 info
    pub unsafe fn from_multiboot(fb_tag: &FramebufferTag) -> Option<Self> {
        // Mkae sure the format is RGB 32 bit
        let (red_shift, green_shift, blue_shift) = match fb_tag.buffer_type().unwrap() {
            multiboot2::FramebufferType::RGB { red, green, blue } => (
                red.position.trailing_zeros() as u8,
                green.position.trailing_zeros() as u8,
                blue.position.trailing_zeros() as u8,
            ),
            _ => panic!("Unsupported color format"),
        };

        Some(Self {
            addr: fb_tag.address() as *mut u32,
            width: fb_tag.width(),
            height: fb_tag.height(),
            pitch: fb_tag.pitch(),
            bpp: fb_tag.bpp(),
            red_shift,
            green_shift,
            blue_shift,
        })
    }

    /// Safe put pixel method
    #[inline]
    pub fn put_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = (y * (self.pitch / 4) + x) as isize;
        unsafe {
            ptr::write_volatile(self.addr.offset(offset), color);
        }
    }

    /// Generate RGB color
    pub fn rgb(&self, r: u8, g: u8, b: u8) -> u32 {
        ((r as u32) << self.red_shift) |
        ((g as u32) << self.green_shift) |
        ((b as u32) << self.blue_shift)
    }
}

/// 位图字体渲染器
pub struct BitmapFontRenderer {
    fb: FramebufferInfo,
    font: BMFParser,
    fg_color: u32,
    bg_color: u32,
    cursor_x: u32,
    cursor_y: u32,
}

impl BitmapFontRenderer {
    pub fn new(fb: FramebufferInfo, font: BMFParser, fg: u32, bg: u32) -> Self {
        Self {
            fb,
            font,
            fg_color: fg,
            bg_color: bg,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    /// Draw single char
    pub fn draw_char(&mut self, c: char) {
        let ascii = c as usize;

        let bitmap = match self.font.get_grayscale_image(ascii.try_into().unwrap()) {
            Some(b) => b,
            None => return,
        };

        let start_x = self.cursor_x;
        let start_y = self.cursor_y;

        for (y, row) in bitmap.iter().enumerate() {
            for (x, &pixel) in row.iter().enumerate() {
                let color = if pixel > 0 { // Check grayscale value
                    self.fg_color
                } else {
                    self.bg_color
                };
                self.fb.put_pixel(
                    start_x + x as u32,
                    start_y + y as u32,
                    color
                );
            }
        }

        self.cursor_x += self.font.font_size as u32;
        if self.cursor_x >= self.fb.width - self.font.font_size as u32 {
            self.cursor_x = 0;
            self.cursor_y += self.font.font_size as u32;
        }
    }

    /// Handler text rendering and next line
    pub fn write_string(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        for c in s.chars() {
            match c {
                '\n' => {
                    self.cursor_x = 0;
                    self.cursor_y += self.font.font_size as u32;
                }
                _ => self.draw_char(c),
            }
        }
        Ok(())
    }
}

// Implement the Write struct
impl Write for BitmapFontRenderer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s)
    }
}
