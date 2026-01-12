extern crate alloc;
use crate::color;
use crate::{
    graphics::{
        color::{self, Color},
        Pixel, Renderer,
    },
    FRAMEBUFFER_REQUEST,
};
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use alloc::{collections::BTreeMap, string::String, vec, vec::Vec};
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use spin::Mutex;

pub const DEFAULT_FONT_SIZE: f32 = 8.0;
pub const TAB_SPACES: usize = 4;
pub const GLYPH_CACHE_SIZE: usize = 512;

// The default font writer
lazy_static! {
    static ref DEFAULT_FONT: FontRef<'static> = {
        let font_data = include_bytes!("../../fonts/maple-mono.ttf");
        FontRef::try_from_slice(font_data).expect("Failed to load font")
    };
    pub static ref CONSOLE: Mutex<Console<'static>> = Mutex::new({
        let renderer = Renderer::new(
            FRAMEBUFFER_REQUEST
                .get_response()
                .expect("Framebuffer request failed")
                .framebuffers()
                .next()
                .expect("No framebuffer found"),
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

#[derive(Clone)]
struct GlyphBitmap {
    bitmap: Vec<u8>,
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
}

// LRU Cache Item
struct CacheItem {
    bitmap: GlyphBitmap,
    last_used: u64,
}

struct GlyphCache {
    cache: BTreeMap<char, CacheItem>,
    counter: u64,
    size: usize,
}

impl GlyphCache {
    fn new(size: usize) -> Self {
        Self {
            cache: BTreeMap::new(),
            counter: 0,
            size,
        }
    }

    fn get(&mut self, ch: char) -> Option<GlyphBitmap> {
        if let Some(item) = self.cache.get_mut(&ch) {
            self.counter += 1;
            item.last_used = self.counter;
            return Some(item.bitmap.clone());
        }
        None
    }

    fn put(&mut self, ch: char, bitmap: GlyphBitmap) {
        self.counter += 1;
        if self.cache.len() >= self.size {
            // Evict LRU
            // Ideally we need a secondary index for O(1) eviction, but BTreeMap iteration is O(N).
            // For N=512, O(N) is acceptable for cache misses (which should be rare after warmup).
            // Finding the min `last_used`.
            if let Some((&k, _)) = self.cache.iter().min_by_key(|(_, v)| v.last_used) {
                // clone key to remove
                let key_to_remove = k;
                self.cache.remove(&key_to_remove);
            }
        }
        self.cache.insert(
            ch,
            CacheItem {
                bitmap,
                last_used: self.counter,
            },
        );
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.counter = 0;
    }
}

// ANSI 解析状态
#[derive(Debug, PartialEq, Eq)]
enum AnsiParseState {
    Normal,                // 正常模式，处理字符
    Escape,                // 收到 ESC (0x1B)
    Csi,                   // 收到 ESC [ (0x1B 0x5B)
    ParsingParams(String), // 收到 ESC [，正在收集参数（数字）
}

pub struct Console<'a> {
    pub renderer: Renderer<'a>, // 渲染器
    font: FontRef<'static>,     // 字体
    scale: PxScale,             // 字体缩放比例
    font_size: f32,             // 字体大小（pt）

    buffer: Vec<Vec<Option<ConsoleChar>>>, // 字符缓冲区
    scroll_offset_y: usize,                // 垂直滚动偏移量

    width_chars: u32,  // 宽度（字符数）
    height_chars: u32, // 高度（字符数）

    cursor_x: u32,           // 光标x位置（字符坐标）
    cursor_y: u32,           // 光标y位置（字符坐标）
    prev_cursor_x: u32,      // 上次光标x位置
    prev_cursor_y: u32,      // 上次光标y位置
    current_color: Color,    // 当前颜色
    current_bg_color: Color, // 当前背景颜色
    default_color: Color,    // 默认颜色
    default_bg_color: Color, // 默认背景颜色

    font_width: u32,    // 字体宽度（像素）
    font_height: u32,   // 字体高度（像素）
    font_baseline: f32, // 基线位置

    cursor_needs_redraw: bool,
    glyph_cache: GlyphCache,
    hidden_cursor: bool,

    ansi_parse_state: AnsiParseState, // ANSI 解析状态
}

