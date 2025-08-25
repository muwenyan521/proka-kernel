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

    buffer: Vec<Vec<Option<ConsoleChar>>>, // 存储所有字符的缓冲区
    scroll_offset_y: usize,                // 垂直滚动偏移量，表示缓冲区顶部有多少行是不可见的

    width_chars: u32,
    height_chars: u32,

    cursor_x: u32,
    cursor_y: u32,
    current_color: Color,
    current_bg_color: Color,

    font_width: u32,
    font_height: u32,
    font_line_height: f32,
    font_baseline: f32,
}

impl<'a> Console<'a> {
    pub fn new(renderer: Renderer<'a>, font: FontRef<'static>) -> Self {
        let scale = font.pt_to_px_scale(DEFAULT_FONT_SIZE).unwrap();
        let scaled_font = font.as_scaled(scale);

        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        let line_gap = scaled_font.line_gap();

        let font_line_height = ascent - descent + line_gap;
        let font_baseline = ascent;

        // 获取'M'的字形边界来计算字符宽度，这通常是一个比较好的近似值
        let g = font.glyph_id('M').with_scale(scale);
        let bound = font.glyph_bounds(&g);
        let font_width = ceilf(bound.width()); // 向上取整以确保宽度足够

        let font_height = ceilf(font_line_height); // 向上取整以确保高度足够

        let width_chars = (renderer.width() as f32 / font_width) as u32;
        let height_chars = (renderer.height() as f32 / font_height) as u32;

        // 初始缓冲区可以只包含屏幕可见的行数
        let initial_buffer = vec![vec![None; width_chars as usize]; height_chars as usize];

        Self {
            renderer,
            font,
            scale,
            cursor_x: 0,
            cursor_y: 0,
            buffer: initial_buffer,
            scroll_offset_y: 0, // 初始时没有滚动
            width_chars,
            height_chars,
            current_color: color::WHITE,
            current_bg_color: color::BLACK,
            font_width: font_width as u32,
            font_height: font_height as u32,

            font_line_height,
            font_baseline,
        }
    }

