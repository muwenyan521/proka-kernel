//! Color representation and manipulation utilities
//!
//! This module provides the `Color` struct for representing RGBA colors and various
//! utilities for color manipulation, conversion, and predefined color constants.
//!
//! # Color Representation
//! Colors are represented as 8-bit RGBA values (red, green, blue, alpha) where:
//! - Each component ranges from 0 to 255
//! - Alpha value of 255 is fully opaque, 0 is fully transparent
//! - Colors can be created from RGB, RGBA, hexadecimal strings, or 32-bit integers
//!
//! # Features
//! - Color creation from multiple formats (RGB, RGBA, hex, u32)
//! - Color mixing and alpha blending
//! - Color inversion
//! - Conversion to/from 32-bit integer representation
//! - Convenient macro for color creation
//! - Predefined color constants
//!
//! # Examples
//! ```
//! use kernel::graphics::color::{Color, color, RED, BLUE};
//!
//! // Create colors using different methods
//! let red = Color::new(255, 0, 0);
//! let blue_with_alpha = Color::with_alpha(0, 0, 255, 128);
//! let green_from_hex = Color::from_hex("#00FF00");
//!
//! // Use the color! macro
//! let yellow = color!(255, 255, 0);
//! let semi_transparent_red = color!(255, 0, 0, 128);
//! let magenta = color!("#FF00FF");
//!
//! // Mix colors
//! let purple = red.mix(&blue, 128);
//! ```
//!
//! # Safety
//! This module contains no unsafe code. All operations are safe and panic-free
//! (except for `from_hex` which may panic on invalid hex strings).

/// Represents a color with red, green, blue, and alpha components
///
/// Each component is an 8-bit value ranging from 0 to 255. The alpha component
/// controls transparency, where 255 is fully opaque and 0 is fully transparent.
///
/// # Fields
/// * `r` - Red component (0-255)
/// * `g` - Green component (0-255)
/// * `b` - Blue component (0-255)
/// * `a` - Alpha (transparency) component (0-255)
///
/// # Examples
/// ```
/// use kernel::graphics::color::Color;
///
/// let opaque_red = Color { r: 255, g: 0, b: 0, a: 255 };
/// let semi_transparent_blue = Color { r: 0, g: 0, b: 255, a: 128 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Creates a new Color with the specified RGB values and default alpha (255)
    ///
    /// # Arguments
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # Returns
    /// A new `Color` with the specified RGB values and alpha set to 255 (fully opaque)
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let red = Color::new(255, 0, 0);
    /// let green = Color::new(0, 255, 0);
    /// let blue = Color::new(0, 0, 255);
    /// ```
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Creates a new Color with the specified RGBA values
    ///
    /// # Arguments
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    /// * `a` - Alpha component (0-255, where 255 is fully opaque)
    ///
    /// # Returns
    /// A new `Color` with the specified RGBA values
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let semi_transparent_red = Color::with_alpha(255, 0, 0, 128);
    /// let transparent_blue = Color::with_alpha(0, 0, 255, 64);
    /// ```
    pub const fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Creates a Color from a hexadecimal string
    ///
    /// Supports both `#RRGGBB` and `#RRGGBBAA` formats. The leading `#` is optional.
    ///
    /// # Arguments
    /// * `hex` - Hexadecimal color string (e.g., "#FF0000" or "#FF000080")
    ///
    /// # Returns
    /// A new `Color` parsed from the hexadecimal string
    ///
    /// # Panics
    /// Panics if the string is not a valid hexadecimal color representation
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let red = Color::from_hex("#FF0000");
    /// let green = Color::from_hex("00FF00");
    /// let semi_transparent_blue = Color::from_hex("#0000FF80");
    /// ```
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

        if hex.len() == 8 {
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            Self::with_alpha(r, g, b, a)
        } else {
            Self::new(r, g, b)
        }
    }

    /// Converts the color to a 32-bit integer representation
    ///
    /// The format depends on the `include_alpha` parameter:
    /// - If `include_alpha` is `true`: `AARRGGBB` format
    /// - If `include_alpha` is `false`: `RRGGBB00` format (alpha set to 0)
    ///
    /// # Arguments
    /// * `include_alpha` - Whether to include the alpha channel in the output
    ///
    /// # Returns
    /// A 32-bit integer representing the color
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let red = Color::new(255, 0, 0);
    /// let with_alpha = red.to_u32(true);  // 0xFF0000FF
    /// let without_alpha = red.to_u32(false); // 0xFF000000
    /// ```
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

    /// Creates a Color from a 32-bit integer
    ///
    /// Assumes the integer is in `RRGGBBAA` format (alpha in lowest 8 bits).
    /// Note: This method ignores the alpha channel from the input and creates
    /// an opaque color (alpha = 255).
    ///
    /// # Arguments
    /// * `color` - 32-bit integer in `RRGGBBAA` format
    ///
    /// # Returns
    /// A new `Color` parsed from the 32-bit integer
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let red = Color::from_u32(0xFF0000FF);
    /// let green = Color::from_u32(0x00FF00FF);
    /// ```
    pub fn from_u32(color: u32) -> Self {
        let r = (color >> 24) & 0xFF;
        let g = (color >> 16) & 0xFF;
        let b = (color >> 8) & 0xFF;
        Self::new(r as u8, g as u8, b as u8)
    }

    /// Creates a new color with the same RGB values but different alpha
    ///
    /// # Arguments
    /// * `alpha` - New alpha value (0-255)
    ///
    /// # Returns
    /// A new `Color` with the same RGB values and the specified alpha
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let red = Color::new(255, 0, 0);
    /// let semi_transparent_red = red.mix_alpha(128);
    /// ```
    pub fn mix_alpha(&self, alpha: u8) -> Self {
        Self::with_alpha(self.r, self.g, self.b, alpha)
    }

    /// Mixes two colors with the specified alpha blending factor
    ///
    /// The alpha parameter controls the blend ratio:
    /// - 0: Result is 100% `other` color
    /// - 255: Result is 100% `self` color
    /// - 128: Equal mix of both colors
    ///
    /// # Arguments
    /// * `other` - The other color to mix with
    /// * `alpha` - Blending factor (0-255)
    ///
    /// # Returns
    /// A new `Color` that is a blend of `self` and `other`
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let red = Color::new(255, 0, 0);
    /// let blue = Color::new(0, 0, 255);
    /// let purple = red.mix(&blue, 128); // Equal mix of red and blue
    /// ```
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

    /// Inverts the color (creates the complementary color)
    ///
    /// Each RGB component is subtracted from 255 to create the inverse color.
    /// The alpha channel remains unchanged.
    ///
    /// # Returns
    /// A new `Color` that is the inverse of `self`
    ///
    /// # Examples
    /// ```
    /// use kernel::graphics::color::Color;
    ///
    /// let white = Color::new(255, 255, 255);
    /// let black = white.invert(); // (0, 0, 0)
    ///
    /// let red = Color::new(255, 0, 0);
    /// let cyan = red.invert(); // (0, 255, 255)
    /// ```
    pub fn invert(&self) -> Color {
        Color::new(255 - self.r, 255 - self.g, 255 - self.b)
    }
}