impl<'a> Console<'a> {
    pub fn new(renderer: Renderer<'a>, font: FontRef<'static>) -> Self {
        let mut console = Self {
            renderer,
            font,
            scale: PxScale::from(0.0), // 临时值，将在 init_font_metrics 中设置
            font_size: DEFAULT_FONT_SIZE,
            cursor_x: 0,
            cursor_y: 0,
            buffer: Vec::new(), // 临时值，将在 init_font_metrics 中设置
            scroll_offset_y: 0,
            width_chars: 0,  // 临时值
            height_chars: 0, // 临时值
            current_color: color::WHITE,
            current_bg_color: color::BLACK,
            default_color: color::WHITE,
            default_bg_color: color::BLACK,
            font_width: 0,  // 临时值
            font_height: 0, // 临时值
            font_baseline: 0.0,
            cursor_needs_redraw: true,
            glyph_cache: GlyphCache::new(GLYPH_CACHE_SIZE),
            hidden_cursor: false,
            ansi_parse_state: AnsiParseState::Normal,
            prev_cursor_x: 0,
            prev_cursor_y: 0,
        };
        console.init_font_metrics(DEFAULT_FONT_SIZE); // 初始化字体度量
        console.buffer =
            vec![vec![None; console.width_chars as usize]; console.height_chars as usize]; // 初始缓冲区
        console
    }

    /// (新增) 根据当前字体和字体大小初始化/重新计算字体度量信息和缓冲区大小。
    fn init_font_metrics(&mut self, font_size_pt: f32) {
        self.font_size = font_size_pt;
        self.scale = self
            .font
            .pt_to_px_scale(font_size_pt)
            .unwrap_or(PxScale::from(16.0));
        let scaled_font = self.font.as_scaled(self.scale);

        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        let line_gap = scaled_font.line_gap();

        let font_line_height = ascent - descent + line_gap;
        self.font_baseline = ascent;

        // 获取'M'的字形边界来计算字符宽度
        let g_id = self.font.glyph_id('M');
        let g = g_id.with_scale(self.scale);
        let bound = self.font.glyph_bounds(&g);

        self.font_width = libm::ceilf(bound.width()) as u32;
        self.font_height = libm::ceilf(font_line_height) as u32;

        self.width_chars = self
            .renderer
            .width()
            .checked_div(self.font_width as u64)
            .unwrap_or(1) as u32;
        self.height_chars = self
            .renderer
            .height()
            .checked_div(self.font_height as u64)
            .unwrap_or(1) as u32;

        // 重置光标位置
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.prev_cursor_x = 0;
        self.prev_cursor_y = 0;
        self.scroll_offset_y = 0;
    }

    /// (新增) 修改字体和/或字体大小。
    ///
    /// `new_font_data`: 新字体的数据 (必须是 `'static`)。
    /// `new_font_size`: 可选的新字体大小 (磅数)。如果为 `None`，则保持当前大小。
    ///
    /// **注意**: 更改字体或字体大小将导致整个屏幕清空并重新绘制。
    pub fn set_font(&mut self, new_font_data: &'static [u8], new_font_size: Option<f32>) {
        // 尝试加载新字体
        match FontRef::try_from_slice(new_font_data) {
            Ok(new_font) => {
                self.font = new_font;
                let size_to_use = new_font_size.unwrap_or(self.font_size);
                self.init_font_metrics(size_to_use);

                // 根据新的字符宽度和高度重置缓冲区
                // 此时，需要清空旧缓冲区的内容，因为字符和布局都已改变
                //self.buffer.clear();
                //self.buffer =
                //    vec![vec![None; self.width_chars as usize]; self.height_chars as usize];

                self.cursor_needs_redraw = true;
                self.glyph_cache.clear();

                // 立即重绘以反映字体变化
                self.redraw();
            }
            Err(_) => {
                return;
            }
        }
    }