    /// 清空整个缓冲区
    pub fn clear(&mut self) {
        for row in self.buffer.iter_mut() {
            for cell in row.iter_mut() {
                *cell = None;
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.scroll_offset_y = 0;
        self.clear_screen_pixels(); // 清空屏幕上的像素
    }

    /// 清空渲染器上的所有像素，以背景色填充
    fn clear_screen_pixels(&mut self) {
        let raw_clear_color = self.renderer.get_clear_color();
        self.renderer.set_clear_color(self.current_bg_color);
        self.renderer.clear();
        self.renderer.set_clear_color(raw_clear_color);
    }

    /// 滚动缓冲区
    pub fn scroll(&mut self, lines: i32) {
        let new_offset = self.scroll_offset_y as i32 + lines;
        self.scroll_offset_y = new_offset
            .max(0) // 确保不向上滚动超过缓冲区顶部
            .min(self.buffer.len() as i32 - self.height_chars as i32) // 确保不向下滚动超过缓冲区底部
            as usize;
        self.draw_buffer_to_screen(); // 滚动后重新绘制屏幕
    }

    /// 确保缓冲区有足够的行来容纳新的内容，并在需要时滚动
    fn ensure_buffer_capacity(&mut self) {
        // 如果当前光标Y位置加上滚动偏移量已经超出了缓冲区的当前长度
        while (self.cursor_y + self.scroll_offset_y as u32) >= self.buffer.len() as u32 {
            // 添加新行
            self.buffer.push(vec![None; self.width_chars as usize]);
        }

        // 如果光标在屏幕上超出了可见高度，则进行滚动
        if self.cursor_y >= self.height_chars {
            let lines_to_scroll = self.cursor_y - self.height_chars + 1;
            self.scroll_offset_y += lines_to_scroll as usize;
            self.cursor_y = self.height_chars - 1; // 将光标设置到屏幕的最后一行
        }
    }

    /// 将单个字符写入缓冲区并在屏幕上渲染
    pub fn put_char(&mut self, ch: char) {
        self.ensure_buffer_capacity();

        let buf_y = (self.cursor_y + self.scroll_offset_y as u32) as usize;
        let buf_x = self.cursor_x as usize;

        // 将字符写入缓冲区
        if let Some(row) = self.buffer.get_mut(buf_y) {
            if let Some(cell) = row.get_mut(buf_x) {
                *cell = Some(ConsoleChar {
                    ch,
                    fg: self.current_color,
                    bg: self.current_bg_color,
                });
            }
        }

        // 立即渲染到屏幕
        self.draw_char_to_screen(
            ch,
            self.cursor_x,
            self.cursor_y,
            self.current_color,
            self.current_bg_color,
        );
    }

    /// 渲染一个字符到屏幕的指定位置
    fn draw_char_to_screen(
        &mut self,
        ch: char,
        x_char: u32,
        y_char: u32,
        fg_color: Color,
        bg_color: Color,
    ) {
        let cursor_x_px = x_char * self.font_width;
        let cursor_y_px = y_char * self.font_height;

        // 绘制背景
        self.renderer.fill_rect(
            Pixel::new(cursor_x_px as u64, cursor_y_px as u64),
            self.font_width as u64,
            self.font_height as u64,
            bg_color,
        );

        if let Some(glyph) = self
            .font
            .outline_glyph(self.font.glyph_id(ch).with_scale(self.scale))
        {
            let pixel_bounds = glyph.px_bounds();

            let x_offset = cursor_x_px as f32 + pixel_bounds.min.x;
            let y_offset = cursor_y_px as f32 + self.font_baseline + pixel_bounds.min.y;

            glyph.draw(|x, y, c| {
                let screen_x = x as f32 + x_offset;
                let screen_y = y as f32 + y_offset;

                if c == 0.0 {
                    return;
                }
                let alpha = (255.0 * c) as u8;

                self.renderer.set_pixel(
                    Pixel::new(round(screen_x as f64) as u64, round(screen_y as f64) as u64),
                    &fg_color.mix_alpha(alpha),
                );
            });
        }
    }

    /// 绘制当前光标到屏幕
    pub fn draw_cursor(&mut self) {
        // 先清除旧光标位置的像素，确保不会留下残影
        let old_cursor_x = self.cursor_x;
        let old_cursor_y = self.cursor_y;
        let old_buf_y = (old_cursor_y + self.scroll_offset_y as u32) as usize;

        if let Some(row) = self.buffer.get(old_buf_y) {
            if let Some(Some(cchar @ ConsoleChar { ch, fg, bg })) = row.get(old_cursor_x as usize) {
                self.draw_char_to_screen(*ch, old_cursor_x, old_cursor_y, *fg, *bg);
            } else {
                // 如果旧光标位置没有字符，就用背景色填充
                self.renderer.fill_rect(
                    Pixel::new(
                        old_cursor_x as u64 * self.font_width as u64,
                        old_cursor_y as u64 * self.font_height as u64,
                    ),
                    self.font_width as u64,
                    self.font_height as u64,
                    self.current_bg_color,
                );
            }
        }

        // 绘制新光标
        let x = self.cursor_x * self.font_width;
        let y = self.cursor_y * self.font_height; // 注意这里修正为使用实际的字符高度
        self.renderer.fill_rect(
            Pixel::new(x as u64, y as u64),
            self.font_width as u64,
            self.font_height as u64,
            color::WHITE, // 光标颜色
        );
    }

    pub fn draw_buffer_to_screen(&mut self) {
        self.clear_screen_pixels();
        let start_display_row = self.scroll_offset_y as usize;
        let end_display_row =
            (self.scroll_offset_y + self.height_chars as usize).min(self.buffer.len());
        struct CharRenderInfo {
            ch: char,
            screen_x: u32,
            screen_y: u32,
            fg: Color,
            bg: Color,
        }
        let mut chars_to_draw: Vec<CharRenderInfo> = Vec::new();
        for (screen_y, buf_row_idx) in (start_display_row..end_display_row).enumerate() {
            if let Some(row) = self.buffer.get(buf_row_idx) {
                // `self.buffer` 在此循环中被不可变借用
                for (screen_x, cell) in row.iter().enumerate() {
                    if let Some(cchar) = cell {
                        // 收集字符信息，稍后在不可变借用结束后再绘制
                        chars_to_draw.push(CharRenderInfo {
                            ch: cchar.ch,
                            screen_x: screen_x as u32,
                            screen_y: screen_y as u32,
                            fg: cchar.fg,
                            bg: cchar.bg,
                        });
                    }
                    // else 分支不再需要，因为 clear_screen_pixels 已经设置了背景色
                }
            }
        }
        for char_info in chars_to_draw {
            self.draw_char_to_screen(
                char_info.ch,
                char_info.screen_x,
                char_info.screen_y,
                char_info.fg,
                char_info.bg,
            );
        }
        self.draw_cursor(); // 绘制完所有字符后，绘制光标
        self.renderer.present();
    }

    /// 写入一个字符串到控制台
    pub fn write_str(&mut self, string: &str) {
        for c in string.chars() {
            match c {
                '\n' => {
                    // 换行符：光标移到下一行开头
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                    self.ensure_buffer_capacity(); // 确保新行在缓冲区中
                }
                '\r' => {
                    // 回车符：光标移到当前行开头
                    self.cursor_x = 0;
                }
                '\t' => {
                    // 制表符：向前移动 TAB_SPACES 个位置
                    let mut spaces_to_add = TAB_SPACES as u32 - (self.cursor_x % TAB_SPACES as u32);
                    if spaces_to_add == 0 {
                        spaces_to_add = TAB_SPACES as u32;
                    }
                    for _ in 0..spaces_to_add {
                        self.put_char(' ');
                        self.cursor_x += 1;
                        if self.cursor_x >= self.width_chars {
                            self.cursor_x = 0;
                            self.cursor_y += 1;
                            self.ensure_buffer_capacity();
                        }
                    }
                }
                _ => {
                    // 其他字符：写入缓冲区并前进光标
                    self.put_char(c);
                    self.cursor_x += 1;
                    if self.cursor_x >= self.width_chars {
                        self.cursor_x = 0;
                        self.cursor_y += 1;
                        self.ensure_buffer_capacity();
                    }
                }
            }
        }
        self.draw_buffer_to_screen(); // 写入字符串后刷新屏幕
    }

    // 设置前景颜色
    pub fn set_fg_color(&mut self, color: Color) {
        self.current_color = color;
    }

    // 设置背景颜色
    pub fn set_bg_color(&mut self, color: Color) {
        self.current_bg_color = color;
        // 当背景色改变时，需要重新绘制整个屏幕以应用新的背景色
        self.draw_buffer_to_screen();
    }
}
