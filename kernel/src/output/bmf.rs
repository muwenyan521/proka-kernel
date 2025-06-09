//! Proka Kernel - A kernel for ProkaOS
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! This file contains the BMF parset struct in order to parse the Bitmap Font.
//!
//! The purpose of it is to return the bits should paint, and show a character finally.
//!
//! Well, we uses `MiSans` here because it is very pretty, our studio's members all like it.

extern crate alloc;
use alloc::vec::Vec;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".font_data")]
#[used]
static DEFAULT_FONT_DATA: [u8; include_bytes!("../../fonts/default.bmf").len()] =
    *include_bytes!("../../fonts/default.bmf");

lazy_static::lazy_static! {
    pub static ref DEFAULT_FONT: BMFParser<'static> = BMFParser::new(
        &DEFAULT_FONT_DATA
    );
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BMFParser<'a> {
    pub font_size: u8,
    bytes_per_char: u8,
    hash_start: u32,
    hash_slots: u32,
    data: &'a [u8],
}

impl<'a> BMFParser<'a> {
    /// Creates a new BMF parser from raw font data
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            font_size: data[6],
            bytes_per_char: data[7],
            hash_start: u32::from_le_bytes([data[8], data[9], data[10], 0]),
            hash_slots: u32::from_le_bytes([data[11], data[12], data[13], 0]),
            data,
        }
    }

    /// Gets raw byte data for a specific Unicode character
    pub fn get_char_data(&self, unicode: u32) -> Option<&[u8]> {
        let mut slot = (unicode % self.hash_slots as u32) as usize;

        loop {
            let entry_start = self.hash_start as usize + slot * 6;
            let entry_end = entry_start + 6;

            // Check if entry is out of bounds
            if entry_end > self.data.len() {
                return None;
            }

            let entry = &self.data[entry_start..entry_end];
            let entry_unicode = u16::from_le_bytes([entry[0], entry[1]]);
            let entry_offset = u32::from_le_bytes([entry[2], entry[3], entry[4], entry[5]]);

            if entry_unicode as u32 == unicode {
                let char_start = entry_offset as usize;
                let char_end = char_start + (self.bytes_per_char as usize);

                // Check if character data is out of bounds
                if char_end > self.data.len() {
                    return None;
                }

                return Some(&self.data[char_start..char_end]);
            } else if entry_unicode == 0 {
                return None;
            }

            slot = (slot + 1) % self.hash_slots as usize;
        }
    }

    /// Converts character data to a grayscale image
    pub fn get_grayscale_image(&self, unicode: u32) -> Option<Vec<Vec<u8>>> {
        let char_data = self.get_char_data(unicode)?;

        // Calculate bytes per line (ceil(font_size/8))
        let bytes_per_line = ((self.font_size as usize) + 7) / 8;
        let mut image =
            alloc::vec![alloc::vec![0u8; self.font_size as usize]; self.font_size as usize];

        for y in 0..self.font_size as usize {
            let line_start = y * bytes_per_line;
            let line_end = line_start + bytes_per_line;

            if line_end > char_data.len() {
                break; // Partial line, but handle gracefully
            }

            let line_bytes = &char_data[line_start..line_end];

            for (byte_idx, &byte) in line_bytes.iter().enumerate() {
                for bit_idx in 0..8 {
                    let x = (byte_idx * 8) + bit_idx;
                    if x >= self.font_size as usize {
                        break; // Skip bits beyond font width
                    }

                    // Extract bit from byte (MSB to LSB)
                    let bit = (byte >> (7 - bit_idx)) & 1;
                    image[y][x] = if bit == 1 { 255 } else { 0 };
                }
            }
        }

        Some(image)
    }
}