    /// 获取渲染器可变引用
    pub fn get_renderer(&mut self) -> &mut Renderer<'a> {
        &mut self.renderer
    }

    /// 隐藏光标
    pub fn cursor_hidden(&mut self) {
        self.hidden_cursor = true;
    }

    pub fn cursor_visible(&mut self) {
        self.hidden_cursor = false;
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
        self.cursor_needs_redraw = true;
        self.redraw(); // 在`clear`后立即绘制以反映变化
    }
    /// 清空渲染器上的所有像素，以背景色填充
    #[allow(dead_code)]
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
            self.cursor_needs_redraw = true; // 光标位置可能不变，但是背景变了，所以也要重新绘制
            self.redraw(); // 滚动后立即绘制屏幕
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
        if self.cursor_y >= self.height_chars {
            let lines_to_scroll = self.cursor_y - self.height_chars + 1;
            let old_scroll_offset_y = self.scroll_offset_y;
            self.scroll_offset_y = (self.scroll_offset_y as u32 + lines_to_scroll) as usize;
            self.cursor_y = self.height_chars - 1; // 将光标设置到屏幕的最后一行

            // 标记整个屏幕为脏，因为滚动导致所有可见内容移动
            if old_scroll_offset_y != self.scroll_offset_y {
                self.redraw();
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

        if needs_redraw && self.cursor_y < self.height_chars {
            self.draw_char_to_screen_at_px(
                ch,
                self.cursor_x * self.font_width,
                self.cursor_y * self.font_height,
                self.current_color,
                self.current_bg_color,
            );
        }

        self.cursor_needs_redraw = true; // 光标位置变动或背景变动都需要重绘光标
    }

    /// 渲染一个字符到屏幕的指定位置
    #[inline(always)]
    fn draw_char_to_screen_at_px(
        &mut self,
        ch: char,
        x_px: u32,
        y_px: u32,
        fg_color: Color,
        bg_color: Color,
    ) {
        let bitmap = if let Some(bm) = self.glyph_cache.get(ch) {
            bm
        } else {
            let glyph = match self
                .font
                .outline_glyph(self.font.glyph_id(ch).with_scale(self.scale))
            {
                Some(g) => g,
                None => return,
            };
            let px_bounds = glyph.px_bounds();
            let width = px_bounds.width() as u32;
            let height = px_bounds.height() as u32;
            let mut data = vec![0u8; (width * height) as usize];

            glyph.draw(|x, y, c| {
                let alpha = (c * 255.0) as u8;
                if alpha > 0 {
                    let idx = (y * width + x) as usize;
                    if idx < data.len() {
                        data[idx] = alpha;
                    }
                }
            });

            let new_bitmap = GlyphBitmap {
                bitmap: data,
                width,
                height,
                offset_x: px_bounds.min.x as i32,
                offset_y: px_bounds.min.y as i32,
            };
            self.glyph_cache.put(ch, new_bitmap.clone());
            new_bitmap
        };

        self.renderer.fill_rect(
            Pixel::new(x_px as u64, y_px as u64),
            self.font_width as u64,
            self.font_height as u64,
            bg_color,
        );

        let baseline_y = y_px as f32 + self.font_baseline;
        let start_x = (x_px as i32 + bitmap.offset_x) as u64;
        let start_y = (baseline_y + bitmap.offset_y as f32) as u64;
        for row in 0..bitmap.height {
            for col in 0..bitmap.width {
                let alpha = bitmap.bitmap[(row * bitmap.width + col) as usize];
                unsafe {
                    self.renderer.set_pixel_raw_unchecked(
                        start_x + col as u64,
                        start_y + row as u64,
                        &fg_color.mix_alpha(alpha),
                    );
                }
            }
        }
    }

    /// 绘制当前光标到屏幕
    pub fn draw_cursor(&mut self) {
        if !self.cursor_needs_redraw || self.hidden_cursor {
            return;
        }
        // 1. 清除旧光标：重绘之前光标位置的字符
        if self.prev_cursor_y < self.height_chars {
            let prev_x = self.prev_cursor_x;
            let prev_y = self.prev_cursor_y;

            if let Some(row) = self
                .buffer
                .get((prev_y + self.scroll_offset_y as u32) as usize)
            {
                if let Some(Some(char_info)) = row.get(prev_x as usize) {
                    self.draw_char_to_screen_at_px(
                        char_info.ch,
                        prev_x * self.font_width,
                        prev_y * self.font_height,
                        char_info.fg,
                        char_info.bg,
                    );
                } else {
                    // 如果旧光标位置缓冲区为空，则用背景色填充
                    self.renderer.fill_rect(
                        Pixel::new(
                            (prev_x * self.font_width) as u64,
                            (prev_y * self.font_height) as u64,
                        ),
                        self.font_width as u64,
                        self.font_height as u64,
                        self.current_bg_color,
                    );
                }
            }
        }
        // 2. 绘制新光标（仅在可见位置绘制）
        if self.cursor_y < self.height_chars {
            let cursor_x_px = self.cursor_x * self.font_width;
            let cursor_y_px = self.cursor_y * self.font_height;

            // 使用反色效果使光标可见
            let inverse_color = self.current_bg_color.invert();
            self.renderer.fill_rect(
                Pixel::new(cursor_x_px as u64, cursor_y_px as u64),
                self.font_width as u64,
                self.font_height as u64,
                inverse_color,
            );
        }
        // 3. 更新记录
        self.prev_cursor_x = self.cursor_x;
        self.prev_cursor_y = self.cursor_y;
        self.cursor_needs_redraw = false;
    }

    /// Redraws the entire visible buffer to the screen.
    pub fn redraw(&mut self) {
        self.clear_screen_pixels();

        let start_display_row = self.scroll_offset_y;
        let end_display_row =
            (self.scroll_offset_y + self.height_chars as usize).min(self.buffer.len());

        let mut chars_to_draw = Vec::new();

        for (y_offset, row) in self.buffer[start_display_row..end_display_row]
            .iter()
            .enumerate()
        {
            for (x, cell) in row.iter().enumerate() {
                if let Some(char_info) = cell {
                    chars_to_draw.push((
                        char_info.ch,
                        x as u32 * self.font_width,
                        y_offset as u32 * self.font_height,
                        char_info.fg,
                        char_info.bg,
                    ));
                }
            }
        }

        for (ch, x, y, fg, bg) in chars_to_draw {
            self.draw_char_to_screen_at_px(ch, x, y, fg, bg);
        }

        self.draw_cursor();
        self.renderer.present();
    }

    /// 写入一个字符串到控制台
    pub fn write_string(&mut self, string: &str) {
        for c in string.chars() {
            match self.ansi_parse_state {
                AnsiParseState::Normal => {
                    match c {
                        '\x1b' => {
                            // ESC
                            self.ansi_parse_state = AnsiParseState::Escape;
                        }
                        '\n' => {
                            self.cursor_x = 0;
                            self.cursor_y += 1;
                            self.ensure_buffer_capacity();
                        }
                        '\r' => {
                            self.cursor_x = 0;
                        }
                        '\t' => {
                            let mut spaces_to_add =
                                TAB_SPACES as u32 - (self.cursor_x % TAB_SPACES as u32);
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
                AnsiParseState::Escape => {
                    match c {
                        '[' => {
                            // CSI (Control Sequence Introducer)
                            self.ansi_parse_state = AnsiParseState::Csi;
                        }
                        _ => {
                            // Not a CSI sequence, treat ESC and the char as literals or ignore
                            self.ansi_parse_state = AnsiParseState::Normal;
                        }
                    }
                }
                AnsiParseState::Csi => {
                    if c.is_ascii_digit() {
                        let mut params = String::new();
                        params.push(c);
                        self.ansi_parse_state = AnsiParseState::ParsingParams(params);
                    } else if c == 'm' {
                        // No parameters, just ESC[m (reset)
                        self.apply_ansi_codes(&[0]); // Apply default reset
                        self.ansi_parse_state = AnsiParseState::Normal;
                    } else {
                        // Malformed CSI sequence (e.g., ESC[A for cursor up, not supported yet)
                        self.ansi_parse_state = AnsiParseState::Normal;
                    }
                }
                AnsiParseState::ParsingParams(ref mut params_str) => {
                    if c.is_ascii_digit() || c == ';' {
                        params_str.push(c);
                    } else if c == 'm' {
                        // SGR (Select Graphic Rendition) sequence end
                        let codes: Vec<u32> = params_str
                            .split(';')
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        self.apply_ansi_codes(&codes);
                        self.ansi_parse_state = AnsiParseState::Normal;
                    } else {
                        // Malformed or unsupported sequence, reset state
                        self.ansi_parse_state = AnsiParseState::Normal;
                    }
                }
            }
        }

        // 标记新的光标位置需要重绘
        self.cursor_needs_redraw = true;

        self.draw_cursor();
        self.renderer.present();
    }

    /// 将 ANSI 颜色代码映射到 `graphics::color::Color`
    fn ansi_code_to_color(code: u32) -> Option<Color> {
        match code {
            30 | 40 => Some(color::BLACK),
            31 | 41 => Some(color::RED),
            32 | 42 => Some(color::GREEN),
            33 | 43 => Some(color::YELLOW),
            34 | 44 => Some(color::BLUE),
            35 | 45 => Some(color::MAGENTA),
            36 | 46 => Some(color::CYAN),
            37 | 47 => Some(color::WHITE),

            90 => Some(color!(128, 128, 128)), // Bright Black (Dark Gray)
            91 => Some(color!(255, 100, 100)), // Bright Red
            92 => Some(color!(100, 255, 100)), // Bright Green
            93 => Some(color!(255, 255, 100)), // Bright Yellow
            94 => Some(color!(100, 100, 255)), // Bright Blue
            95 => Some(color!(255, 100, 255)), // Bright Magenta
            96 => Some(color!(100, 255, 255)), // Bright Cyan
            97 => Some(color!(255, 255, 255)), // Bright White

            100 => Some(color!(64, 64, 64)), // Bright Black Background
            101 => Some(color!(150, 0, 0)),  // Bright Red Background
            102 => Some(color!(0, 150, 0)),  // Bright Green Background
            103 => Some(color!(150, 150, 0)), // Bright Yellow Background
            104 => Some(color!(0, 0, 150)),  // Bright Blue Background
            105 => Some(color!(150, 0, 150)), // Bright Magenta Background
            106 => Some(color!(0, 150, 150)), // Bright Cyan Background
            107 => Some(color!(150, 150, 150)), // Bright White Background
            _ => None,
        }
    }

    /// 应用 ANSI SGR (Select Graphic Rendition) 代码
    fn apply_ansi_codes(&mut self, codes: &[u32]) {
        if codes.is_empty() {
            // ESC[m 或 ESC[0m 默认重置所有属性
            self.set_fg_color(self.default_color);
            self.set_bg_color(self.default_bg_color);
            return;
        }

        for &code in codes {
            match code {
                0 => {
                    // Reset all attributes
                    self.set_fg_color(self.default_color);
                    self.set_bg_color(self.default_bg_color);
                }
                30..=37 => {
                    if let Some(color) = Self::ansi_code_to_color(code) {
                        self.set_fg_color(color);
                    }
                }
                39 => {
                    // Default foreground color
                    self.set_fg_color(self.default_color);
                }

                // Background colors (40-47)
                40..=47 => {
                    if let Some(color) = Self::ansi_code_to_color(code) {
                        self.set_bg_color(color);
                    }
                }
                49 => {
                    // Default background color
                    self.set_bg_color(self.default_bg_color);
                }

                // Bright foreground colors (90-97)
                90..=97 => {
                    if let Some(color) = Self::ansi_code_to_color(code) {
                        self.set_fg_color(color);
                    }
                }

                // Bright background colors (100-107)
                100..=107 => {
                    if let Some(color) = Self::ansi_code_to_color(code) {
                        self.set_bg_color(color);
                    }
                }
                // Other SGR codes (e.g., underlining, inverse, etc.) are ignored for now.
                _ => {}
            }
        }
    }

    /// 设置前景颜色
    pub fn set_fg_color(&mut self, color: Color) {
        if self.current_color != color {
            self.current_color = color;
            // 标记光标需要重绘，因为其颜色可能影响后面写入的字符
            self.cursor_needs_redraw = true;
        }
    }

    /// 设置背景颜色
    pub fn set_bg_color(&mut self, color: Color) {
        if self.current_bg_color != color {
            self.current_bg_color = color;
            // 当背景色改变时，整个屏幕可能需要重绘以应用新的背景色
            self.cursor_needs_redraw = true;
            self.redraw();
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
    CONSOLE
        .lock()
        .write_fmt(args)
        .expect("Failed to write to console");
}
