//! BMP图像格式解析模块
//!
//! 此模块提供Windows位图（BMP）图像格式的解析功能。
//! BMP是一种常见的无损图像格式，广泛用于图标、光标和简单图像存储。
//!
//! ## 功能
//!
//! - 解析BMP文件头和数据
//! - 支持24位和32位BMP格式
//! - 将BMP像素数据转换为内核颜色格式
//! - 提供图像尺寸和像素访问接口
//!
//! ## 支持的BMP格式
//!
//! 此模块支持以下BMP格式：
//! - **24位BMP**：每个像素使用3字节（BGR顺序）
//! - **32位BMP**：每个像素使用4字节（BGRA顺序）
//!
//! ## BMP文件结构
//!
//! BMP文件由以下部分组成：
//! 1. **文件头**（14字节）：包含文件类型、大小和数据偏移量
//! 2. **信息头**（40字节）：包含图像尺寸、位深度等信息
//! 3. **像素数据**：按行存储的像素数据（BMP是倒序存储的）
//!
//! ## 示例
//!
//! ```no_run
//! use kernel::libs::bmp;
//!
//! // 从字节数据加载BMP图像
//! let bmp_data = include_bytes!("example.bmp");
//! match bmp::BmpImage::from_bytes(bmp_data) {
//!     Ok(image) => {
//!         println!("BMP图像尺寸: {}x{}", image.width(), image.height());
//!         // 访问像素数据
//!         if let Some(pixel) = image.pixel(10, 10) {
//!             println!("像素(10,10)的颜色: {:?}", pixel);
//!         }
//!     }
//!     Err(err) => println!("BMP解析失败: {:?}", err),
//! }
//! ```

extern crate alloc;
use crate::graphics::color;
use alloc::vec::Vec;

/// BMP解析错误类型
///
/// 表示在解析BMP文件时可能发生的各种错误。
#[derive(Debug)]
pub enum BmpError {
    /// 无效的BMP文件签名
    ///
    /// BMP文件必须以"BM"（0x42 0x4D）开头。
    InvalidSignature,
    
    /// 不支持的BMP格式
    ///
    /// 目前只支持24位和32位BMP格式。
    UnsupportedFormat,
    
    /// 无效的BMP数据
    ///
    /// 像素数据损坏或文件不完整。
    InvalidData,
}

/// BMP图像结构
///
/// 表示一个已解析的BMP图像，包含图像的尺寸和像素数据。
///
/// ## 内存布局
///
/// 像素数据以行优先顺序存储，每行从左到右，行从上到下。
/// 这与BMP文件的存储顺序（倒序）相反，以提供更自然的访问方式。
pub struct BmpImage {
    /// 图像宽度（像素）
    width: u32,
    /// 图像高度（像素）
    height: u32,
    /// 像素数据向量
    pixels: Vec<color::Color>,
}

impl BmpImage {
    /// 从字节数据解析BMP图像
    ///
    /// 此函数解析BMP文件格式，提取图像尺寸和像素数据。
    ///
    /// # 参数
    ///
    /// * `data` - 包含BMP文件数据的字节切片
    ///
    /// # 返回值
    ///
    /// 如果解析成功，返回`Ok(BmpImage)`；否则返回`Err(BmpError)`。
    ///
    /// # 错误
    ///
    /// 此函数可能返回以下错误：
    /// - `BmpError::InvalidSignature`：文件不是有效的BMP文件
    /// - `BmpError::UnsupportedFormat`：不支持的位深度（不是24位或32位）
    /// - `BmpError::InvalidData`：像素数据损坏或文件不完整
    ///
    /// # 算法细节
    ///
    /// 1. 验证BMP文件签名（"BM"）
    /// 2. 解析文件头和信息头
    /// 3. 检查支持的位深度（24或32位）
    /// 4. 计算行填充（BMP行长度必须是4字节对齐）
    /// 5. 解析像素数据（BMP是倒序存储的）
    /// 6. 将BGR(A)像素转换为内核颜色格式
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use kernel::libs::bmp;
    ///
    /// let bmp_data = include_bytes!("logo.bmp");
    /// match bmp::BmpImage::from_bytes(bmp_data) {
    ///     Ok(image) => println!("成功加载BMP图像"),
    ///     Err(err) => println!("加载失败: {:?}", err),
    /// }
    /// ```
    pub fn from_bytes(data: &[u8]) -> Result<Self, BmpError> {
        // 检查BMP文件头
        if data.len() < 54 || data[0] != b'B' || data[1] != b'M' {
            return Err(BmpError::InvalidSignature);
        }

        // 解析BMP文件头
        let width = u32::from_le_bytes([data[18], data[19], data[20], data[21]]);
        let height = u32::from_le_bytes([data[22], data[23], data[24], data[25]]);
        let bpp = u16::from_le_bytes([data[28], data[29]]);

        // 目前只支持24位和32位BMP
        if bpp != 24 && bpp != 32 {
            return Err(BmpError::UnsupportedFormat);
        }

        let data_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;

        let mut pixels = Vec::with_capacity((width * height) as usize);
        let bytes_per_pixel = (bpp / 8) as usize;
        let row_padding = (4 - (width * bytes_per_pixel as u32) % 4) % 4;

        // 解析像素数据 (BMP是倒序存储的)
        for y in 0..height {
            let row_start = data_offset
                + ((height - y - 1) * (width * bytes_per_pixel as u32 + row_padding)) as usize;

            for x in 0..width {
                let pixel_start = row_start + (x * bytes_per_pixel as u32) as usize;
                if pixel_start + bytes_per_pixel > data.len() {
                    return Err(BmpError::InvalidData);
                }

                let b = data[pixel_start];
                let g = data[pixel_start + 1];
                let r = data[pixel_start + 2];
                let a = if bytes_per_pixel == 4 {
                    data[pixel_start + 3]
                } else {
                    255
                };

                pixels.push(color::Color::with_alpha(r, g, b, a));
            }
        }

        Ok(BmpImage {
            width,
            height,
            pixels,
        })
    }

    /// 获取图像宽度
    ///
    /// # 返回值
    ///
    /// 图像的宽度（像素数）。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use kernel::libs::bmp;
    ///
    /// let image = bmp::BmpImage::from_bytes(bmp_data).unwrap();
    /// println!("图像宽度: {}", image.width());
    /// ```
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 获取图像高度
    ///
    /// # 返回值
    ///
    /// 图像的高度（像素数）。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use kernel::libs::bmp;
    ///
    /// let image = bmp::BmpImage::from_bytes(bmp_data).unwrap();
    /// println!("图像高度: {}", image.height());
    /// ```
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 获取指定位置的像素颜色
    ///
    /// # 参数
    ///
    /// * `x` - X坐标（0到宽度-1）
    /// * `y` - Y坐标（0到高度-1）
    ///
    /// # 返回值
    ///
    /// 如果坐标在图像范围内，返回`Some(Color)`；否则返回`None`。
    ///
    /// # 注意
    ///
    /// 坐标原点在左上角，X轴向右，Y轴向下。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use kernel::libs::bmp;
    ///
    /// let image = bmp::BmpImage::from_bytes(bmp_data).unwrap();
    /// if let Some(color) = image.pixel(10, 20) {
    ///     println!("像素(10,20)的颜色: {:?}", color);
    /// }
    /// ```
    pub fn pixel(&self, x: u32, y: u32) -> Option<color::Color> {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            Some(self.pixels[index])
        } else {
            None
        }
    }
}
