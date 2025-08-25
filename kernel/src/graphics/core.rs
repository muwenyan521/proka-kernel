use crate::graphics::color;
use glam::U64Vec2;
use limine::framebuffer::Framebuffer;

pub type Pixel = U64Vec2;

pub struct Renderer<'a> {
    buffer: Framebuffer<'a>,
    clear_color: color::Color,
}

impl<'a> Renderer<'a> {
    pub fn new(framebuffer: Framebuffer<'a>) -> Self {
        Self {
            buffer: framebuffer,
            clear_color: color::BLACK,
        }
    }

    #[inline(always)]
    fn get_offset(&self, x: u64, y: u64) -> usize {
        (y * self.buffer.pitch() + x * (self.buffer.bpp() as u64 / 8)) as usize
    }

    fn mask_color(&self, color: &color::Color) -> u32 {
        if self.buffer.bpp() == 32 {
            let value: u32 = ((color.r as u32) << self.buffer.red_mask_shift())
                | ((color.g as u32) << self.buffer.green_mask_shift())
                | ((color.b as u32) << self.buffer.blue_mask_shift());
            return value;
        } else if self.buffer.bpp() == 24 {
            color.to_u32(false)
        } else {
            panic!("Unsupported bit per pixel")
        }
    }
    #[inline(always)]
    fn set_pixel_raw(&mut self, x: u64, y: u64, color: &color::Color) {
        // 边界检查：确保像素在屏幕范围内
        if x < self.buffer.width() && y < self.buffer.height() {
            let offset = self.get_offset(x, y);

            let color_u32 = if color.a == 255 {
                self.mask_color(color)
            } else if color.a > 0 {
                // 获取当前像素颜色
                let current_color = self.get_pixel_raw(x, y);

                // 执行alpha混合: result = (source * alpha + destination * (255 - alpha)) / 255
                let alpha = color.a as u32;
                let inv_alpha = 255 - alpha;

                let r = (color.r as u32 * alpha + current_color.r as u32 * inv_alpha) / 255;
                let g = (color.g as u32 * alpha + current_color.g as u32 * inv_alpha) / 255;
                let b = (color.b as u32 * alpha + current_color.b as u32 * inv_alpha) / 255;

                let mixed_color = color::Color::with_alpha(r as u8, g as u8, b as u8, 255);

                self.mask_color(&mixed_color)
            } else {
                // 透明,不绘制
                return;
            };

            unsafe {
                self.buffer
                    .addr()
                    .add(offset)
                    .cast::<u32>()
                    .write(color_u32);
            }
        }
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, pixel: Pixel, color: &color::Color) {
        let (x, y) = (pixel.x, pixel.y);
        self.set_pixel_raw(x, y, color);
    }

    pub fn get_pixel(&self, pixel: Pixel) -> color::Color {
        let (x, y) = (pixel.x, pixel.y);
        self.get_pixel_raw(x, y)
    }

    fn get_pixel_raw(&self, x: u64, y: u64) -> color::Color {
        let offset = self.get_offset(x, y);
        unsafe {
            let p = self.buffer.addr().add(offset).cast::<u32>();
            color::Color::from_u32(*p)
        }
    }

    pub fn set_clear_color(&mut self, color: color::Color) {
        self.clear_color = color;
    }

    pub fn get_clear_color(&self) -> color::Color {
        self.clear_color
    }

    pub fn clear(&mut self) {
        let width = self.buffer.width();
        let height = self.buffer.height();
        let color = self.clear_color.clone();
        for y in 0..height {
            for x in 0..width {
                self.set_pixel_raw(x, y, &color);
            }
        }
    }

    pub fn draw_line(&mut self, p1: Pixel, p2: Pixel, color: color::Color) {
        let dx_abs = ((p2.x as i64 - p1.x as i64).abs()) as u64;
        let dy_abs = ((p2.y as i64 - p1.y as i64).abs()) as u64;
        let steep = dy_abs > dx_abs;

        let (mut x1, mut y1, mut x2, mut y2) = (p1.x, p1.y, p2.x, p2.y);

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
                if y >= 0 && (y as u64) < self.buffer.width() && x < self.buffer.height() {
                    self.set_pixel_raw(y as u64, x, &color);
                }
            } else {
                if x < self.buffer.width() && y >= 0 && (y as u64) < self.buffer.height() {
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

    pub fn draw_triangle(&mut self, p1: Pixel, p2: Pixel, p3: Pixel, color: color::Color) {
        self.draw_line(p1, p2, color);
        self.draw_line(p2, p3, color);
        self.draw_line(p3, p1, color);
    }

    pub fn fill_triangle(&mut self, p1: Pixel, p2: Pixel, p3: Pixel, color: color::Color) {
        let (x1, y1) = (p1.x, p1.y);
        let (x2, y2) = (p2.x, p2.y);
        let (x3, y3) = (p3.x, p3.y);

        // 定义3个变换后的 Pixel
        let mut pts = [Pixel::new(x1, y1), Pixel::new(x2, y2), Pixel::new(x3, y3)];

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
            if y < 0 || y >= self.buffer.height() as i32 {
                return;
            }

            let mut start_x = start_x.max(0);
            let mut end_x = end_x.min(self.buffer.width() as i32 - 1);
            if start_x > end_x {
                core::mem::swap(&mut start_x, &mut end_x);
            }

            if start_x < 0 || end_x >= self.buffer.width() as i32 {
                start_x = start_x.max(0);
                end_x = end_x.min(self.buffer.width() as i32 - 1);
                if start_x > end_x {
                    return;
                }
            }

            // 填充
            for x in start_x..=end_x {
                if x >= 0 {
                    let pixel = Pixel::new(x as u64, y as u64);
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
        self.buffer.width()
    }

    pub fn height(&self) -> u64 {
        self.buffer.height()
    }

    pub fn draw_rect(&mut self, pixel: Pixel, width: u64, height: u64, color: color::Color) -> () {
        let (x, y) = (pixel.x, pixel.y);
        let x2 = x + width;
        let y2 = y + height;

        // 绘制四条边
        self.draw_line(Pixel::new(x, y), Pixel::new(x2, y), color);

        self.draw_line(Pixel::new(x2, y), Pixel::new(x2, y2), color);

        self.draw_line(Pixel::new(x2, y2), Pixel::new(x, y2), color);

        self.draw_line(Pixel::new(x, y2), Pixel::new(x, y), color);
    }

    pub fn fill_rect(&mut self, pixel: Pixel, width: u64, height: u64, color: color::Color) {
        let (x_min, y_min) = (pixel.x, pixel.y);
        let x_max = x_min + width;
        let y_max = y_min + height;

        let x_start = x_min.max(0);
        let x_end = x_max.min(self.width() - 1);
        let y_start = y_min.max(0);
        let y_end = y_max.min(self.height() - 1);

        for y in y_start..=y_end {
            for x in x_start..=x_end {
                self.set_pixel_raw(x, y, &color);
            }
        }
    }
}
