#[derive(Debug, Clone, Copy)]
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
}
pub const BLACK: Color = Color::new(0, 0, 0);
pub const WHITE: Color = Color::new(255, 255, 255);
pub const RED: Color = Color::new(255, 0, 0);
pub const GREEN: Color = Color::new(0, 255, 0);
pub const BLUE: Color = Color::new(0, 0, 255);
pub const YELLOW: Color = Color::new(255, 255, 0);
pub const CYAN: Color = Color::new(0, 255, 255);
pub const MAGENTA: Color = Color::new(255, 0, 255);
pub const GRAY: Color = Color::new(128, 128, 128);
