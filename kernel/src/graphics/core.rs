//! Core graphics rendering engine with double buffering support
//!
//! This module provides the main rendering engine for the Proka kernel graphics system.
//! It implements a double-buffered renderer that draws to a back buffer and presents
//! to the front buffer (framebuffer) to prevent screen tearing and artifacts.
//!
//! # Key Features
//!
//! - **Double buffering**: All drawing operations are performed on a back buffer,
//!   then presented to the screen with `present()` method
//! - **Pixel-level operations**: Direct pixel manipulation with alpha blending support
//! - **Drawing primitives**: Lines, rectangles, circles, polygons, triangles
//! - **BMP image support**: Loading and drawing BMP images with scaling and distortion
//! - **Alpha blending**: Support for transparent colors and alpha compositing
//! - **Scanline polygon filling**: Efficient polygon filling using scanline algorithms
//!
//! # Usage
//!
//! ```no_run
//! use crate::graphics::core::{Renderer, Pixel, pixel};
//! use crate::graphics::color::{Color, RED, BLUE, GREEN};
//!
//! // Initialize renderer with framebuffer
//! let mut renderer = Renderer::new(framebuffer);
//!
//! // Set clear color and clear screen
//! renderer.set_clear_color(Color::from_hex(0x1a1a2e));
//! renderer.clear();
//!
//! // Draw shapes
//! renderer.draw_line(pixel!(10, 10), pixel!(100, 100), RED);
//! renderer.fill_rect(pixel!(50, 50), 80, 60, BLUE);
//! renderer.draw_circle(pixel!(200, 150), 40, GREEN);
//!
//! // Present to screen
//! renderer.present();
//! ```
//!
//! # Coordinate System
//!
//! The coordinate system uses (0, 0) as the top-left corner of the screen,
//! with x increasing to the right and y increasing downward.

extern crate alloc;
use crate::graphics::color;
use crate::libs::bmp::{BmpError, BmpImage};
use alloc::{vec, vec::Vec};
use core::slice;
use limine::framebuffer::Framebuffer;

/// Represents a pixel coordinate in the framebuffer
///
/// The `Pixel` struct stores x and y coordinates as 64-bit unsigned integers.
/// It provides methods for coordinate manipulation and implements the `PixelCoord` trait.
///
/// # Examples
///
/// ```
/// use crate::graphics::core::Pixel;
///
/// let pixel = Pixel::new(100, 200);
/// assert_eq!(pixel.x, 100);
/// assert_eq!(pixel.y, 200);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixel {
    /// The x-coordinate (horizontal position)
    pub x: u64,
    /// The y-coordinate (vertical position)
    pub y: u64,
}

impl Pixel {
    /// Creates a new pixel with the given coordinates
    ///
    /// # Arguments
    ///
    /// * `x` - The x-coordinate (horizontal position)
    /// * `y` - The y-coordinate (vertical position)
    ///
    /// # Returns
    ///
    /// A new `Pixel` instance
    pub fn new(x: u64, y: u64) -> Self {
        Self { x, y }
    }
}

/// Trait for types that can be converted to pixel coordinates
///
/// This trait provides a common interface for converting various types
/// to pixel coordinates in the form of `(x, y)` tuples.
pub trait PixelCoord {
    /// Converts the implementing type to pixel coordinates
    ///
    /// # Returns
    ///
    /// A tuple `(x, y)` representing the pixel coordinates
    fn to_coord(&self) -> (u64, u64);
}

impl PixelCoord for Pixel {
    /// Converts a `Pixel` to its coordinate tuple
    ///
    /// # Returns
    ///
    /// A tuple `(x, y)` containing the pixel's coordinates
    fn to_coord(&self) -> (u64, u64) {
        (self.x, self.y)
    }
}

/// Macro for creating `Pixel` instances with concise syntax
///
/// This macro creates a `Pixel` instance from x and y coordinates,
/// automatically converting them to `u64`.
///
/// # Examples
///
/// ```
/// use crate::graphics::core::pixel;
///
/// let p1 = pixel!(10, 20);      // Creates Pixel { x: 10, y: 20 }
/// let p2 = pixel!(100.5, 200);  // Creates Pixel { x: 100, y: 200 } (truncates float)
/// ```
#[macro_export]
macro_rules! pixel {
    ($x:expr, $y:expr) => {{
        Pixel::new(($x) as u64, ($y) as u64)
    }};
}

