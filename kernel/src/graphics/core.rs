extern crate alloc;
use crate::graphics::color;
use crate::libs::bmp::{BmpError, BmpImage};
use alloc::{vec, vec::Vec};
use core::slice;
use limine::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixel {
    pub x: u64,
    pub y: u64,
}

impl Pixel {
    pub fn new(x: u64, y: u64) -> Self {
        Self { x, y }
    }
}

pub trait PixelCoord {
    fn to_coord(&self) -> (u64, u64);
}

impl PixelCoord for Pixel {
    fn to_coord(&self) -> (u64, u64) {
        (self.x, self.y)
    }
}

#[macro_export]
macro_rules! pixel {
    ($x:expr, $y:expr) => {{
        Pixel::new(($x) as u64, ($y) as u64)
    }};
}

pub struct Renderer<'a> {
    framebuffer: Framebuffer<'a>, // 前台缓冲区
    back_buffer: Vec<u8>,         // 后台缓冲区
    pixel_size: usize,            // 每个像素占用的字节数
    clear_color: color::Color,    // 默认清屏颜色
    dirty_min_x: u64,
    dirty_min_y: u64,
    dirty_max_x: u64,
    dirty_max_y: u64,
    width: usize,
    height: usize,
    bpp: usize,
}

impl<'a> Renderer<'a> {
    pub fn new(framebuffer: Framebuffer<'a>) -> Self {
        let width = framebuffer.width() as usize;
        let height = framebuffer.height() as usize;
        let bpp = framebuffer.bpp() as usize; // bits per pixel
        let pixel_size = bpp / 8; // bytes per pixel
        let buffer_size = width * height * pixel_size; // 后台缓冲区总字节数

        // 初始化后台缓冲区，填充为0（黑色）
        let back_buffer = vec![0; buffer_size];
        Self {
            framebuffer: framebuffer,
            back_buffer,
            pixel_size,
            clear_color: color::BLACK,
            dirty_min_x: u64::MAX,
            dirty_min_y: u64::MAX,
            dirty_max_x: 0,
            dirty_max_y: 0,
            width,
            height,
            bpp,
        }
    }

    /// 获取后台缓冲区偏移
    #[inline(always)]
    fn get_buffer_offset(&self, x: u64, y: u64) -> usize {
        // 后台缓冲区的布局是线性的，不一定与framebuffer的pitch相同
        y as usize * self.framebuffer.width() as usize * self.pixel_size
            + x as usize * self.pixel_size
    }

    /// 转换颜色为帧缓冲区格式
    #[inline(always)]
    fn mask_color(&self, color: &color::Color) -> u32 {
        if self.bpp == 32 {
            let value: u32 = ((color.r as u32) << self.framebuffer.red_mask_shift())
                | ((color.g as u32) << self.framebuffer.green_mask_shift())
                | ((color.b as u32) << self.framebuffer.blue_mask_shift());
            return value;
        } else if self.bpp == 24 {
            color.to_u32(false)
        } else {
            panic!("Unsupported bit per pixel: {}", self.framebuffer.bpp())
        }
    }

    /// 绘制像素到后台缓冲区
    #[inline(always)]
    pub unsafe fn set_pixel_raw_unchecked(&mut self, x: u64, y: u64, color: &color::Color) {
        let offset = self.get_buffer_offset(x, y);

        let color_u32 = if color.a == 255 {
            self.mask_color(color)
        } else if color.a == 0 {
            return;
        } else {
            // 读取后台缓冲区当前像素颜色进行alpha混合
            let current_color = self.get_pixel_raw(x, y);

            // 执行alpha混合: result = (source * alpha + destination * (255 - alpha)) / 255
            let alpha = color.a as u32;
            let inv_alpha = 255 - alpha;
            let r = (color.r as u32 * alpha + current_color.r as u32 * inv_alpha) / 255;
            let g = (color.g as u32 * alpha + current_color.g as u32 * inv_alpha) / 255;
            let b = (color.b as u32 * alpha + current_color.b as u32 * inv_alpha) / 255;

            let mixed_color = color::Color::with_alpha(r as u8, g as u8, b as u8, 255);
            self.mask_color(&mixed_color)
        };

        let pixel_bytes = color_u32.to_le_bytes(); // 转换为字节数组
        unsafe {
            let dst_ptr = self.back_buffer.as_mut_ptr().add(offset);
            core::ptr::copy_nonoverlapping(pixel_bytes.as_ptr(), dst_ptr, self.pixel_size);
        }

        self.dirty_min_x = self.dirty_min_x.min(x);
        self.dirty_min_y = self.dirty_min_y.min(y);
        self.dirty_max_x = self.dirty_max_x.max(x);
        self.dirty_max_y = self.dirty_max_y.max(y);
    }

