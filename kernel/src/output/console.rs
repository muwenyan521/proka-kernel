extern crate alloc;
use crate::graphics::{
    Pixel, Renderer,
    color::{self, Color},
};
use ab_glyph::{Font, FontRef, PxScale};
use alloc::{vec, vec::Vec};

pub const DEFAULT_FONT_SIZE: f32 = 12.0;
pub const TAB_SPACES: usize = 4;

#[derive(Clone)]
struct ConsoleChar {
    ch: char,
    fg: Color,
    bg: Color,
}

pub struct Console<'a> {
    renderer: Renderer<'a>,
    font: FontRef<'static>,
    scale: PxScale,

    buffer: Vec<Vec<Option<ConsoleChar>>>,
    width_chars: u32,
    height_chars: u32,

    cursor_x: u32,
    cursor_y: u32,
    current_color: Color,
    current_bg_color: Color,

    font_width: u32,
    font_height: u32,
}

impl<'a> Console<'a> {
    pub fn new(renderer: Renderer<'a>, font: FontRef<'static>) -> Self {
        let scale = font.pt_to_px_scale(DEFAULT_FONT_SIZE).unwrap();
        let g = font.glyph_id('M').with_scale(scale);
        let bound = font.glyph_bounds(&g);
        let font_width = bound.width();
        let font_height = bound.height();
        let width_chars = (renderer.width() as f32 / font_width) as u32;
        let height_chars = (renderer.height() as f32 / font_height) as u32;
        let initial_buffer = vec![vec![None; width_chars as usize]; height_chars as usize];

        Self {
            renderer,
            font,
            scale,
            cursor_x: 0,
            cursor_y: 0,
            buffer: initial_buffer,
            width_chars: width_chars,
            height_chars: height_chars,
            current_color: color::WHITE,
            current_bg_color: color::BLACK,
            font_width: font_width as u32,
            font_height: font_height as u32,
        }
    }

    pub fn draw_char(&mut self, ch: char) {
        let glyph = self.font.glyph_id(ch).with_scale(self.scale);

        if let Some(g) = self.font.outline_glyph(glyph) {
            g.draw(|x, y, c| {
                let x = x + (self.cursor_x * self.font_width);
                let y = y + (self.cursor_y * self.font_height);

                if c == 0.0 {
                    return;
                }
                let alpha = (255.0 * c) as u8;
                // 设置混合后的像素颜色
                self.renderer.set_pixel(
                    Pixel::new(x as u64, y as u64),
                    &self.current_color.mix_alpha(alpha),
                );
            });
        }
    }

    pub fn draw_string(&mut self, string: &str) {
        for c in string.chars() {
            self.draw_char(c);
            self.cursor_x += 1;
            if self.cursor_x >= self.width_chars {
                self.cursor_x = 0;
                self.cursor_y += 1;
            }
        }
        self.draw_cursor();
    }

    pub fn draw_cursor(&mut self) {
        let x = self.cursor_x * self.font_width;
        let y = self.cursor_y * self.font_height;
        self.renderer.fill_rect(
            Pixel::new(x as u64, y as u64),
            self.font_width as u64,
            self.font_height as u64,
            color::WHITE,
        );
    }
}
