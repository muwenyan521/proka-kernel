extern crate alloc;
use crate::{
    graphics::{
        Pixel, Renderer,
        color::{self, Color},
    },
    FRAMEBUFFER_REQUEST
    // serial_println, // 如果不需要，可以移除
};
use ab_glyph::{Font, FontRef, PxScale, ScaleFont}; // 引入 ScaleFont trait
use alloc::{vec, vec::Vec};
use spin::Mutex;
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use libm::*; // round, ceilf 等函数

pub const DEFAULT_FONT_SIZE: f32 = 12.0;
pub const TAB_SPACES: usize = 4;

// The default font writer
lazy_static! {
    static ref DEFAULT_FONT: FontRef<'static> = {
        let font_data = include_bytes!("../../fonts/maple-mono.ttf");
        FontRef::try_from_slice(font_data).unwrap()
    };

    pub static ref DEFAULT_FONT_WRITER: Mutex<Console<'static>> = Mutex::new({
        let renderer = Renderer::new(
            FRAMEBUFFER_REQUEST.get_response().unwrap().framebuffers().next().unwrap(),
        );
        Console::new(renderer, DEFAULT_FONT.clone())
    });
}

/// Represents a character with its foreground and background colors in the console buffer.
#[derive(Clone, Copy, PartialEq, Eq)] // 添加 PartialEq 和 Eq 便于比较
struct ConsoleChar {
    ch: char,
    fg: Color,
    bg: Color,
}

/// Represents a rectangular region on the screen in character coordinates.
/// Used for tracking dirty regions that need redrawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Creates a new rectangle.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Checks if this rectangle overlaps with another rectangle.
    pub fn overlaps_with(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Merges this rectangle with another, returning a new rectangle that encompasses both.
    pub fn merge(&self, other: &Rect) -> Self {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
}

pub struct Console<'a> {
    pub renderer: Renderer<'a>,
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
    font_line_height: f32, // Line height in font pixels (f32)
    font_baseline: f32,    // Baseline offset in font pixels (f32)

    dirty_regions: Vec<Rect>,  // 存储需要重绘的矩形区域 (字符坐标)
    cursor_needs_redraw: bool, // 标记光标是否需要重绘

    hidden_cursor: bool,
}

