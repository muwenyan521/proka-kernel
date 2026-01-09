use spin::Mutex;
use lazy_static::lazy_static;

pub struct Keyboard {
    shift_pressed: bool,
    caps_lock: bool,
    enabled: bool,
}

impl Keyboard {
    pub const fn new() -> Self {
        Self {
            shift_pressed: false,
            caps_lock: false,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn handle_scancode(&mut self, scancode: u8) -> Option<char> {
        if !self.enabled {
            return None;
        }
        match scancode {
            0x2A | 0x36 => {
                self.shift_pressed = true;
                None
            }
            0xAA | 0xB6 => {
                self.shift_pressed = false;
                None
            }
            0x3A => {
                self.caps_lock = !self.caps_lock;
                None
            }
            s if s & 0x80 != 0 => None,
            _ => self.scancode_to_char(scancode),
        }
    }

    fn scancode_to_char(&self, scancode: u8) -> Option<char> {
        let is_upper = self.shift_pressed ^ self.caps_lock;
        
        match scancode {
            0x02 => Some(if self.shift_pressed { '!' } else { '1' }),
            0x03 => Some(if self.shift_pressed { '@' } else { '2' }),
            0x04 => Some(if self.shift_pressed { '#' } else { '3' }),
            0x05 => Some(if self.shift_pressed { '$' } else { '4' }),
            0x06 => Some(if self.shift_pressed { '%' } else { '5' }),
            0x07 => Some(if self.shift_pressed { '^' } else { '6' }),
            0x08 => Some(if self.shift_pressed { '&' } else { '7' }),
            0x09 => Some(if self.shift_pressed { '*' } else { '8' }),
            0x0A => Some(if self.shift_pressed { '(' } else { '9' }),
            0x0B => Some(if self.shift_pressed { ')' } else { '0' }),
            
            0x10 => Some(if is_upper { 'Q' } else { 'q' }),
            0x11 => Some(if is_upper { 'W' } else { 'w' }),
            0x12 => Some(if is_upper { 'E' } else { 'e' }),
            0x13 => Some(if is_upper { 'R' } else { 'r' }),
            0x14 => Some(if is_upper { 'T' } else { 't' }),
            0x15 => Some(if is_upper { 'Y' } else { 'y' }),
            0x16 => Some(if is_upper { 'U' } else { 'u' }),
            0x17 => Some(if is_upper { 'I' } else { 'i' }),
            0x18 => Some(if is_upper { 'O' } else { 'o' }),
            0x19 => Some(if is_upper { 'P' } else { 'p' }),
            
            0x1E => Some(if is_upper { 'A' } else { 'a' }),
            0x1F => Some(if is_upper { 'S' } else { 's' }),
            0x20 => Some(if is_upper { 'D' } else { 'd' }),
            0x21 => Some(if is_upper { 'F' } else { 'f' }),
            0x22 => Some(if is_upper { 'G' } else { 'g' }),
            0x23 => Some(if is_upper { 'H' } else { 'h' }),
            0x24 => Some(if is_upper { 'J' } else { 'j' }),
            0x25 => Some(if is_upper { 'K' } else { 'k' }),
            0x26 => Some(if is_upper { 'L' } else { 'l' }),
            
            0x2C => Some(if is_upper { 'Z' } else { 'z' }),
            0x2D => Some(if is_upper { 'X' } else { 'x' }),
            0x2E => Some(if is_upper { 'C' } else { 'c' }),
            0x2F => Some(if is_upper { 'V' } else { 'v' }),
            0x30 => Some(if is_upper { 'B' } else { 'b' }),
            0x31 => Some(if is_upper { 'N' } else { 'n' }),
            0x32 => Some(if is_upper { 'M' } else { 'm' }),
            
            0x39 => Some(' '),
            0x1C => Some('\n'),
            0x0E => Some('\x08'),
            
            _ => None,
        }
    }
}

lazy_static! {
    pub static ref KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new());
}