/// Convenience macro for creating colors
///
/// This macro provides a concise syntax for creating `Color` instances.
/// It supports three formats:
///
/// 1. **RGB format**: `color!(r, g, b)` - Creates an opaque color
/// 2. **RGBA format**: `color!(r, g, b, a)` - Creates a color with alpha
/// 3. **Hex format**: `color!(#hex)` - Creates a color from a hex string
///
/// # Examples
/// ```
/// use kernel::graphics::color::color;
///
/// // RGB format
/// let red = color!(255, 0, 0);
/// let green = color!(0, 255, 0);
///
/// // RGBA format
/// let semi_transparent_blue = color!(0, 0, 255, 128);
///
/// // Hex format
/// let magenta = color!("#FF00FF");
/// let cyan = color!("#00FFFF80"); // With alpha
/// ```
#[macro_export]
macro_rules! color {
    // RGB format (automatically fills alpha=255)
    ($r:expr, $g:expr, $b:expr) => {
        Color::new($r as u8, $g as u8, $b as u8)
    };

    // RGBA format
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        Color::with_alpha($r as u8, $g as u8, $b as u8, $a as u8)
    };

    // Hexadecimal color code (supports #RRGGBB and #RRGGBBAA)
    (#$hex:expr) => {
        Color::from_hex($hex)
    };
}

/// Pure black color (RGB: 0, 0, 0)
pub const BLACK: Color = color!(0, 0, 0);

/// Pure white color (RGB: 255, 255, 255)
pub const WHITE: Color = color!(255, 255, 255);

/// Pure red color (RGB: 255, 0, 0)
pub const RED: Color = color!(255, 0, 0);

/// Pure green color (RGB: 0, 255, 0)
pub const GREEN: Color = color!(0, 255, 0);

/// Pure blue color (RGB: 0, 0, 255)
pub const BLUE: Color = color!(0, 0, 255);

/// Yellow color (RGB: 255, 255, 0) - Mix of red and green
pub const YELLOW: Color = color!(255, 255, 0);

/// Cyan color (RGB: 0, 255, 255) - Mix of green and blue
pub const CYAN: Color = color!(0, 255, 255);

/// Magenta color (RGB: 255, 0, 255) - Mix of red and blue
pub const MAGENTA: Color = color!(255, 0, 255);

/// Medium gray color (RGB: 128, 128, 128)
pub const GRAY: Color = color!(128, 128, 128);