    #[inline(always)]
    pub fn set_pixel_raw(&mut self, x: u64, y: u64, color: &color::Color) {
        // 边界检查：确保像素在屏幕范围内
        if x >= self.width as u64 || y >= self.height as u64 {
            return;
        }
        unsafe { self.set_pixel_raw_unchecked(x, y, color) };
    }

    /// 设置像素
    #[inline(always)]
    pub fn set_pixel(&mut self, pixel: Pixel, color: &color::Color) {
        let (x, y) = pixel.to_coord();
        self.set_pixel_raw(x, y, color);
    }

    /// 获取像素
    pub fn get_pixel(&self, pixel: Pixel) -> color::Color {
        let (x, y) = pixel.to_coord();
        self.get_pixel_raw(x, y) // 从前台缓冲区获取
    }

    /// 获取像素
    fn get_pixel_raw(&self, x: u64, y: u64) -> color::Color {
        let offset = self.get_buffer_offset(x, y);
        let mut pixel_data_u32 = 0u32;
        for i in 0..self.pixel_size {
            pixel_data_u32 |= (self.back_buffer[offset + i] as u32) << (i * 8);
        }
        color::Color::from_u32(pixel_data_u32)
    }

    pub fn set_clear_color(&mut self, color: color::Color) {
        self.clear_color = color;
    }

    pub fn get_clear_color(&self) -> color::Color {
        self.clear_color
    }

    // 清空后台缓冲区
    pub fn clear(&mut self) {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        let color = self.clear_color.clone();
        // 优化清空操作：直接填充后台缓冲区
        let masked_clear_color = self.mask_color(&color);
        let pixel_bytes = masked_clear_color.to_le_bytes(); // 转换为字节数组
        let bytes_to_fill = &pixel_bytes[..self.pixel_size];
        for y in 0..height {
            for x in 0..width {
                let offset = self.get_buffer_offset(x, y);
                for i in 0..self.pixel_size {
                    self.back_buffer[offset + i] = bytes_to_fill[i];
                }
            }
        }

        self.dirty_min_x = 0;
        self.dirty_min_y = 0;
        self.dirty_max_x = width;
        self.dirty_max_y = height;
    }

    /// 绘制线
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

    /// 绘制三角形
    pub fn draw_triangle(&mut self, p1: Pixel, p2: Pixel, p3: Pixel, color: color::Color) {
        self.draw_line(p1, p2, color);
        self.draw_line(p2, p3, color);
        self.draw_line(p3, p1, color);
    }

    /// 填充三角形
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

    pub fn width(&self) -> u64 {
        self.framebuffer.width()
    }

    pub fn height(&self) -> u64 {
        self.framebuffer.height()
    }

    /// 绘制矩形
    pub fn draw_rect(&mut self, pixel: Pixel, width: u64, height: u64, color: color::Color) -> () {
        let (x, y) = pixel.to_coord();
        let x2 = x + width;
        let y2 = y + height;
        // 绘制到后台缓冲区
        self.draw_line(pixel!(x, y), pixel!(x2, y), color);
        self.draw_line(pixel!(x2, y), pixel!(x2, y2), color);
        self.draw_line(pixel!(x2, y2), pixel!(x, y2), color);
        self.draw_line(pixel!(x, y2), pixel!(x, y), color);
    }