/// Double-buffered graphics renderer
///
/// The `Renderer` struct manages a framebuffer with double buffering to prevent
/// screen tearing. All drawing operations are performed on a back buffer, and
/// the `present()` method copies the back buffer to the front buffer (framebuffer).
///
/// # Lifetime
///
/// The renderer borrows a `Framebuffer` for its lifetime, ensuring the framebuffer
/// outlives the renderer.
///
/// # Fields
///
/// * `framebuffer` - The front buffer (framebuffer) provided by the bootloader
/// * `back_buffer` - The back buffer where all drawing operations are performed
/// * `pixel_size` - Number of bytes per pixel (depends on framebuffer BPP)
/// * `clear_color` - Default color used when clearing the screen
pub struct Renderer<'a> {
    /// The front buffer (framebuffer) provided by the bootloader
    framebuffer: Framebuffer<'a>,
    /// The back buffer where all drawing operations are performed
    back_buffer: Vec<u8>,
    /// Number of bytes per pixel (depends on framebuffer BPP)
    pixel_size: usize,
    /// Default color used when clearing the screen
    clear_color: color::Color,
}

impl<'a> Renderer<'a> {
    /// Creates a new renderer with the given framebuffer
    ///
    /// Initializes a back buffer with the same dimensions as the framebuffer
    /// and sets the default clear color to black.
    ///
    /// # Arguments
    ///
    /// * `framebuffer` - The framebuffer to render to
    ///
    /// # Returns
    ///
    /// A new `Renderer` instance
    ///
    /// # Panics
    ///
    /// Panics if the framebuffer's bits per pixel (BPP) is not 24 or 32
    pub fn new(framebuffer: Framebuffer<'a>) -> Self {
        let width = framebuffer.width() as usize;
        let height = framebuffer.height() as usize;
        let bpp = framebuffer.bpp() as usize; // bits per pixel
        let pixel_size = bpp / 8; // bytes per pixel
        let buffer_size = width * height * pixel_size; // Total bytes in back buffer

        // Initialize back buffer with zeros (black)
        let back_buffer = vec![0; buffer_size];
        Self {
            framebuffer: framebuffer,
            back_buffer,
            pixel_size,
            clear_color: color::BLACK,
        }
    }

    /// Calculates the byte offset in the back buffer for a given pixel coordinate
    ///
    /// The back buffer uses a linear layout (no pitch), unlike the framebuffer
    /// which may have padding at the end of each scanline.
    ///
    /// # Arguments
    ///
    /// * `x` - The x-coordinate of the pixel
    /// * `y` - The y-coordinate of the pixel
    ///
    /// # Returns
    ///
    /// The byte offset in the back buffer for the specified pixel
    #[inline(always)]
    fn get_buffer_offset(&self, x: u64, y: u64) -> usize {
        // Back buffer layout is linear, not necessarily matching framebuffer pitch
        y as usize * self.framebuffer.width() as usize * self.pixel_size
            + x as usize * self.pixel_size
    }

    /// Converts a color to the framebuffer's pixel format
    ///
    /// This method handles different framebuffer formats (24-bit and 32-bit)
    /// by applying the appropriate color masks.
    ///
    /// # Arguments
    ///
    /// * `color` - The color to convert
    ///
    /// # Returns
    ///
    /// The color encoded in the framebuffer's pixel format
    ///
    /// # Panics
    ///
    /// Panics if the framebuffer's bits per pixel (BPP) is not 24 or 32
    #[inline(always)]
    fn mask_color(&self, color: &color::Color) -> u32 {
        if self.framebuffer.bpp() == 32 {
            let value: u32 = ((color.r as u32) << self.framebuffer.red_mask_shift())
                | ((color.g as u32) << self.framebuffer.green_mask_shift())
                | ((color.b as u32) << self.framebuffer.blue_mask_shift());
            return value;
        } else if self.framebuffer.bpp() == 24 {
            color.to_u32(false)
        } else {
            panic!("Unsupported bit per pixel: {}", self.framebuffer.bpp())
        }
    }

    /// Draws a pixel to the back buffer at the specified coordinates
    ///
    /// This method performs bounds checking and supports alpha blending
    /// for transparent colors.
    ///
    /// # Arguments
    ///
    /// * `x` - The x-coordinate of the pixel
    /// * `y` - The y-coordinate of the pixel
    /// * `color` - The color to draw
    #[inline(always)]
    fn set_pixel_raw(&mut self, x: u64, y: u64, color: &color::Color) {
        // Bounds check: ensure pixel is within screen bounds
        if x < self.framebuffer.width() && y < self.framebuffer.height() {
            let offset = self.get_buffer_offset(x, y);
            let color_u32 = if color.a == 255 {
                self.mask_color(color)
            } else if color.a > 0 {
                // Read current pixel color from back buffer for alpha blending
                let current_color = self.get_pixel_raw(x, y);

                // Perform alpha blending: result = (source * alpha + destination * (255 - alpha)) / 255
                let alpha = color.a as u32;
                let inv_alpha = 255 - alpha;
                let r = (color.r as u32 * alpha + current_color.r as u32 * inv_alpha) / 255;
                let g = (color.g as u32 * alpha + current_color.g as u32 * inv_alpha) / 255;
                let b = (color.b as u32 * alpha + current_color.b as u32 * inv_alpha) / 255;

                let mixed_color = color::Color::with_alpha(r as u8, g as u8, b as u8, 255);
                self.mask_color(&mixed_color)
            } else {
                // Fully transparent, don't draw
                return;
            };

            let pixel_bytes = color_u32.to_le_bytes(); // Convert to byte array
            let bytes_to_write = &pixel_bytes[..self.pixel_size]; // Take bytes for BPP
            for i in 0..self.pixel_size {
                self.back_buffer[offset + i] = bytes_to_write[i];
            }
        }
    }

