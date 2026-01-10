//! Console output implementation with text rendering and ANSI escape code support
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! This module provides a graphical console implementation that renders text
//! to the framebuffer with support for TrueType fonts, ANSI escape codes,
//! and efficient dirty region tracking for partial screen updates.
//!
//! # Features
//!
//! - TrueType font rendering with anti-aliasing
//! - ANSI escape code support for colors and formatting
//! - Efficient dirty region tracking for partial screen updates
//! - Cursor management with blinking support
//! - Scrolling buffer for command-line history
//! - Tab expansion and line wrapping
//!
//! # Examples
//!
//! ```rust
//! use kernel::output::console::CONSOLE;
//! use kernel::println;
//!
//! // Print to console
//! println!("Hello, World!");
//!
//! // Change colors using ANSI codes
//! println!("\x1b[31mRed text\x1b[0m");
//! println!("\x1b[32;44mGreen text on blue background\x1b[0m");
//! ```
//!
//! # ANSI Escape Codes
//!
//! The console supports a subset of ANSI escape codes:
//!
//! - `\x1b[0m` - Reset all attributes
//! - `\x1b[30-37m` - Set foreground color (black, red, green, yellow, blue, magenta, cyan, white)
//! - `\x1b[40-47m` - Set background color
//! - `\x1b[90-97m` - Set bright foreground color
//! - `\x1b[100-107m` - Set bright background color
//!
//! # Safety
//!
//! This module uses unsafe operations for framebuffer access and requires
//! proper initialization before use. The global `CONSOLE` instance must be
//! initialized with a valid framebuffer.

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
use alloc::{string::String, vec, vec::Vec};
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use libm::ceilf;
use spin::Mutex;

/// Default font size in points
pub const DEFAULT_FONT_SIZE: f32 = 12.0;

/// Number of spaces to expand tabs to
pub const TAB_SPACES: usize = 4;

// The default font writer
lazy_static! {
    /// Default TrueType font loaded from embedded resources
    static ref DEFAULT_FONT: FontRef<'static> = {
        let font_data = include_bytes!("../../fonts/maple-mono.ttf");
        FontRef::try_from_slice(font_data).expect("Failed to load font")
    };
    
    /// Global console instance protected by a mutex for thread-safe access
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
///
/// This structure stores the character along with its display attributes
/// for efficient rendering and comparison.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ConsoleChar {
    /// The character to display
    ch: char,
    /// Foreground color
    fg: Color,
    /// Background color
    bg: Color,
}

/// Represents a rectangular region on the screen in character coordinates.
///
/// Used for tracking dirty regions that need redrawing to optimize
/// screen updates by only redrawing changed areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    /// X coordinate of the top-left corner (in character units)
    pub x: u32,
    /// Y coordinate of the top-left corner (in character units)
    pub y: u32,
    /// Width of the rectangle (in character units)
    pub width: u32,
    /// Height of the rectangle (in character units)
    pub height: u32,
}