    /// 填充矩形
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
                self.set_pixel_raw(x, y, &color); // 绘制到后台缓冲区
            }
        }
    }

    /// 绘制任意多边形（轮廓）
    pub fn draw_polygon(&mut self, points: &[Pixel], color: color::Color) {
        if points.len() < 3 {
            return; // 少于3个点无法构成多边形
        }
        // 连接所有点形成闭合多边形
        for i in 0..points.len() {
            let p1 = points[i];
            let p2 = points[(i + 1) % points.len()]; // 最后一个点连接回第一个点
            self.draw_line(p1, p2, color);
        }
    }
    /// 填充任意凸多边形（扫描线算法）
    pub fn fill_convex_polygon(&mut self, points: &[Pixel], color: color::Color) {
        if points.len() < 3 {
            return; // 少于3个点无法构成多边形
        }
        // 收集所有边的信息
        let mut edges = Vec::new();
        for i in 0..points.len() {
            let p1 = points[i];
            let p2 = points[(i + 1) % points.len()];
            edges.push((p1, p2));
        }
        // 找到多边形的y范围
        let min_y = edges.iter().map(|&(p, _)| p.y).min().unwrap_or(0);
        let max_y = edges.iter().map(|&(p, _)| p.y).max().unwrap_or(0);
        // 计算每一条边的x增量信息
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
        // 扫描线填充
        for y in min_y..=max_y {
            let mut intersections = Vec::new();

            // 计算当前扫描线y与所有边的交点
            for &(y_start, y_end, x_start, dx) in &edge_info {
                if (y as f64) >= y_start && (y as f64) <= y_end {
                    let x = x_start + (y as f64 - y_start) * dx;
                    intersections.push(x);
                }
            }
            // 交点排序
            intersections.sort_by(|a, b| a.partial_cmp(b).expect("Float comparison failed"));
            // 填充扫描线交点之间的区域
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
    /// 填充任意多边形（使用奇偶规则）
    pub fn fill_polygon(&mut self, points: &[Pixel], color: color::Color) {
        if points.len() < 3 {
            return;
        }
        // 找到多边形的y范围
        let min_y = points.iter().map(|p| p.y).min().unwrap_or(0);
        let max_y = points.iter().map(|p| p.y).max().unwrap_or(0);
        // 收集所有边的信息
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
        // 扫描线填充
        for y in min_y..=max_y {
            let mut intersections = Vec::new();

            // 检查每条边是否与当前扫描线相交
            for &(y_min, y_max, mut x, dx) in &edge_table {
                if (y as f64) >= y_min && (y as f64) < y_max {
                    if y as f64 > y_min {
                        x += (y as f64 - y_min) * dx;
                    }
                    intersections.push(x);
                }
            }
            // 交点排序
            intersections.sort_by(|a, b| a.partial_cmp(b).expect("Float comparison failed"));
            // 填充扫描线交点之间的区域（奇偶规则）
            let mut inside = false;
            for i in 0..intersections.len() {
                if inside && i < intersections.len() {
                    let start_x = intersections[i].max(0.0).min(self.width() as f64 - 1.0) as u64;

                    // 确保不会越界访问
                    if i + 1 < intersections.len() {
                        let end_x =
                            intersections[i + 1].max(0.0).min(self.width() as f64 - 1.0) as u64;

                        if start_x <= end_x {
                            for x in start_x..=end_x {
                                self.set_pixel_raw(x, y, &color);
                            }
                        }
                    } else {
                        // 处理最后一个点
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

    /// 绘制BMP图像
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
    /// 绘制BMP图像 (带缩放)
    pub fn draw_bmp_scaled(&mut self, pos: Pixel, bmp: &BmpImage, scale_x: f32, scale_y: f32) {
        let scaled_width = (bmp.width() as f32 * scale_x) as u64;
        let scaled_height = (bmp.height() as f32 * scale_y) as u64;

        for y in 0..scaled_height {
            for x in 0..scaled_width {
                // 计算原始图像中的对应位置
                let src_x = (x as f32 / scale_x) as u32;
                let src_y = (y as f32 / scale_y) as u32;

                if let Some(color) = bmp.pixel(src_x, src_y) {
                    self.set_pixel_raw(pos.x + x, pos.y + y, &color);
                }
            }
        }
    }
    /// 绘制BMP图像 (扭曲变形)
    pub fn draw_bmp_distorted(&mut self, corners: [Pixel; 4], bmp: &BmpImage) {
        // 计算包围盒
        let min_x = corners.iter().map(|p| p.x).min().unwrap_or(0);
        let max_x = corners.iter().map(|p| p.x).max().unwrap_or(0);
        let min_y = corners.iter().map(|p| p.y).min().unwrap_or(0);
        let max_y = corners.iter().map(|p| p.y).max().unwrap_or(0);

        // 计算变换矩阵 (简化的双线性插值)
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // 计算相对位置 (简化版，实际应该使用更精确的纹理映射)
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
    /// 从字节加载并绘制BMP图像
    pub fn draw_bmp_from_bytes(&mut self, pos: Pixel, data: &[u8]) -> Result<(), BmpError> {
        let bmp = BmpImage::from_bytes(data)?;
        self.draw_bmp(pos, &bmp);
        Ok(())
    }

    /// 绘制圆形
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

    pub fn scroll_y(&mut self, offset: i64) {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        let pixel_size = self.pixel_size;
        let row_bytes = width as usize * pixel_size;

        if offset == 0 {
            return;
        }

        let abs_offset = offset.unsigned_abs();
        if abs_offset >= height {
            self.clear();
            return;
        }

        let move_rows = height - abs_offset;
        let move_bytes = move_rows as usize * row_bytes;

        if offset > 0 {
            let src_start = 0;
            let dest_start = abs_offset as usize * row_bytes;
            self.back_buffer
                .copy_within(src_start..move_bytes, dest_start);

            let clear_color = self.clear_color;
            let masked_color = self.mask_color(&clear_color);
            let pixel_bytes = masked_color.to_le_bytes();
            let bytes_to_fill = &pixel_bytes[..pixel_size];

            let mut clear_row = vec![0u8; row_bytes];
            for x in 0..width as usize {
                for i in 0..pixel_size {
                    clear_row[x * pixel_size + i] = bytes_to_fill[i];
                }
            }
            for y in 0..abs_offset {
                let off = y as usize * row_bytes;
                self.back_buffer[off..off + row_bytes].copy_from_slice(&clear_row);
            }
        } else {
            let src_start = abs_offset as usize * row_bytes;
            let dest_start = 0;
            self.back_buffer
                .copy_within(src_start..(src_start + move_bytes), dest_start);

            let clear_color = self.clear_color;
            let masked_color = self.mask_color(&clear_color);
            let pixel_bytes = masked_color.to_le_bytes();
            let bytes_to_fill = &pixel_bytes[..pixel_size];

            let mut clear_row = vec![0u8; row_bytes];
            for x in 0..width as usize {
                for i in 0..pixel_size {
                    clear_row[x * pixel_size + i] = bytes_to_fill[i];
                }
            }
            for y in move_rows..height {
                let off = y as usize * row_bytes;
                self.back_buffer[off..off + row_bytes].copy_from_slice(&clear_row);
            }
        }
        self.dirty_min_x = 0;
        self.dirty_min_y = 0;
        self.dirty_max_x = width;
        self.dirty_max_y = height;
    }

    /// 将后台缓冲区的内容复制到前台帧缓冲区，从而显示绘制结果。
    pub fn present(&mut self) {
        let fb_width = self.framebuffer.width();
        let fb_height = self.framebuffer.height();

        let min_x = self.dirty_min_x.min(fb_width);
        let min_y = self.dirty_min_y.min(fb_height);
        let max_x = self.dirty_max_x.min(fb_width);
        let max_y = self.dirty_max_y.min(fb_height);

        if min_x >= max_x || min_y >= max_y {
            return;
        }

        let width = (max_x - min_x) as usize;
        let pitch = self.framebuffer.pitch() as usize; // Framebuffer每行的字节数
        let pixel_size = self.pixel_size; // 后台缓冲区每个像素的字节数

        unsafe {
            let front_buffer_addr = self.framebuffer.addr();
            for y in min_y..max_y {
                let back_buffer_offset = (y * fb_width + min_x) as usize * pixel_size;
                let front_buffer_offset = y as usize * pitch + min_x as usize * pixel_size;

                let source_slice = &self.back_buffer
                    [back_buffer_offset..(back_buffer_offset + width * pixel_size)];

                let dest_ptr = front_buffer_addr.add(front_buffer_offset);
                let dest_slice = slice::from_raw_parts_mut(dest_ptr, width * pixel_size);

                dest_slice.copy_from_slice(source_slice);
            }
        }

        self.dirty_min_x = u64::MAX;
        self.dirty_min_y = u64::MAX;
        self.dirty_max_x = 0;
        self.dirty_max_y = 0;
    }
}