    /// Sets a pixel at the specified `Pixel` coordinate
    ///
    /// # Arguments
    ///
    /// * `pixel` - The pixel coordinate
    /// * `color` - The color to set
    #[inline(always)]
    pub fn set_pixel(&mut self, pixel: Pixel, color: &color::Color) {
        let (x, y) = pixel.to_coord();
        self.set_pixel_raw(x, y, color);
    }

    /// Gets the color of a pixel at the specified coordinate
    ///
    /// This method reads from the back buffer, which contains the most recent
    /// drawing operations (not yet presented to the screen).
    ///
    /// # Arguments
    ///
    /// * `pixel` - The pixel coordinate to read
    ///
    /// # Returns
    ///
    /// The color of the pixel at the specified coordinate
    pub fn get_pixel(&self, pixel: Pixel) -> color::Color {
        let (x, y) = pixel.to_coord();
        self.get_pixel_raw(x, y) // Read from back buffer
    }

    /// Gets the color of a pixel at raw coordinates (internal method)
    ///
    /// # Arguments
    ///
    /// * `x` - The x-coordinate
    /// * `y` - The y-coordinate
    ///
    /// # Returns
    ///
    /// The color of the pixel at the specified coordinates
    fn get_pixel_raw(&self, x: u64, y: u64) -> color::Color {
        let offset = self.get_buffer_offset(x, y);
        let mut pixel_data_u32 = 0u32;
        for i in 0..self.pixel_size {
            pixel_data_u32 |= (self.back_buffer[offset + i] as u32) << (i * 8);
        }
        color::Color::from_u32(pixel_data_u32)
    }

    /// Sets the clear color used by the `clear()` method
    ///
    /// # Arguments
    ///
    /// * `color` - The new clear color
    pub fn set_clear_color(&mut self, color: color::Color) {
        self.clear_color = color;
    }

    /// Gets the current clear color
    ///
    /// # Returns
    ///
    /// The current clear color
    pub fn get_clear_color(&self) -> color::Color {
        self.clear_color
    }

    /// Clears the back buffer with the current clear color
    ///
    /// This method fills the entire back buffer with the clear color,
    /// effectively erasing all previous drawing operations.
    pub fn clear(&mut self) {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        let color = self.clear_color.clone();
        // Optimized clear operation: directly fill back buffer
        let masked_clear_color = self.mask_color(&color);
        let pixel_bytes = masked_clear_color.to_le_bytes(); // Convert to byte array
        let bytes_to_fill = &pixel_bytes[..self.pixel_size];
        for y in 0..height {
            for x in 0..width {
                let offset = self.get_buffer_offset(x, y);
                for i in 0..self.pixel_size {
                    self.back_buffer[offset + i] = bytes_to_fill[i];
                }
            }
        }
    }

    /// Draws a line between two points using Bresenham's algorithm
    ///
    /// This method implements Bresenham's line algorithm with support for
    /// steep lines (where |dy| > |dx|) and handles all octants.
    ///
    /// # Arguments
    ///
    /// * `p1` - The starting point of the line
    /// * `p2` - The ending point of the line
    /// * `color` - The color of the line
    pub fn draw_line(&mut self, p1: Pixel, p2: Pixel, color: color::Color) {
        let dx_abs = ((p2.x as i64 - p1.x as i64).abs()) as u64;
        let dy_abs = ((p2.y as i64 - p1.y as i64).abs()) as u64;
        let steep = dy_abs > dx_abs;
        let (mut x1, mut y1) = p1.to_coord();
        let (mut x2, mut y2) = p2.to_coord();
        if steep {
            core::mem::swap(&mut x1, &mut y1);
            core::mem::swap(&mut x2, &mut y2);
        }
        if x1 > x2 {
            core::mem::swap(&mut x1, &mut x2);
            core::mem::swap(&mut y1, &mut y2);
        }
        let dx = x2 - x1;
        let dy = (y2 as i64 - y1 as i64).abs() as u64;
        let mut error = (dx / 2) as i64;
        let y_step = if y1 < y2 { 1 } else { -1 };
        let mut y = y1 as i64;
        for x in x1..=x2 {
            if steep {
                // 确保 y, x 坐标在帧缓冲区范围内
                if y >= 0 && (y as u64) < self.framebuffer.width() && x < self.framebuffer.height()
                {
                    self.set_pixel_raw(y as u64, x, &color);
                }
            } else {
                if x < self.framebuffer.width() && y >= 0 && (y as u64) < self.framebuffer.height()
                {
                    self.set_pixel_raw(x, y as u64, &color);
                }
            }
            error -= dy as i64;
            if error < 0 {
                y += y_step;
                error += dx as i64;
            }
        }
    }

