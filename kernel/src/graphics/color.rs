#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Creates a new Color with the specified RGB values and default alpha (255)
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Creates a new Color with the specified RGBA values
    pub const fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();

        if hex.len() == 8 {
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap();
            Self::with_alpha(r, g, b, a)
        } else {
            Self::new(r, g, b)
        }
    }

    pub fn to_u32(&self, include_alpha: bool) -> u32 {
        let r = self.r as u32;
        let g = self.g as u32;
        let b = self.b as u32;
        if include_alpha {
            let a = self.a as u32;
            return (a << 24) | (r << 16) | (g << 8) | b;
        }
        (r << 24) | (g << 16) | (b << 8)
    }

    pub fn from_u32(color: u32) -> Self {
        let r = (color >> 24) & 0xFF;
        let g = (color >> 16) & 0xFF;
        let b = (color >> 8) & 0xFF;
        Self::new(r as u8, g as u8, b as u8)
    }

    pub fn mix_alpha(&self, alpha: u8) -> Self {
        Self::with_alpha(self.r, self.g, self.b, alpha)
    }

    pub fn mix(&self, other: &Color, alpha: u8) -> Self {
        let r =
            ((self.r as u16 * alpha as u16 + other.r as u16 * (255 - alpha) as u16) / 255) as u8;
        let g =
            ((self.g as u16 * alpha as u16 + other.g as u16 * (255 - alpha) as u16) / 255) as u8;
        let b =
            ((self.b as u16 * alpha as u16 + other.b as u16 * (255 - alpha) as u16) / 255) as u8;
        let a = alpha;
        Self { r, g, b, a }
    }
}

#[macro_export]
macro_rules! color {
    // RGB 格式 (自动填充 alpha=255)
    ($r:expr, $g:expr, $b:expr) => {
        Color::new($r as u8, $g as u8, $b as u8)
    };

    // RGBA 格式
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        Color::with_alpha($r as u8, $g as u8, $b as u8, $a as u8)
    };

    // 十六进制颜色码 (支持 #RRGGBB 和 #RRGGBBAA)
    (#$hex:expr) => {
        Color::from_hex($hex)
    };
}

// 基础颜色
pub const BLACK: Color = color!(0, 0, 0);
pub const WHITE: Color = color!(255, 255, 255);
pub const RED: Color = color!(255, 0, 0);
pub const GREEN: Color = color!(0, 255, 0);
pub const BLUE: Color = color!(0, 0, 255);
// 实用混合色
pub const YELLOW: Color = color!(255, 255, 0); // 红+绿
pub const CYAN: Color = color!(0, 255, 255); // 绿+蓝
pub const MAGENTA: Color = color!(255, 0, 255); // 红+蓝
// 中性色
pub const GRAY: Color = color!(128, 128, 128);