impl Rect {
    /// Creates a new rectangle with the specified position and dimensions.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of the top-left corner
    /// * `y` - Y coordinate of the top-left corner
    /// * `width` - Width of the rectangle
    /// * `height` - Height of the rectangle
    ///
    /// # Returns
    ///
    /// Returns a new `Rect` instance.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Checks if this rectangle overlaps with another rectangle.
    ///
    /// # Arguments
    ///
    /// * `other` - The other rectangle to check for overlap
    ///
    /// # Returns
    ///
    /// Returns `true` if the rectangles overlap, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::Rect;
    ///
    /// let rect1 = Rect::new(0, 0, 10, 10);
    /// let rect2 = Rect::new(5, 5, 10, 10);
    /// assert!(rect1.overlaps_with(&rect2));
    ///
    /// let rect3 = Rect::new(20, 20, 5, 5);
    /// assert!(!rect1.overlaps_with(&rect3));
    /// ```
    pub fn overlaps_with(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Merges this rectangle with another, returning a new rectangle that encompasses both.
    ///
    /// # Arguments
    ///
    /// * `other` - The other rectangle to merge with
    ///
    /// # Returns
    ///
    /// Returns a new `Rect` that contains both input rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::Rect;
    ///
    /// let rect1 = Rect::new(0, 0, 10, 10);
    /// let rect2 = Rect::new(5, 5, 10, 10);
    /// let merged = rect1.merge(&rect2);
    /// assert_eq!(merged.x, 0);
    /// assert_eq!(merged.y, 0);
    /// assert_eq!(merged.width, 15);
    /// assert_eq!(merged.height, 15);
    /// ```
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

/// ANSI escape code parsing state
///
/// Tracks the current state of ANSI escape sequence parsing.
#[derive(Debug, PartialEq, Eq)]
enum AnsiParseState {
    /// Normal mode, processing regular characters
    Normal,
    /// Received ESC (0x1B) character
    Escape,
    /// Received ESC [ (Control Sequence Introducer)
    Csi,
    /// Received ESC [, collecting numeric parameters
    ParsingParams(String),
}

/// Graphical console for rendering text to the framebuffer
///
/// The console manages a character buffer, renders text using TrueType fonts,
/// supports ANSI escape codes for colors and formatting, and implements
/// efficient screen updates through dirty region tracking.
///
/// # Lifetime
///
/// The `'a` lifetime parameter is tied to the framebuffer renderer,
/// ensuring the console doesn't outlive the framebuffer resources.
pub struct Console<'a> {
    /// Renderer for drawing to the framebuffer
    pub renderer: Renderer<'a>,
    /// TrueType font for text rendering
    font: FontRef<'static>,
    /// Font scaling factor
    scale: PxScale,
    /// Font size in points
    font_size: f32,

    /// Character buffer storing display attributes for each screen position
    buffer: Vec<Vec<Option<ConsoleChar>>>,
    /// Vertical scroll offset for viewing buffer history
    scroll_offset_y: usize,

    /// Screen width in characters
    width_chars: u32,
    /// Screen height in characters
    height_chars: u32,

    /// Current cursor X position (character coordinates)
    cursor_x: u32,
    /// Current cursor Y position (character coordinates)
    cursor_y: u32,
    /// Previous cursor X position for cursor tracking
    prev_cursor_x: u32,
    /// Previous cursor Y position for cursor tracking
    prev_cursor_y: u32,
    /// Current foreground color
    current_color: Color,
    /// Current background color
    current_bg_color: Color,
    /// Default foreground color (used for reset)
    default_color: Color,
    /// Default background color (used for reset)
    default_bg_color: Color,

    /// Font width in pixels
    font_width: u32,
    /// Font height in pixels
    font_height: u32,
    /// Font baseline position for proper character alignment
    font_baseline: f32,

    /// Dirty regions that need to be redrawn (character coordinates)
    dirty_regions: Vec<Rect>,
    /// Flag indicating if the cursor needs to be redrawn
    cursor_needs_redraw: bool,

    /// Flag to hide the cursor (e.g., during password input)
    hidden_cursor: bool,
    /// Current state of ANSI escape code parsing
    ansi_parse_state: AnsiParseState,
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
            font_width: 0,      // 临时值
            font_height: 0,     // 临时值
            font_baseline: 0.0, // 临时值
            dirty_regions: Vec::new(),
            cursor_needs_redraw: true,
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
    /// Creates a new console with the specified renderer and font.
    ///
    /// # Arguments
    ///
    /// * `renderer` - Renderer for drawing to the framebuffer
    /// * `font` - TrueType font for text rendering
    ///
    /// # Returns
    ///
    /// Returns a new `Console` instance initialized with default settings.
    ///
    /// # Panics
    ///
    /// This function may panic if the font cannot be properly scaled
    /// or if the renderer dimensions are invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::{Console, DEFAULT_FONT};
    /// use kernel::graphics::Renderer;
    /// 
    /// // Assuming a framebuffer is available
    /// let renderer = Renderer::new(framebuffer);
    /// let console = Console::new(renderer, DEFAULT_FONT.clone());
    /// ```

    /// Initializes or recalculates font metrics and buffer size based on current font and font size.
    ///
    /// This method calculates font dimensions (width, height, baseline) based on the current
    /// font and specified point size, then adjusts the console buffer dimensions accordingly.
    ///
    /// # Arguments
    ///
    /// * `font_size_pt` - Font size in points
    ///
    /// # Notes
    ///
    /// - This method is called automatically when the font is changed via `set_font`
    /// - It resets cursor positions and scroll offset
    /// - The character buffer dimensions are recalculated based on screen resolution and font metrics
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

        self.font_width = ceilf(bound.width()) as u32;
        self.font_height = ceilf(font_line_height) as u32;

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

    /// Changes the font and/or font size used by the console.
    ///
    /// This method loads a new TrueType font from the provided data and optionally
    /// changes the font size. Changing the font or font size will clear the entire
    /// screen and trigger a complete redraw.
    ///
    /// # Arguments
    ///
    /// * `new_font_data` - Raw TrueType font data (must have `'static` lifetime)
    /// * `new_font_size` - Optional new font size in points. If `None`, the current
    ///   font size is preserved.
    ///
    /// # Returns
    ///
    /// This method returns nothing. If the font cannot be loaded, it silently fails
    /// and preserves the current font.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// // Load a custom font
    /// let font_data = include_bytes!("path/to/custom-font.ttf");
    /// CONSOLE.lock().set_font(font_data, Some(14.0));
    /// ```
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

                // 标记整个屏幕为脏，需要完全重绘
                self.dirty_regions.clear();
                self.add_dirty_region(Rect::new(0, 0, self.width_chars, self.height_chars));
                self.cursor_needs_redraw = true;

                // 立即重绘以反映字体变化
                self.draw_buffer_to_screen();
            }
            Err(_) => {
                return;
            }
        }
    }

    /// Returns a mutable reference to the underlying renderer.
    ///
    /// This method provides direct access to the renderer for advanced
    /// graphics operations that are not covered by the console's API.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the `Renderer<'a>` instance.
    ///
    /// # Safety
    ///
    /// Direct manipulation of the renderer may interfere with console
    /// operations and cause rendering artifacts. Use with caution.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// let renderer = console.get_renderer();
    /// // Perform custom rendering operations
    /// ```
    pub fn get_renderer(&mut self) -> &mut Renderer<'a> {
        &mut self.renderer
    }

    /// Adds a dirty region to the list of areas that need to be redrawn.
    ///
    /// Dirty regions are tracked to optimize screen updates by only
    /// redrawing the parts of the screen that have changed. This method
    /// automatically merges overlapping regions to minimize the number
    /// of redraw operations.
    ///
    /// # Arguments
    ///
    /// * `region` - A `Rect` representing the dirty area in character coordinates
    ///
    /// # Notes
    ///
    /// - The region coordinates are in character units, not pixels
    /// - Overlapping regions are automatically merged to reduce redraw overhead
    /// - This method is called internally when characters are written or modified
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

    /// Hides the cursor from the screen.
    ///
    /// This method is useful for operations where cursor visibility
    /// would be distracting, such as during password input or when
    /// performing batch operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.cursor_hidden();
    /// // Perform operations without visible cursor
    /// console.cursor_visible(); // Restore cursor visibility
    /// ```
    pub fn cursor_hidden(&mut self) {
        self.hidden_cursor = true;
    }

    /// Makes the cursor visible on the screen.
    ///
    /// This method restores cursor visibility after it has been
    /// hidden using `cursor_hidden()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.cursor_hidden();
    /// // Perform operations without visible cursor
    /// console.cursor_visible(); // Restore cursor visibility
    /// ```
    pub fn cursor_visible(&mut self) {
        self.hidden_cursor = false;
    }

    /// Clears the entire console buffer and screen.
    ///
    /// This method resets the console to a blank state by:
    /// - Clearing all characters from the buffer
    /// - Resetting cursor position to (0, 0)
    /// - Resetting scroll offset to 0
    /// - Marking the entire screen as dirty for redraw
    /// - Immediately redrawing the screen to reflect changes
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.clear(); // Clears the entire console
    /// ```
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
    /// Clears all pixels on the renderer by filling with the current background color.
    ///
    /// This method is marked as `#[allow(dead_code)]` because it may not be used
    /// in all console configurations, but is kept for completeness and potential
    /// future use cases.
    ///
    /// # Notes
    ///
    /// - Temporarily changes the renderer's clear color to the current background color
    /// - Restores the original clear color after clearing
    /// - This is a lower-level operation than `clear()` which operates on the character buffer
    #[allow(dead_code)]
    fn clear_screen_pixels(&mut self) {
        let raw_clear_color = self.renderer.get_clear_color(); // 保存原始清除色
        self.renderer.set_clear_color(self.current_bg_color);
        self.renderer.clear();
        self.renderer.set_clear_color(raw_clear_color); // 恢复原始清除色
    }

    /// Scrolls the console buffer by the specified number of lines.
    ///
    /// Positive values scroll down (showing older content), negative values scroll up
    /// (showing newer content). The scroll offset is clamped to valid bounds.
    ///
    /// # Arguments
    ///
    /// * `lines` - Number of lines to scroll:
    ///   - `lines > 0`: Scroll down (show older content)
    ///   - `lines < 0`: Scroll up (show newer content)
    ///   - `lines = 0`: No effect
    ///
    /// # Notes
    ///
    /// - Scrolling marks the entire screen as dirty for redraw
    /// - Cursor position is preserved relative to the viewport
    /// - The scroll offset is clamped to prevent scrolling beyond buffer boundaries
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.scroll(5);  // Scroll down 5 lines
    /// console.scroll(-2); // Scroll up 2 lines
    /// ```
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

    /// Ensures the buffer has enough rows to accommodate new content and scrolls if necessary.
    ///
    /// This method manages the console buffer's capacity by:
    /// 1. Adding new rows to the buffer when the cursor position exceeds the current buffer size
    /// 2. Automatically scrolling the viewport when the cursor moves beyond the visible screen area
    ///
    /// # Behavior
    ///
    /// - When the cursor moves to a row beyond the current buffer size, new empty rows are added
    /// - When the cursor moves beyond the bottom of the visible screen, the viewport scrolls down
    /// - Scrolling marks the entire screen as dirty for redraw
    /// - The cursor is repositioned to the last visible line after scrolling
    ///
    /// # Notes
    ///
    /// - This method is called automatically before writing characters to the buffer
    /// - It maintains the invariant that the buffer always has enough rows for the current cursor position
    /// - Scrolling preserves the scroll offset for viewing command history
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
                self.dirty_regions.clear();
                self.add_dirty_region(Rect::new(0, 0, self.width_chars, self.height_chars));
            }
        }
    }

    /// Writes a single character to the buffer and renders it to the screen.
    ///
    /// This method handles the core character writing logic by:
    /// 1. Ensuring buffer capacity for the current cursor position
    /// 2. Checking if the character at the cursor position has changed
    /// 3. Updating the character buffer with the new character and colors
    /// 4. Marking the affected area as dirty for redraw if needed
    /// 5. Flagging the cursor for redraw since its position or background may have changed
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to write to the console
    ///
    /// # Behavior
    ///
    /// - If the character and colors at the cursor position are identical to the new character,
    ///   no redraw is performed (optimization)
    /// - Only the visible portion of the screen is marked as dirty
    /// - The cursor is always flagged for redraw after character writing
    ///
    /// # Notes
    ///
    /// - This method is called internally by `write_string` for each character
    /// - Character coordinates are in character units, not pixels
    /// - The method respects the current foreground and background colors
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

    /// Renders a character to the screen at the specified pixel coordinates.
    ///
    /// This is the low-level character rendering method that:
    /// 1. Fills the character cell background with the specified background color
    /// 2. Extracts the glyph outline from the TrueType font
    /// 3. Renders the glyph with anti-aliasing using alpha blending
    /// 4. Applies proper positioning based on font metrics (baseline, bounds)
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to render
    /// * `x_px` - X coordinate in pixels (top-left corner of character cell)
    /// * `y_px` - Y coordinate in pixels (top-left corner of character cell)
    /// * `fg_color` - Foreground color for the character
    /// * `bg_color` - Background color for the character cell
    ///
    /// # Implementation Details
    ///
    /// - Uses `ab_glyph` for font rendering with anti-aliasing
    /// - Applies alpha blending to mix foreground color with glyph alpha values
    /// - Respects font baseline for proper vertical alignment
    /// - Includes bounds checking to prevent drawing outside the screen
    /// - Optimized with `#[inline(always)]` for performance in tight rendering loops
    ///
    /// # Notes
    ///
    /// - This method operates in pixel coordinates, not character coordinates
    /// - The character cell size is determined by `font_width` and `font_height`
    /// - Alpha values from the glyph are used to blend the foreground color with the background
    #[inline(always)]
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

    /// Draws the cursor to the screen at its current position.
    ///
    /// This method handles cursor rendering by:
    /// 1. Erasing the previous cursor by redrawing the character at the old cursor position
    /// 2. Drawing the new cursor at the current cursor position using inverse color effect
    /// 3. Updating internal tracking of cursor position for the next redraw
    ///
    /// # Behavior
    ///
    /// - The cursor is only drawn if `cursor_needs_redraw` is `true` and the cursor is not hidden
    /// - The cursor uses an inverse color effect (background becomes foreground and vice versa)
    /// - Previous cursor position is restored by redrawing the character that was there
    /// - Cursor is only drawn within visible screen bounds
    ///
    /// # Notes
    ///
    /// - This method is called automatically by `draw_buffer_to_screen`
    /// - Cursor visibility can be controlled with `cursor_hidden()` and `cursor_visible()`
    /// - The cursor occupies a full character cell for maximum visibility
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.draw_cursor(); // Manually trigger cursor redraw
    /// ```
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

    /// Renders dirty regions from the buffer to the screen.
    ///
    /// This method is the main rendering engine of the console that:
    /// 1. Checks if any rendering is needed (dirty regions or cursor redraw)
    /// 2. Collects all characters from dirty regions that need to be redrawn
    /// 3. Clears the background of dirty regions with the current background color
    /// 4. Renders all collected characters to their pixel positions
    /// 5. Draws the cursor if needed
    /// 6. Presents the final frame to the screen
    ///
    /// # Optimization
    ///
    /// - Only redraws characters within dirty regions to minimize rendering work
    /// - Collects character information before rendering to avoid borrowing conflicts
    /// - Clears background in bulk before character rendering for efficiency
    /// - Skips rendering entirely if no dirty regions exist and cursor doesn't need redraw
    ///
    /// # Behavior
    ///
    /// - Dirty regions are processed and cleared after rendering
    /// - Characters are rendered with their stored foreground and background colors
    /// - The cursor is drawn after characters to ensure it's always visible
    /// - The renderer's `present()` method is called to update the display
    ///
    /// # Notes
    ///
    /// - This method is called automatically when content changes or cursor moves
    /// - It respects the current scroll offset when accessing the character buffer
    /// - Character coordinates are converted to pixel coordinates based on font metrics
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// // Write some text to create dirty regions
    /// console.write_string("Hello");
    /// // Manually trigger screen update
    /// console.draw_buffer_to_screen();
    /// ```
    pub fn draw_buffer_to_screen(&mut self) {
        // 如果没有脏区域且光标不需要重绘，则无需进行任何渲染操作
        if self.dirty_regions.is_empty() && !self.cursor_needs_redraw {
            // self.renderer.present();
            return;
        }

        let start_display_row = self.scroll_offset_y;
        let end_display_row =
            (self.scroll_offset_y + self.height_chars as usize).min(self.buffer.len());

        // 收集所有需要绘制的字符信息，避免在借用self时调用self的方法
        let mut chars_to_draw: Vec<(char, u32, u32, Color, Color)> = Vec::new();

        // 遍历脏区域，只重绘这些区域内的字符
        let mut regions_to_clear_bg = Vec::new(); // 存储需要先用背景色填充的像素区域

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

            regions_to_clear_bg.push((
                Pixel::new(px_x as u64, px_y as u64),
                px_width as u64,
                px_height as u64,
            ));

            for screen_y_offset in actual_start_char_y..actual_end_char_y {
                let current_buf_row_idx = (screen_y_offset + start_display_row as u32) as usize;

                if current_buf_row_idx >= end_display_row {
                    continue; // 超出可见范围
                }

                // 检查缓冲区行是否存在，以防止索引越界
                if let Some(row) = self.buffer.get(current_buf_row_idx) {
                    for screen_x_offset in actual_start_char_x..actual_end_char_x {
                        // 检查缓冲区单元格是否存在
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

        // 先用当前背景色填充所有脏区域的像素，确保背景色更新
        for (pos, w, h) in regions_to_clear_bg {
            self.renderer.fill_rect(pos, w, h, self.current_bg_color);
        }

        // 绘制所有收集到的字符
        for (ch, x_px, y_px, fg, bg) in chars_to_draw {
            self.draw_char_to_screen_at_px(ch, x_px, y_px, fg, bg);
        }

        self.draw_cursor(); // 绘制光标
        self.renderer.present(); // 刷新屏幕
    }

    /// Writes a string to the console with full text processing.
    ///
    /// This is the primary method for outputting text to the console. It handles:
    /// - Character-by-character processing with proper cursor movement
    /// - Special character handling (newline, carriage return, tab)
    /// - ANSI escape code parsing for colors and formatting
    /// - Automatic screen updates when content changes
    ///
    /// # Arguments
    ///
    /// * `string` - The string to write to the console
    ///
    /// # Text Processing
    ///
    /// The method processes each character in the string:
    /// - `\n` (newline): Moves cursor to beginning of next line
    /// - `\r` (carriage return): Moves cursor to beginning of current line
    /// - `\t` (tab): Expands to spaces based on `TAB_SPACES` constant
    /// - `\x1b` (ESC): Begins ANSI escape sequence parsing
    /// - Regular characters: Written at current cursor position
    ///
    /// # ANSI Escape Code Support
    ///
    /// The method parses ANSI escape sequences in the format:
    /// - `ESC[` (Control Sequence Introducer)
    /// - Optional numeric parameters separated by semicolons
    /// - Ending with `m` (Select Graphic Rendition)
    ///
    /// Example: `\x1b[31;42m` sets foreground to red and background to green
    ///
    /// # Screen Updates
    ///
    /// - Automatically triggers screen redraw when content changes
    /// - Marks dirty regions for efficient partial updates
    /// - Updates cursor position and handles cursor redraw
    /// - Calls `draw_buffer_to_screen()` when needed
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.write_string("Hello, World!\n");
    /// console.write_string("\x1b[31mRed text\x1b[0m\n");
    /// console.write_string("Tab\tseparated\tvalues\n");
    /// ```
    pub fn write_string(&mut self, string: &str) {
        let mut changed_content = false; // 标记是否有实际内容写入
        let mut cursor_moved = false; // 标记光标是否移动，包括换行、回车、tab

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
                            cursor_moved = true;
                        }
                        '\r' => {
                            self.cursor_x = 0;
                            cursor_moved = true;
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
                            changed_content = true;
                            cursor_moved = true;
                        }
                        _ => {
                            self.put_char(c);
                            self.cursor_x += 1;
                            if self.cursor_x >= self.width_chars {
                                self.cursor_x = 0;
                                self.cursor_y += 1;
                                self.ensure_buffer_capacity();
                            }
                            changed_content = true;
                            cursor_moved = true;
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
                        changed_content = true; // 颜色变化也算作内容变化
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
                        changed_content = true; // 颜色变化也算作内容变化
                    } else {
                        // Malformed or unsupported sequence, reset state
                        self.ansi_parse_state = AnsiParseState::Normal;
                    }
                }
            }
        }

        // 如果光标移动了，将旧的光标位置标记为脏区域，以确保它被重绘（从而清除光标块）
        if self.cursor_x != self.prev_cursor_x || self.cursor_y != self.prev_cursor_y {
            // 仅当旧光标位置在屏幕内时才标记它为脏
            if self.prev_cursor_y < self.height_chars {
                self.add_dirty_region(Rect::new(self.prev_cursor_x, self.prev_cursor_y, 1, 1));
            }
            cursor_moved = true;
        }

        // 标记新的光标位置需要重绘
        self.cursor_needs_redraw = true;

        // 如果内容有变化或光标移动了，或者有脏区域，则触发一次渲染
        if changed_content || cursor_moved || !self.dirty_regions.is_empty() {
            self.draw_buffer_to_screen();
        } else {
            // 啥也没变，但可能光标需要闪烁，所以还是要刷新
            self.renderer.present();
        }
    }

    /// Maps ANSI color codes to `graphics::color::Color` values.
    ///
    /// This method converts standard ANSI color codes (30-37, 40-47, 90-97, 100-107)
    /// to their corresponding RGB color values. The mapping follows the standard
    /// ANSI color scheme with support for both normal and bright variants.
    ///
    /// # Arguments
    ///
    /// * `code` - ANSI color code to convert
    ///
    /// # Returns
    ///
    /// Returns `Some(Color)` if the code corresponds to a valid ANSI color,
    /// or `None` if the code is not recognized.
    ///
    /// # ANSI Color Codes
    ///
    /// The following codes are supported:
    ///
    /// ## Foreground Colors (30-37)
    /// - 30: Black
    /// - 31: Red
    /// - 32: Green
    /// - 33: Yellow
    /// - 34: Blue
    /// - 35: Magenta
    /// - 36: Cyan
    /// - 37: White
    ///
    /// ## Background Colors (40-47)
    /// - 40: Black
    /// - 41: Red
    /// - 42: Green
    /// - 43: Yellow
    /// - 44: Blue
    /// - 45: Magenta
    /// - 46: Cyan
    /// - 47: White
    ///
    /// ## Bright Foreground Colors (90-97)
    /// - 90: Bright Black (Dark Gray)
    /// - 91: Bright Red
    /// - 92: Bright Green
    /// - 93: Bright Yellow
    /// - 94: Bright Blue
    /// - 95: Bright Magenta
    /// - 96: Bright Cyan
    /// - 97: Bright White
    ///
    /// ## Bright Background Colors (100-107)
    /// - 100: Bright Black Background
    /// - 101: Bright Red Background
    /// - 102: Bright Green Background
    /// - 103: Bright Yellow Background
    /// - 104: Bright Blue Background
    /// - 105: Bright Magenta Background
    /// - 106: Bright Cyan Background
    /// - 107: Bright White Background
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::Console;
    /// use kernel::graphics::color;
    ///
    /// // Get color for ANSI code 31 (red)
    /// let color = Console::ansi_code_to_color(31);
    /// assert_eq!(color, Some(color::RED));
    ///
    /// // Get color for ANSI code 92 (bright green)
    /// let color = Console::ansi_code_to_color(92);
    /// assert_eq!(color, Some(color!(100, 255, 100)));
    ///
    /// // Invalid code returns None
    /// let color = Console::ansi_code_to_color(255);
    /// assert_eq!(color, None);
    /// ```
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

    /// Applies ANSI SGR (Select Graphic Rendition) codes to modify console attributes.
    ///
    /// This method processes ANSI escape sequence parameters to change text attributes
    /// such as foreground color, background color, and other formatting options.
    /// It's called internally when ANSI escape sequences are parsed by `write_string`.
    ///
    /// # Arguments
    ///
    /// * `codes` - Slice of ANSI SGR codes to apply
    ///
    /// # Supported SGR Codes
    ///
    /// The following SGR codes are currently supported:
    ///
    /// ## Color Codes
    /// - `0`: Reset all attributes to defaults
    /// - `30-37`: Set foreground color (black, red, green, yellow, blue, magenta, cyan, white)
    /// - `39`: Default foreground color
    /// - `40-47`: Set background color
    /// - `49`: Default background color
    /// - `90-97`: Set bright foreground color
    /// - `100-107`: Set bright background color
    ///
    /// # Behavior
    ///
    /// - Empty codes slice (`ESC[m`) is treated as reset (code 0)
    /// - Multiple codes can be combined with semicolons (e.g., `ESC[31;42m`)
    /// - Codes are processed in order, with later codes potentially overriding earlier ones
    /// - Unsupported codes are silently ignored
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::Console;
    /// use kernel::graphics::color;
    ///
    /// // Assuming a console instance exists
    /// let mut console = Console::new(renderer, font);
    ///
    /// // Reset all attributes
    /// console.apply_ansi_codes(&[0]);
    ///
    /// // Set red foreground and green background
    /// console.apply_ansi_codes(&[31, 42]);
    ///
    /// // Set bright cyan foreground
    /// console.apply_ansi_codes(&[96]);
    /// ```
    ///
    /// # Notes
    ///
    /// - This method is called internally by the ANSI escape code parser
    /// - Color codes are mapped using `ansi_code_to_color`
    /// - Changing background color marks the entire screen as dirty for redraw
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

    /// Sets the foreground color for subsequently written characters.
    ///
    /// This method changes the color used for drawing text characters.
    /// The change takes effect immediately for all future character writes
    /// but does not retroactively change already-drawn characters.
    ///
    /// # Arguments
    ///
    /// * `color` - The new foreground color as a `Color` value
    ///
    /// # Behavior
    ///
    /// - If the new color is different from the current color, the cursor is marked for redraw
    /// - The change does not trigger an immediate screen redraw
    /// - Characters written after this call will use the new foreground color
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// use kernel::graphics::color;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.set_fg_color(color::RED);
    /// console.write_string("This text will be red\n");
    /// console.set_fg_color(color::GREEN);
    /// console.write_string("This text will be green\n");
    /// ```
    pub fn set_fg_color(&mut self, color: Color) {
        if self.current_color != color {
            self.current_color = color;
            // Mark cursor for redraw since its color may affect subsequently written characters
            self.cursor_needs_redraw = true;
        }
    }

    /// Sets the background color for the console.
    ///
    /// This method changes the background color used for the entire console.
    /// Unlike foreground color changes, background color changes affect the
    /// entire screen and require a complete redraw to apply the new color
    /// to all character cells.
    ///
    /// # Arguments
    ///
    /// * `color` - The new background color as a `Color` value
    ///
    /// # Behavior
    ///
    /// - If the new color is different from the current background color:
    ///   - All dirty regions are cleared
    ///   - The entire screen is marked as dirty for redraw
    ///   - The cursor is marked for redraw
    /// - The change does not trigger an immediate screen redraw
    /// - The new background color applies to all future character writes
    /// - Existing characters retain their original background colors until redrawn
    ///
    /// # Performance Considerations
    ///
    /// Changing the background color is more expensive than changing the
    /// foreground color because it marks the entire screen as dirty,
    /// requiring a complete redraw during the next screen update.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::output::console::CONSOLE;
    /// use kernel::graphics::color;
    /// 
    /// let mut console = CONSOLE.lock();
    /// console.set_bg_color(color::BLUE);
    /// console.write_string("Text on blue background\n");
    /// console.set_bg_color(color::BLACK);
    /// console.write_string("Text on black background\n");
    /// ```
    pub fn set_bg_color(&mut self, color: Color) {
        if self.current_bg_color != color {
            self.current_bg_color = color;
            // When background color changes, the entire screen may need to be redrawn
            // Instead of directly calling draw_buffer_to_screen, mark the entire screen as dirty
            self.dirty_regions.clear();
            self.add_dirty_region(Rect::new(0, 0, self.width_chars, self.height_chars));
            self.cursor_needs_redraw = true;
        }
    }
}