    /// Draws the outline of a triangle
    ///
    /// This method draws three lines connecting the three points to form
    /// a triangle outline.
    ///
    /// # Arguments
    ///
    /// * `p1` - First vertex of the triangle
    /// * `p2` - Second vertex of the triangle
    /// * `p3` - Third vertex of the triangle
    /// * `color` - The color of the triangle outline
    pub fn draw_triangle(&mut self, p1: Pixel, p2: Pixel, p3: Pixel, color: color::Color) {
        self.draw_line(p1, p2, color);
        self.draw_line(p2, p3, color);
        self.draw_line(p3, p1, color);
    }

    /// Fills a triangle with the specified color
    ///
    /// This method implements a scanline triangle filling algorithm that
    /// handles both the top and bottom halves of the triangle separately.
    ///
    /// # Arguments
    ///
    /// * `p1` - First vertex of the triangle
    /// * `p2` - Second vertex of the triangle
    /// * `p3` - Third vertex of the triangle
    /// * `color` - The fill color
    pub fn fill_triangle(&mut self, p1: Pixel, p2: Pixel, p3: Pixel, color: color::Color) {
        let (x1, y1) = p1.to_coord();
        let (x2, y2) = p2.to_coord();
        let (x3, y3) = p3.to_coord();
        // 定义3个变换后的 Pixel
        let mut pts = [pixel!(x1, y1), pixel!(x2, y2), pixel!(x3, y3)];
        // 按 y 轻量排序：冒泡排序也可以
        for i in 0..pts.len() {
            for j in i + 1..pts.len() {
                if pts[i].y > pts[j].y {
                    pts.swap(i, j);
                }
            }
        }
        let p1 = pts[0];
        let p2 = pts[1];
        let p3 = pts[2];
        // 如果三点 y 相同，不画
        if p1.y == p3.y {
            return;
        }
        // 获取 u32 坐标
        let (x1, y1) = (p1.x as i32, p1.y as i32);
        let (x2, y2) = (p2.x as i32, p2.y as i32);
        let (x3, y3) = (p3.x as i32, p3.y as i32);
        // 水平线闭包填充函数
        let mut fill_h_line = |start_x: i32, end_x: i32, y: i32| {
            if y < 0 || y >= self.framebuffer.height() as i32 {
                return;
            }
            let mut start_x = start_x.max(0);
            let mut end_x = end_x.min(self.framebuffer.width() as i32 - 1);
            if start_x > end_x {
                core::mem::swap(&mut start_x, &mut end_x);
            }
            if start_x < 0 || end_x >= self.framebuffer.width() as i32 {
                start_x = start_x.max(0);
                end_x = end_x.min(self.framebuffer.width() as i32 - 1);
                if start_x > end_x {
                    return;
                }
            }
            // 填充到后台缓冲区
            for x in start_x..=end_x {
                if x >= 0 {
                    let pixel = pixel!(x, y);
                    self.set_pixel(pixel, &color);
                }
            }
        };
        let long_dx = x3 - x1;
        let long_dy = y3 - y1;
        if long_dy != 0 {
            // 上半部分三角形（p1 -> p2）
            let upper_dx = x2 - x1;
            let upper_dy = y2 - y1;
            let y_start = y1;
            let y_end = y2;
            for y in y_start..=y_end {
                let dy = y - y1;
                let x_long = if long_dy != 0 {
                    x1 + (long_dx * dy + long_dy / 2) / long_dy
                } else {
                    x1
                };
                let x_upper = if upper_dy != 0 {
                    x1 + (upper_dx * dy + upper_dy / 2) / upper_dy
                } else {
                    x1
                };
                fill_h_line(x_long, x_upper, y);
            }
            // 下半部分三角形（p2 -> p3）
            let lower_dx = x3 - x2;
            let lower_dy = y3 - y2;
            if lower_dy != 0 {
                for y in y2..=y3 {
                    let dy_long = y - y1;
                    let dy_lower = y - y2;
                    let x_long = if long_dy != 0 {
                        x1 + (long_dx * dy_long + long_dy / 2) / long_dy
                    } else {
                        x1
                    };
                    let x_lower = if lower_dy != 0 {
                        x2 + (lower_dx * dy_lower + lower_dy / 2) / lower_dy
                    } else {
                        x2
                    };
                    fill_h_line(x_long, x_lower, y);
                }
            }
        }
    }