impl<'a> Console<'a> {
    pub fn new(renderer: Renderer<'a>, font: FontRef<'static>) -> Self {
        // 使用as_scaled()直接获取ScaledFont，避免重复创建
        let scale = font.pt_to_px_scale(DEFAULT_FONT_SIZE).unwrap();
        let scaled_font = font.as_scaled(scale);

        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        let line_gap = scaled_font.line_gap();

        let font_line_height = ascent - descent + line_gap;
        let font_baseline = ascent;

        // 获取'M'的字形边界来计算字符宽度，这通常是一个比较好的近似值
        // 直接使用bound.width()和bound.height()会更精确
        let g_id = font.glyph_id('M');
        let g = g_id.with_scale(scale);
        let bound = font.glyph_bounds(&g);
        // 使用ceilf确保可以容纳字符，并直接转换为u32
        let font_width = ceilf(bound.width()) as u32;
        let font_height = ceilf(font_line_height) as u32;

        let width_chars = renderer.width().checked_div(font_width as u64).unwrap_or(1) as u32;
        let height_chars = renderer
            .height()
            .checked_div(font_height as u64)
            .unwrap_or(1) as u32;

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
            font_width,  // 已是u32
            font_height, // 已是u32
            font_line_height,
            font_baseline,
            dirty_regions: Vec::new(),
            cursor_needs_redraw: true, // 初始时光标需要绘制
            hidden_cursor: false,
        }
    }

    pub fn get_renderer(&mut self) -> &mut Renderer<'a> {
        &mut self.renderer
    }

    /// 添加一个脏区域，会尝试合并相邻或重叠的区域
    fn add_dirty_region(&mut self, region: Rect) {
        let mut merged = false;
        for i in 0..self.dirty_regions.len() {
            if self.dirty_regions[i].overlaps_with(&region) {
                self.dirty_regions[i] = self.dirty_regions[i].merge(&region);
                merged = true;
                break; // 假设合并可以简化为一次，更复杂的合并可能需要多次遍历
            }
        }
        if !merged {
            self.dirty_regions.push(region);
        }
    }

    pub fn cursor_hidden(&mut self) {
        self.hidden_cursor = true;
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
        // 清空整个屏幕是一个脏区域
        self.dirty_regions.clear(); // 清除之前的脏区域
        self.add_dirty_region(Rect::new(0, 0, self.width_chars, self.height_chars));
        self.cursor_needs_redraw = true;
        self.draw_buffer_to_screen(); // 在`clear`后立即绘制以反映变化
    }

    /// 清空渲染器上的所有像素，以背景色填充
    /// 此函数现在只在完全重绘屏幕时使用，例如滚动或改变背景色
    fn clear_screen_pixels(&mut self) {
        let raw_clear_color = self.renderer.get_clear_color(); // 保存原始清除色
        self.renderer.set_clear_color(self.current_bg_color);
        self.renderer.clear();
        self.renderer.set_clear_color(raw_clear_color); // 恢复原始清除色
    }

    /// 滚动缓冲区 (lines > 0 -> 向下滚动, lines < 0 -> 向上滚动)
    pub fn scroll(&mut self, lines: i32) {
        let old_offset = self.scroll_offset_y;
        let new_offset = (self.scroll_offset_y as i32 + lines)
            .max(0) // 确保不向上滚动超过缓冲区顶部
            .min(self.buffer.len() as i32 - self.height_chars as i32 + 1) // 确保不向下滚动超过超出缓冲区底部 + 1行，以便显示新行
            as usize;

        self.scroll_offset_y = new_offset;

        if old_offset != new_offset {
            // 滚动导致整个屏幕内容需要重新绘制
            self.dirty_regions.clear();
            self.add_dirty_region(Rect::new(0, 0, self.width_chars, self.height_chars));
            self.cursor_needs_redraw = true; // 光标位置可能不变，但是背景变了，所以也要重新绘制
            self.draw_buffer_to_screen(); // 滚动后立即绘制屏幕
        }
    }

    /// 确保缓冲区有足够的行来容纳新的内容，并在需要时滚动
    fn ensure_buffer_capacity(&mut self) {
        // 如果当前光标Y位置加上滚动偏移量已经超出了缓冲区的当前长度
        let target_buf_y = (self.cursor_y + self.scroll_offset_y as u32) as usize;
        while target_buf_y >= self.buffer.len() {
            // 添加新行
            self.buffer.push(vec![None; self.width_chars as usize]);
        }

        // 如果光标在屏幕上超出了可见高度，则进行滚动
        // 注意：这里需要考虑屏幕可见高度和实际的缓冲区高度
        if self.cursor_y >= self.height_chars {
            let lines_to_scroll = self.cursor_y - self.height_chars + 1;
            let old_scroll_offset_y = self.scroll_offset_y;
            self.scroll_offset_y = (self.scroll_offset_y as u32 + lines_to_scroll) as usize;
            self.cursor_y = self.height_chars - 1; // 将光标设置到屏幕的最后一行

            // 标记整个屏幕为脏，因为滚动导致所有可见内容移动
            if old_scroll_offset_y != self.scroll_offset_y {
                self.dirty_regions.clear();
                self.add_dirty_region(Rect::new(0, 0, self.width_chars, self.height_chars));
            }
        }
    }

    /// 将单个字符写入缓冲区并在屏幕上渲染
    pub fn put_char(&mut self, ch: char) {
        self.ensure_buffer_capacity();

        let current_buf_y = (self.cursor_y + self.scroll_offset_y as u32) as usize;
        let buf_x = self.cursor_x as usize;

        let new_char_info = Some(ConsoleChar {
            ch,
            fg: self.current_color,
            bg: self.current_bg_color,
        });

        // 检查缓冲区中是否已有该字符，并且颜色、字符都相同，如果相同则不需要重绘
        let mut needs_redraw = true;
        if let Some(row) = self.buffer.get(current_buf_y) {
            if let Some(current_cell) = row.get(buf_x) {
                if *current_cell == new_char_info {
                    needs_redraw = false; // 字符和颜色都相同，不需要重绘
                }
            }
        }

        // 更新缓冲区
        if let Some(row) = self.buffer.get_mut(current_buf_y) {
            if let Some(cell) = row.get_mut(buf_x) {
                *cell = new_char_info;
            }
        }

        // 如果在屏幕可见范围内且需要重绘，则标记为脏
        if needs_redraw && self.cursor_y < self.height_chars {
            self.add_dirty_region(Rect::new(self.cursor_x, self.cursor_y, 1, 1));
        }

        self.cursor_needs_redraw = true; // 光标位置变动或背景变动都需要重绘光标
    }

    /// 渲染一个字符到屏幕的指定位置
    #[inline(always)] // 尝试内联这个函数以减少函数调用开销
    fn draw_char_to_screen_at_px(
        &mut self,
        ch: char,
        x_px: u32,
        y_px: u32,
        fg_color: Color,
        bg_color: Color,
    ) {
        // 绘制背景
        self.renderer.fill_rect(
            Pixel::new(x_px as u64, y_px as u64),
            self.font_width as u64,
            self.font_height as u64,
            bg_color,
        );

        if let Some(glyph) = self
            .font
            .outline_glyph(self.font.glyph_id(ch).with_scale(self.scale))
        {
            let pixel_bounds = glyph.px_bounds();

            let x_offset = x_px as f32 + pixel_bounds.min.x;
            let y_offset = y_px as f32 + self.font_baseline + pixel_bounds.min.y;

            // 优化浮点数到整数转换，避免在循环内部重复计算round
            // 对于每个像素，c是alpha值，我们只在c > 0.0 时进行绘制
            glyph.draw(|x, y, c| {
                if c == 0.0 {
                    return;
                }

                // 结合偏移量计算屏幕像素坐标
                // roundf 足够，或者直接转u32会截断，需要根据需求选择
                // 使用 (val + 0.5) as u32 进行四舍五入到最近整数可以替代 round()
                let screen_x = ((x as f32 + x_offset) + 0.5) as u32;
                let screen_y = ((y as f32 + y_offset) + 0.5) as u32;

                // 边界检查，防止画到屏幕外
                if screen_x >= self.renderer.width() as u32
                    || screen_y >= self.renderer.height() as u32
                {
                    return;
                }

                let alpha = (255.0 * c) as u8;
                self.renderer.set_pixel(
                    Pixel::new(screen_x as u64, screen_y as u64),
                    &fg_color.mix_alpha(alpha),
                );
            });
        }
    }

    /// 绘制当前光标到屏幕
    pub fn draw_cursor(&mut self) {
        // 仅在光标需要重绘时才执行
        if !self.cursor_needs_redraw {
            return;
        }
        if self.hidden_cursor {
            return;
        }

        // 重新绘制光标位置的字符，以清除旧光标
        let old_cursor_x = self.cursor_x;
        let old_cursor_y = self.cursor_y;
        let old_buf_y = (old_cursor_y + self.scroll_offset_y as u32) as usize;

        // 获取旧光标位置的字符信息
        let char_at_cursor = if let Some(row) = self.buffer.get(old_buf_y) {
            row.get(old_cursor_x as usize).and_then(|c| *c)
        } else {
            None
        };

        let screen_x_px = old_cursor_x * self.font_width;
        let screen_y_px = old_cursor_y * self.font_height;

        match char_at_cursor {
            Some(cchar) => {
                // 如果有字符，重绘该字符
                self.draw_char_to_screen_at_px(
                    cchar.ch,
                    screen_x_px,
                    screen_y_px,
                    cchar.fg,
                    cchar.bg,
                );
            }
            None => {
                // 如果没有字符，用背景色填充
                self.renderer.fill_rect(
                    Pixel::new(screen_x_px as u64, screen_y_px as u64),
                    self.font_width as u64,
                    self.font_height as u64,
                    self.current_bg_color,
                );
            }
        }

        // 绘制新光标 (反色矩形或其他表示)
        // 计算光标的像素坐标
        let cursor_screen_x_px = self.cursor_x * self.font_width;
        let cursor_screen_y_px = self.cursor_y * self.font_height;

        // 这里可以使用反色或一个闪烁的方块来表示光标
        self.renderer.fill_rect(
            Pixel::new(cursor_screen_x_px as u64, cursor_screen_y_px as u64),
            self.font_width as u64,
            self.font_height as u64,
            color::WHITE, // 光标颜色
        );
        self.cursor_needs_redraw = false; // 光标已绘制，清除标记
    }

    /// 绘制缓冲区中变脏的部分到屏幕
    pub fn draw_buffer_to_screen(&mut self) {
        // 如果没有脏区域且光标不需要重绘，则无需进行任何渲染操作
        if self.dirty_regions.is_empty() && !self.cursor_needs_redraw {
            self.renderer.present(); // 即使没有脏区，也可能需要刷新屏幕，取决于Renderer实现
            return;
        }

        let start_display_row = self.scroll_offset_y;
        let end_display_row =
            (self.scroll_offset_y + self.height_chars as usize).min(self.buffer.len());

        // 收集所有需要绘制的字符信息，避免在借用self时调用self的方法
        let mut chars_to_draw: Vec<(char, u32, u32, Color, Color)> = Vec::new();

        // 遍历脏区域，只重绘这些区域内的字符
        for dirty_region in self.dirty_regions.drain(..) {
            let start_char_x = dirty_region.x;
            let end_char_x = dirty_region.x + dirty_region.width;
            let start_char_y = dirty_region.y;
            let end_char_y = dirty_region.y + dirty_region.height;

            let actual_start_char_x = start_char_x.min(self.width_chars);
            let actual_end_char_x = end_char_x.min(self.width_chars);
            let actual_start_char_y = start_char_y.min(self.height_chars);
            let actual_end_char_y = end_char_y.min(self.height_chars);

            let px_x = actual_start_char_x * self.font_width;
            let px_y = actual_start_char_y * self.font_height;
            let px_width = (actual_end_char_x - actual_start_char_x) * self.font_width;
            let px_height = (actual_end_char_y - actual_start_char_y) * self.font_height;
            if px_width > 0 && px_height > 0 {
                self.renderer.fill_rect(
                    Pixel::new(px_x as u64, px_y as u64),
                    px_width as u64,
                    px_height as u64,
                    self.current_bg_color,
                );
            }

            for screen_y_offset in actual_start_char_y..actual_end_char_y {
                let current_buf_row_idx = (screen_y_offset + start_display_row as u32) as usize;

                if current_buf_row_idx >= end_display_row {
                    break; // 超出可见范围
                }

                if let Some(row) = self.buffer.get(current_buf_row_idx) {
                    for screen_x_offset in actual_start_char_x..actual_end_char_x {
                        if let Some(Some(ConsoleChar { ch, fg, bg })) =
                            row.get(screen_x_offset as usize)
                        {
                            let x_px = screen_x_offset * self.font_width;
                            let y_px = screen_y_offset * self.font_height;
                            chars_to_draw.push((*ch, x_px, y_px, *fg, *bg));
                        }
                    }
                }
            }
        }

        // 绘制所有收集到的字符
        for (ch, x_px, y_px, fg, bg) in chars_to_draw {
            self.draw_char_to_screen_at_px(ch, x_px, y_px, fg, bg);
        }

        self.draw_cursor(); // 绘制光标
        self.renderer.present(); // 刷新屏幕
    }

    /// 写入一个字符串到控制台
    pub fn write_string(&mut self, string: &str) {
        let old_cursor_x = self.cursor_x;
        let old_cursor_y = self.cursor_y;

        for c in string.chars() {
            match c {
                '\n' => {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                    self.ensure_buffer_capacity();
                }
                '\r' => {
                    self.cursor_x = 0;
                }
                '\t' => {
                    let mut spaces_to_add = TAB_SPACES as u32 - (self.cursor_x % TAB_SPACES as u32);
                    if spaces_to_add == 0 {
                        spaces_to_add = TAB_SPACES as u32;
                    }
                    for _ in 0..spaces_to_add {
                        // `put_char` 会自动标记脏区域
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
                    self.put_char(c); // `put_char` 会自动标记脏区域
                    self.cursor_x += 1;
                    if self.cursor_x >= self.width_chars {
                        self.cursor_x = 0;
                        self.cursor_y += 1;
                        self.ensure_buffer_capacity();
                    }
                }
            }
        }

        // 标记光标需要重绘，因为其位置可能已经改变，且背景色可能因为字符写入而改变
        self.cursor_needs_redraw = true;

        // 如果光标位置发生变化 (表示内容已经写入)，则触发一次渲染
        if old_cursor_x != self.cursor_x || old_cursor_y != self.cursor_y {
            // 需要清除旧光标位置。这里不再直接重绘，而是通过dirty_regions来处理
            // `put_char` 会处理单个字符的脏区域，这里只需要在最后统一刷新
            // 如果光标移动到了屏幕之外，也要标记清除旧光标位置
            self.draw_buffer_to_screen();
        } else if !self.dirty_regions.is_empty() {
            // 如果光标没动，但有脏区域，也刷新
            self.draw_buffer_to_screen();
        } else {
            // 啥也没变，但可能光标需要闪烁，所以还是要刷新
            self.renderer.present();
        }
    }

    // 设置前景颜色
    pub fn set_fg_color(&mut self, color: Color) {
        self.current_color = color;
        // 标记光标需要重绘，因为其颜色可能影响后面写入的字符
        self.cursor_needs_redraw = true;
    }

    // 设置背景颜色
    pub fn set_bg_color(&mut self, color: Color) {
        if self.current_bg_color != color {
            self.current_bg_color = color;
            // 当背景色改变时，整个屏幕可能需要重绘以应用新的背景色
            // 而不是直接调用draw_buffer_to_screen，而是标记整个屏幕为脏
            self.dirty_regions.clear();
            self.add_dirty_region(Rect::new(0, 0, self.width_chars, self.height_chars));
            self.cursor_needs_redraw = true;
            self.draw_buffer_to_screen(); // 在背景色改变后立即绘制以反映变化
        }
    }
}

// Implement the [`Write`] trait to support formatting
impl Write for Console<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
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
    DEFAULT_FONT_WRITER.lock().write_fmt(args).unwrap();
}