/// Implementation of the [`Write`] trait for formatted output.
///
/// This implementation enables the console to be used with Rust's formatting macros
/// (`write!`, `writeln!`, etc.) and integrates with the standard formatting system.
///
/// # Examples
///
/// ```
/// use core::fmt::Write;
/// use kernel::output::console::CONSOLE;
///
/// let mut console = CONSOLE.lock();
/// write!(console, "Formatted: {}, {}", 42, "hello").unwrap();
/// writeln!(console, "Line with formatting").unwrap();
/// ```
///
/// # Returns
///
/// Returns `Ok(())` on success. In practice, writing to the console should never fail,
/// but the trait requires returning a `fmt::Result`.
impl Write for Console<'_> {
    /// Writes a string slice to the console.
    ///
    /// This method delegates to [`Console::write_string`] to handle the actual
    /// character rendering and screen updates.
    ///
    /// # Arguments
    ///
    /// * `s` - The string slice to write
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. Console writing operations should not fail
    /// under normal circumstances.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Prints formatted text to the console without a newline.
///
/// This macro provides a convenient way to output formatted text to the console,
/// similar to Rust's standard `print!` macro. It supports all standard formatting
/// specifiers and automatically handles string formatting and console output.
///
/// # Arguments
///
/// * `$($arg:tt)*` - Format string and arguments, following Rust's formatting syntax
///
/// # Examples
///
/// ```
/// use kernel::print;
///
/// // Print simple text
/// print!("Hello, ");
/// print!("World!");
///
/// // Print formatted values
/// print!("The answer is {}", 42);
/// print!("Debug: {:?}", vec![1, 2, 3]);
/// print!("Hex: 0x{:x}", 255);
///
/// // Print with ANSI color codes
/// print!("\x1b[31mRed text\x1b[0m");
/// print!("\x1b[32;44mGreen on blue\x1b[0m");
/// ```
///
/// # Notes
///
/// - This macro expands to a call to `_print` with the formatted arguments
/// - Output appears immediately without buffering
/// - The cursor position is updated after printing
/// - ANSI escape codes are supported for colors and formatting
/// - This macro is exported at the crate root for easy access
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::output::console::_print(format_args!($($arg)*)));
}