    /// Gets the width of the framebuffer
    ///
    /// # Returns
    ///
    /// The width of the framebuffer in pixels
    pub fn width(&self) -> u64 {
        self.framebuffer.width()
    }

    /// Gets the height of the framebuffer
    ///
    /// # Returns
    ///
    /// The height of the framebuffer in pixels
    pub fn height(&self) -> u64 {
        self.framebuffer.height()
    }

    /// Draws the outline of a rectangle
    ///
    /// # Arguments
    ///
    /// * `pixel` - The top-left corner of the rectangle
    /// * `width` - The width of the rectangle
    /// * `height` - The height of the rectangle
    /// * `color` - The color of the rectangle outline
    pub fn draw_rect(&mut self, pixel: Pixel, width: u64, height: u64, color: color::Color) -> () {
        let (x, y) = pixel.to_coord();
        let x2 = x + width;
        let y2 = y + height;
        // Draw to back buffer
        self.draw_line(pixel!(x, y), pixel!(x2, y), color);
        self.draw_line(pixel!(x2, y), pixel!(x2, y2), color);
        self.draw_line(pixel!(x2, y2), pixel!(x, y2), color);
        self.draw_line(pixel!(x, y2), pixel!(x, y), color);
    }

    /// Fills a rectangle with the specified color
    ///
    /// # Arguments
    ///
    /// * `pixel` - The top-left corner of the rectangle
    /// * `width` - The width of the rectangle
    /// * `height` - The height of the rectangle
    /// * `color` - The fill color
    pub fn fill_rect(&mut self, pixel: Pixel, width: u64, height: u64, color: color::Color) {
        let (x_min, y_min) = pixel.to_coord();
        let x_max = x_min + width;
        let y_max = y_min + height;
        let x_start = x_min.max(0);
        let x_end = x_max.min(self.width() - 1);
        let y_start = y_min.max(0);
        let y_end = y_max.min(self.height() - 1);
        for y in y_start..=y_end {
            for x in x_start..=x_end {
                self.set_pixel_raw(x, y, &color); // Draw to back buffer
            }
        }
    }

