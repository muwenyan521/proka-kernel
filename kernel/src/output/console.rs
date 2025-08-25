extern crate alloc;
use crate::{
    graphics::{
        Pixel, Renderer,
        color::{self, Color},
    },
    serial_println,
};
use ab_glyph::{Font, FontRef, PxScale, ScaleFont}; // 引入 ScaleFont trait
use alloc::{vec, vec::Vec};
use libm::*;

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

    font_line_height: f32, // 新增：字体的行高，用于计算垂直偏移
    font_baseline: f32,    // 新增：字体的基线偏移
}

impl<'a> Console<'a> {
    pub fn new(renderer: Renderer<'a>, font: FontRef<'static>) -> Self {
        let scale = font.pt_to_px_scale(DEFAULT_FONT_SIZE).unwrap();
        let scaled_font = font.as_scaled(scale); // 创建缩放后的字体实例，方便获取度量

        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        let line_gap = scaled_font.line_gap();

        let font_line_height = ascent - descent + line_gap;
        let font_baseline = ascent; // 基线通常是 ascent

        let g = font.glyph_id('M').with_scale(scale);
        let bound = font.glyph_bounds(&g);
        let font_width = bound.width();
        // 使用实际的行高作为字符高度，这样可以保证字符间距一致
        let font_height = font_line_height;

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
            font_height: font_height as u32, // 修正为使用 font_line_height

            font_line_height,
            font_baseline,
        }
    }

    pub fn draw_char(&mut self, ch: char) {
        if let Some(glyph) = self
            .font
            .outline_glyph(self.font.glyph_id(ch).with_scale(self.scale))
        {
            // 计算渲染的起始x和y坐标
            let cursor_x_px = self.cursor_x * self.font_width;
            let cursor_y_px = self.cursor_y * self.font_height;

            // 根据字形边界和基线进行垂直偏移调整
            // glyph.px_bounds() 返回字形在像素网格上的实际渲染区域
            let pixel_bounds = glyph.px_bounds();

            // 调整y的起始位置，使其基于基线而不是仅仅从顶部开始
            // (pixel_bounds.min_y) 是字形相对于其基线（通常是0）的最小y坐标
            // (pixel_bounds.min_y).ceil() 向上取整以确保所有像素都在可见范围内
            let x_offset = cursor_x_px as f32 + pixel_bounds.min.x;
            let y_offset = cursor_y_px as f32 + self.font_baseline + pixel_bounds.min.y;

            glyph.draw(|x, y, c| {
                // 将局部坐标转换为屏幕坐标
                let screen_x = x as f32 + x_offset;
                let screen_y = y as f32 + y_offset;

                if c == 0.0 {
                    return;
                }
                let alpha = (255.0 * c) as u8;

                // 设置混合后的像素颜色
                self.renderer.set_pixel(
                    Pixel::new(round(screen_x as f64) as u64, round(screen_y as f64) as u64),
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
        // 游标的高度现在应该匹配我们计算的行高，而不是字形高度
        let y = self.cursor_y as f32 * self.font_line_height;
        self.renderer.fill_rect(
            Pixel::new(x as u64, y as u64),
            self.font_width as u64,
            self.font_height as u64, // 使用修正后的 font_height
            color::WHITE,
        );
    }
}