/// Prints formatted text to the console with a trailing newline.
///
/// This macro provides a convenient way to output formatted text to the console
/// followed by a newline, similar to Rust's standard `println!` macro. It supports
/// all standard formatting specifiers and automatically handles string formatting
/// and console output.
///
/// # Arguments
///
/// * `$($arg:tt)*` - Format string and arguments, following Rust's formatting syntax
///
/// # Behavior
///
/// - Outputs the formatted text followed by a newline character (`\n`)
/// - The cursor moves to the beginning of the next line after printing
/// - ANSI escape codes are supported for colors and formatting
/// - The macro expands to a call to `print!` with an added newline
///
/// # Examples
///
/// ```
/// use kernel::println;
///
/// // Print simple text with newline
/// println!("Hello, World!");
///
/// // Print formatted values with newline
/// println!("The answer is {}", 42);
/// println!("Debug: {:?}", vec![1, 2, 3]);
/// println!("Hex: 0x{:x}", 255);
///
/// // Print with ANSI color codes and newline
/// println!("\x1b[31mRed text\x1b[0m");
/// println!("\x1b[32;44mGreen on blue\x1b[0m");
///
/// // Empty println! prints just a newline
/// println!();
/// ```
///
/// # Notes
///
/// - This macro is exported at the crate root for easy access
/// - Output appears immediately without buffering
/// - The newline character triggers automatic scrolling when needed
/// - This macro delegates to the `print!` macro internally
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Internal function that handles formatted printing to the console.
///
/// This function is the backend for both the `print!` and `println!` macros.
/// It acquires a lock on the global console instance, formats the arguments,
/// and writes the formatted text to the console.
///
/// # Arguments
///
/// * `args` - Format arguments created by the `format_args!` macro
///
/// # Behavior
///
/// - Acquires a mutex lock on the global `CONSOLE` instance
/// - Formats the arguments using Rust's standard formatting system
/// - Writes the formatted text to the console via the `Write` trait
/// - Panics if writing to the console fails (should not happen under normal circumstances)
///
/// # Safety
///
/// This function is marked as `#[doc(hidden)]` because it's an implementation detail
/// and should not be called directly. Use the `print!` and `println!` macros instead.
///
/// # Panics
///
/// This function panics if:
/// - The console mutex cannot be acquired (should not happen in a single-threaded kernel)
/// - Writing to the console fails (e.g., if the console is not properly initialized)
///
/// # Examples
///
/// ```ignore
/// use kernel::output::console::_print;
/// use core::fmt::Arguments;
///
/// // This is how the print! macro uses _print internally
/// _print(format_args!("Hello, {}", "World"));
/// ```
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    CONSOLE
        .lock()
        .write_fmt(args)
        .expect("Failed to write to console");
}