    /// Draws the outline of a polygon
    ///
    /// This method connects all points in sequence to form a closed polygon.
    /// The last point is automatically connected back to the first point.
    ///
    /// # Arguments
    ///
    /// * `points` - A slice of `Pixel` points defining the polygon vertices
    /// * `color` - The color of the polygon outline
    ///
    /// # Note
    ///
    /// If there are fewer than 3 points, the method returns without drawing anything.
    pub fn draw_polygon(&mut self, points: &[Pixel], color: color::Color) {
        if points.len() < 3 {
            return; // Cannot form a polygon with fewer than 3 points
        }
        // Connect all points to form a closed polygon
        for i in 0..points.len() {
            let p1 = points[i];
            let p2 = points[(i + 1) % points.len()]; // Last point connects back to first
            self.draw_line(p1, p2, color);
        }
    }
    /// Fills a convex polygon using a scanline algorithm
    ///
    /// This method implements a scanline polygon filling algorithm that works
    /// specifically for convex polygons. It calculates intersections with each
    /// scanline and fills between pairs of intersections.
    ///
    /// # Arguments
    ///
    /// * `points` - A slice of `Pixel` points defining the convex polygon vertices
    /// * `color` - The fill color
    ///
    /// # Note
    ///
    /// This method is optimized for convex polygons. For concave polygons,
    /// use `fill_polygon()` which implements the even-odd rule.
    pub fn fill_convex_polygon(&mut self, points: &[Pixel], color: color::Color) {
        if points.len() < 3 {
            return; // Cannot form a polygon with fewer than 3 points
        }
        // Collect information about all edges
        let mut edges = Vec::new();
        for i in 0..points.len() {
            let p1 = points[i];
            let p2 = points[(i + 1) % points.len()];
            edges.push((p1, p2));
        }
        // Find the y-range of the polygon
        let min_y = edges.iter().map(|&(p, _)| p.y).min().unwrap_or(0);
        let max_y = edges.iter().map(|&(p, _)| p.y).max().unwrap_or(0);
        // Calculate x-increment information for each edge
        let mut edge_info: Vec<(f64, f64, f64, f64)> = Vec::new();
        for &(p1, p2) in &edges {
            if p1.y != p2.y {
                let y_start = p1.y.min(p2.y) as f64;
                let y_end = p1.y.max(p2.y) as f64;
                let x_start = if p1.y < p2.y {
                    p1.x as f64
                } else {
                    p2.x as f64
                };
                let dx = (p2.x as f64 - p1.x as f64) / (p2.y as f64 - p1.y as f64);
                edge_info.push((y_start, y_end, x_start, dx));
            }
        }
        // Scanline filling
        for y in min_y..=max_y {
            let mut intersections = Vec::new();

            // Calculate intersections of current scanline y with all edges
            for &(y_start, y_end, x_start, dx) in &edge_info {
                if (y as f64) >= y_start && (y as f64) <= y_end {
                    let x = x_start + (y as f64 - y_start) * dx;
                    intersections.push(x);
                }
            }
            // Sort intersections
            intersections.sort_by(|a, b| a.partial_cmp(b).expect("Float comparison failed"));
            // Fill between pairs of intersections
            for i in (0..intersections.len()).step_by(2) {
                if i + 1 >= intersections.len() {
                    break;
                }

                let start_x = intersections[i].max(0.0).min(self.width() as f64 - 1.0) as u64;
                let end_x = intersections[i + 1].max(0.0).min(self.width() as f64 - 1.0) as u64;

                if start_x > end_x {
                    continue;
                }

                for x in start_x..=end_x {
                    self.set_pixel_raw(x, y, &color);
                }
            }
        }
    }
    /// Fills any polygon (convex or concave) using the even-odd rule
    ///
    /// This method implements a scanline polygon filling algorithm that works
    /// for both convex and concave polygons using the even-odd rule to determine
    /// which pixels are inside the polygon.
    ///
    /// # Arguments
    ///
    /// * `points` - A slice of `Pixel` points defining the polygon vertices
    /// * `color` - The fill color
    ///
    /// # Algorithm
    ///
    /// The even-odd rule determines whether a point is inside a polygon by
    /// drawing a ray from the point to infinity and counting how many times
    /// it crosses the polygon boundary. If the count is odd, the point is inside.
    pub fn fill_polygon(&mut self, points: &[Pixel], color: color::Color) {
        if points.len() < 3 {
            return;
        }
        // Find the y-range of the polygon
        let min_y = points.iter().map(|p| p.y).min().unwrap_or(0);
        let max_y = points.iter().map(|p| p.y).max().unwrap_or(0);
        // Collect information about all edges
        let mut edge_table = Vec::new();
        for i in 0..points.len() {
            let p1 = points[i];
            let p2 = points[(i + 1) % points.len()];

            if p1.y != p2.y {
                let (start, end) = if p1.y < p2.y { (p1, p2) } else { (p2, p1) };
                let dx = (end.x as f64 - start.x as f64) / (end.y as f64 - start.y as f64);
                edge_table.push((start.y as f64, end.y as f64, start.x as f64, dx));
            }
        }
        // Scanline filling
        for y in min_y..=max_y {
            let mut intersections = Vec::new();

            // Check if each edge intersects with the current scanline
            for &(y_min, y_max, mut x, dx) in &edge_table {
                if (y as f64) >= y_min && (y as f64) < y_max {
                    if y as f64 > y_min {
                        x += (y as f64 - y_min) * dx;
                    }
                    intersections.push(x);
                }
            }
            // Sort intersections
            intersections.sort_by(|a, b| a.partial_cmp(b).expect("Float comparison failed"));
            // Fill between intersections using even-odd rule
            let mut inside = false;
            for i in 0..intersections.len() {
                if inside && i < intersections.len() {
                    let start_x = intersections[i].max(0.0).min(self.width() as f64 - 1.0) as u64;

                    // Ensure we don't access out of bounds
                    if i + 1 < intersections.len() {
                        let end_x =
                            intersections[i + 1].max(0.0).min(self.width() as f64 - 1.0) as u64;

                        if start_x <= end_x {
                            for x in start_x..=end_x {
                                self.set_pixel_raw(x, y, &color);
                            }
                        }
                    } else {
                        // Handle the last point
                        let end_x = self.width().min(self.width() - 1);
                        if start_x <= end_x {
                            for x in start_x..=end_x {
                                self.set_pixel_raw(x, y, &color);
                            }
                        }
                    }
                }
                inside = !inside;
            }
        }
    }

