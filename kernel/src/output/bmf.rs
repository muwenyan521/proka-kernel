extern crate alloc;

use alloc::vec::Vec;
use libm::ceil;
use spin::Mutex;

const DEFAULT_FONT_DATA: &[u8] = include_bytes!("../../fonts/default.bmf");

lazy_static::lazy_static! {
    pub static ref DEFAULT_FONT: Mutex<BMFParser> = Mutex::new(BMFParser::new(
        DEFAULT_FONT_DATA.to_vec()
    ));
}

pub struct BMFParser {
    pub font_size: u8,
    bytes_per_char: u8,
    bitmap_start: usize,
    hash_start: usize,
    hash_slots: usize,
    data: Vec<u8>,
}

impl BMFParser {
    pub fn new(data: Vec<u8>) -> Self {
        BMFParser {
            font_size: data[6],
            bytes_per_char: data[7],
            bitmap_start: u32::from_le_bytes([data[3], data[4], data[5], 0]) as usize,
            hash_start: u32::from_le_bytes([data[8], data[9], data[10], 0]) as usize,
            hash_slots: u32::from_le_bytes([data[11], data[12], data[13], 0]) as usize,
            data,
        }
    }

    pub fn get_bytes(&self, unicode: u32) -> Option<&[u8]> {
        let mut slot = (unicode % self.hash_slots as u32) as usize;
        let mut count = 0;

        loop {
            if count >= 50 {
                // 防止死循环
                return None;
            }

            let entry_start = self.hash_start + slot * 6;
            let entry_end = entry_start + 6;
            let entry = &self.data[entry_start..entry_end];

            let entry_unicode = u16::from_le_bytes([entry[0], entry[1]]);
            let entry_offset = u32::from_le_bytes([entry[2], entry[3], entry[4], 0]) as usize;

            if entry_unicode as u32 == unicode {
                let char_start = entry_offset;
                let char_end = entry_offset + self.bytes_per_char as usize;
                return Some(&self.data[char_start..char_end]);
            } else if entry_unicode == 0 {
                return None;
            }

            slot = (slot + 1) % self.hash_slots;
            count += 1;
        }
    }

    pub fn get_grayscale_image(&self, unicode: u32) -> Option<Vec<Vec<u8>>> {
        if let Some(char_data) = self.get_bytes(unicode) {
            let bytes_per_line = ceil((self.font_size as f64 / 8.0) as f64) as usize;
            let mut image = alloc::vec![
                alloc::vec![0; self.font_size as usize]
            ];

            for y in 0..self.font_size as usize {
                let line_start = y * bytes_per_line;
                let line_end = (y + 1) * bytes_per_line;
                let line_byte = &char_data[line_start..line_end];

                let mut bits = Vec::new();
                for &byte in line_byte {
                    bits.extend((0..8).rev().map(|i| (byte >> i) & 1));
                }

                for x in 0..self.font_size as usize {
                    image[y][x] = if bits[x] == 1 { 255 } else { 0 };
                }
            }

            Some(image)
        } else {
            None
        }
    }
}