    /// Draws a BMP image at the specified position
    ///
    /// This method draws the BMP image pixel by pixel at the given position.
    /// The image is drawn at its original size without scaling.
    ///
    /// # Arguments
    ///
    /// * `pos` - The top-left position where to draw the image
    /// * `bmp` - The BMP image to draw
    pub fn draw_bmp(&mut self, pos: Pixel, bmp: &BmpImage) {
        let (x_start, y_start) = (pos.x, pos.y);

        for y in 0..bmp.height() {
            for x in 0..bmp.width() {
                if let Some(color) = bmp.pixel(x, y) {
                    self.set_pixel_raw(x_start + x as u64, y_start + y as u64, &color);
                }
            }
        }
    }
    /// Draws a BMP image with scaling
    ///
    /// This method draws the BMP image scaled by the specified factors.
    /// Nearest-neighbor interpolation is used for scaling.
    ///
    /// # Arguments
    ///
    /// * `pos` - The top-left position where to draw the image
    /// * `bmp` - The BMP image to draw
    /// * `scale_x` - Horizontal scaling factor (1.0 = original size)
    /// * `scale_y` - Vertical scaling factor (1.0 = original size)
    pub fn draw_bmp_scaled(&mut self, pos: Pixel, bmp: &BmpImage, scale_x: f32, scale_y: f32) {
        let scaled_width = (bmp.width() as f32 * scale_x) as u64;
        let scaled_height = (bmp.height() as f32 * scale_y) as u64;
        let (x_start, y_start) = pos.to_coord();

        for y in 0..scaled_height {
            for x in 0..scaled_width {
                // Calculate source coordinates using nearest-neighbor interpolation
                let src_x = (x as f32 / scale_x) as u32;
                let src_y = (y as f32 / scale_y) as u32;

                if src_x < bmp.width() && src_y < bmp.height() {
                    if let Some(color) = bmp.pixel(src_x, src_y) {
                        self.set_pixel_raw(x_start + x, y_start + y, &color);
                    }
                }
            }
        }
    }
    /// Draws a BMP image with distortion to fit a quadrilateral
    ///
    /// This method draws a BMP image distorted to fit the specified quadrilateral
    /// defined by four corner points. It uses bilinear interpolation to map
    /// source image coordinates to destination coordinates.
    ///
    /// # Arguments
    ///
    /// * `corners` - An array of four `Pixel` points defining the quadrilateral corners
    ///   in clockwise or counter-clockwise order
    /// * `bmp` - The BMP image to draw
    ///
    /// # Algorithm
    ///
    /// The method calculates the bounding box of the quadrilateral and uses
    /// simplified bilinear interpolation to map each destination pixel to
    /// a source pixel in the BMP image.
    pub fn draw_bmp_distorted(&mut self, corners: [Pixel; 4], bmp: &BmpImage) {
        // Calculate bounding box
        let min_x = corners.iter().map(|p| p.x).min().unwrap_or(0);
        let max_x = corners.iter().map(|p| p.x).max().unwrap_or(0);
        let min_y = corners.iter().map(|p| p.y).min().unwrap_or(0);
        let max_y = corners.iter().map(|p| p.y).max().unwrap_or(0);

        // Calculate transformation matrix (simplified bilinear interpolation)
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Calculate relative position (simplified version, actual implementation
                // should use more precise texture mapping)
                let u = (x - min_x) as f32 / (max_x - min_x) as f32;
                let v = (y - min_y) as f32 / (max_y - min_y) as f32;

                let src_x = (u * bmp.width() as f32) as u32;
                let src_y = (v * bmp.height() as f32) as u32;

                if let Some(color) = bmp.pixel(src_x, src_y) {
                    self.set_pixel_raw(x, y, &color);
                }
            }
        }
    }
    /// Loads and draws a BMP image from raw byte data
    ///
    /// This method parses raw BMP image data from a byte slice and draws it
    /// at the specified position. It handles BMP parsing errors and returns
    /// them to the caller.
    ///
    /// # Arguments
    ///
    /// * `pos` - The top-left position where to draw the image
    /// * `data` - Raw byte data containing the BMP image
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the BMP image was successfully parsed and drawn
    /// * `Err(BmpError)` - If there was an error parsing the BMP data
    ///
    /// # Errors
    ///
    /// This method can return various `BmpError` variants if the BMP data
    /// is malformed or unsupported.
    pub fn draw_bmp_from_bytes(&mut self, pos: Pixel, data: &[u8]) -> Result<(), BmpError> {
        let bmp = BmpImage::from_bytes(data)?;
        self.draw_bmp(pos, &bmp);
        Ok(())
    }

    /// Draws a circle with the specified center, radius, and color
    ///
    /// This method implements Bresenham's circle algorithm to draw a circle
    /// outline efficiently using only integer arithmetic. The algorithm draws
    /// 8 symmetric points for each calculated point to complete the circle.
    ///
    /// # Arguments
    ///
    /// * `center` - The center point of the circle
    /// * `radius` - The radius of the circle in pixels
    /// * `color` - The color of the circle outline
    ///
    /// # Algorithm
    ///
    /// The algorithm uses Bresenham's circle drawing algorithm which works by:
    /// 1. Starting at point (0, radius)
    /// 2. Calculating the decision parameter `d`
    /// 3. Iteratively determining the next pixel position
    /// 4. Drawing 8 symmetric points for each calculated position
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::graphics::core::{Renderer, pixel};
    /// use crate::graphics::color::RED;
    ///
    /// // Assuming renderer is initialized
    /// # let framebuffer = unsafe { core::mem::zeroed() };
    /// # let mut renderer = Renderer::new(framebuffer);
    /// renderer.draw_circle(pixel!(100, 100), 50, RED);
    /// ```
    ///
    /// # Note
    ///
    /// If radius is 0, the method returns immediately without drawing anything.
    pub fn draw_circle(&mut self, center: Pixel, radius: u64, color: color::Color) {
        if radius == 0 {
            return;
        }

        let (cx, cy) = center.to_coord();
        let mut x = 0i64;
        let mut y = radius as i64;
        let mut d = 3 - 2 * radius as i64;

        while x <= y {
            // 绘制8个对称点
            self.set_pixel_raw((cx as i64 + x) as u64, (cy as i64 + y) as u64, &color);
            self.set_pixel_raw((cx as i64 + x) as u64, (cy as i64 - y) as u64, &color);
            self.set_pixel_raw((cx as i64 - x) as u64, (cy as i64 + y) as u64, &color);
            self.set_pixel_raw((cx as i64 - x) as u64, (cy as i64 - y) as u64, &color);
            self.set_pixel_raw((cx as i64 + y) as u64, (cy as i64 + x) as u64, &color);
            self.set_pixel_raw((cx as i64 + y) as u64, (cy as i64 - x) as u64, &color);
            self.set_pixel_raw((cx as i64 - y) as u64, (cy as i64 + x) as u64, &color);
            self.set_pixel_raw((cx as i64 - y) as u64, (cy as i64 - x) as u64, &color);

            if d < 0 {
                d = d + 4 * x + 6;
            } else {
                d = d + 4 * (x - y) + 10;
                y -= 1;
            }
            x += 1;
        }
    }

    /// Presents the back buffer to the front buffer (framebuffer)
    ///
    /// This method copies the contents of the back buffer to the front buffer,
    /// making all drawing operations visible on the screen. This is the final
    /// step in the double-buffering rendering pipeline.
    ///
    /// # How It Works
    ///
    /// The method performs a row-by-row copy from the back buffer to the
    /// framebuffer, handling potential differences in pitch (bytes per row)
    /// between the two buffers. The back buffer uses a linear layout with
    /// no padding, while the framebuffer may have padding at the end of each
    /// scanline.
    ///
    /// # Safety
    ///
    /// This method uses unsafe code to directly access the framebuffer memory.
    /// It assumes:
    /// 1. The framebuffer address is valid and points to writable memory
    /// 2. The framebuffer dimensions and pitch are correctly reported by the bootloader
    /// 3. The back buffer has been properly initialized with the correct size
    ///
    /// # Performance
    ///
    /// This is a relatively expensive operation as it copies the entire
    /// framebuffer contents. For optimal performance, minimize the number of
    /// `present()` calls by batching drawing operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::graphics::core::{Renderer, pixel};
    /// use crate::graphics::color::{RED, BLUE, GREEN};
    ///
    /// // Assuming renderer is initialized
    /// # let framebuffer = unsafe { core::mem::zeroed() };
    /// # let mut renderer = Renderer::new(framebuffer);
    ///
    /// // Perform drawing operations
    /// renderer.clear();
    /// renderer.draw_line(pixel!(10, 10), pixel!(100, 100), RED);
    /// renderer.fill_rect(pixel!(50, 50), 80, 60, BLUE);
    /// renderer.draw_circle(pixel!(200, 150), 40, GREEN);
    ///
    /// // Make everything visible on screen
    /// renderer.present();
    /// ```
    ///
    /// # Note
    ///
    /// This method should be called after completing all drawing operations
    /// for a frame. Calling it multiple times per frame may cause screen tearing
    /// or reduced performance.
    pub fn present(&mut self) {
        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;
        let pitch = self.framebuffer.pitch() as usize; // Framebuffer每行的字节数
        let pixel_size = self.pixel_size; // 后台缓冲区每个像素的字节数
        unsafe {
            let front_buffer_addr = self.framebuffer.addr();
            // 逐行复制以处理可能不同的 pitch
            for y_idx in 0..height {
                let back_buffer_row_start = y_idx * width * pixel_size;
                let front_buffer_row_start = y_idx * pitch;
                // 获取后台缓冲区当前行的切片
                let source_slice = &self.back_buffer
                    [back_buffer_row_start..(back_buffer_row_start + width * pixel_size)];
                // 获取前台帧缓冲区当前行的可变切片
                let dest_ptr = front_buffer_addr.add(front_buffer_row_start);
                let dest_slice = slice::from_raw_parts_mut(dest_ptr, width * pixel_size);
                // 复制数据
                dest_slice.copy_from_slice(source_slice);
            }
        }
    }
}
