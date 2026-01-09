extern crate alloc;
use crate::drivers::{CharDevice, Device, DeviceError, DeviceInner, DeviceType, SharedDeviceOps};
use crate::serial_println;
use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex;

const BUFFER_SIZE: usize = 128;

pub struct KeyboardInner {
    shift_pressed: bool,
    caps_lock: bool,
    enabled: bool,
    buffer: [char; BUFFER_SIZE],
    head: usize,
    tail: usize,
}

pub struct Keyboard {
    inner: Mutex<KeyboardInner>,
    name: String,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(KeyboardInner {
                shift_pressed: false,
                caps_lock: false,
                enabled: true,
                buffer: ['\0'; BUFFER_SIZE],
                head: 0,
                tail: 0,
            }),
            name: String::from("keyboard"),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.inner.lock().enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.lock().enabled
    }

    pub fn handle_scancode(&self, scancode: u8) {
        let mut inner = self.inner.lock();
        if !inner.enabled {
            return;
        }

        let c = match scancode {
            0x2A | 0x36 => {
                inner.shift_pressed = true;
                None
            }
            0xAA | 0xB6 => {
                inner.shift_pressed = false;
                None
            }
            0x3A => {
                inner.caps_lock = !inner.caps_lock;
                None
            }
            s if s & 0x80 != 0 => None,
            _ => self.scancode_to_char(scancode, inner.shift_pressed, inner.caps_lock),
        };

        if let Some(ch) = c {
            serial_println!("Key: {}", ch);
            let tail = inner.tail;
            let next_tail = (tail + 1) % BUFFER_SIZE;
            if next_tail != inner.head {
                inner.buffer[tail] = ch;
                inner.tail = next_tail;
            }
        }
    }

    fn scancode_to_char(&self, scancode: u8, shift_pressed: bool, caps_lock: bool) -> Option<char> {
        let is_upper = shift_pressed ^ caps_lock;

        match scancode {
            0x02 => Some(if shift_pressed { '!' } else { '1' }),
            0x03 => Some(if shift_pressed { '@' } else { '2' }),
            0x04 => Some(if shift_pressed { '#' } else { '3' }),
            0x05 => Some(if shift_pressed { '$' } else { '4' }),
            0x06 => Some(if shift_pressed { '%' } else { '5' }),
            0x07 => Some(if shift_pressed { '^' } else { '6' }),
            0x08 => Some(if shift_pressed { '&' } else { '7' }),
            0x09 => Some(if shift_pressed { '*' } else { '8' }),
            0x0A => Some(if shift_pressed { '(' } else { '9' }),
            0x0B => Some(if shift_pressed { ')' } else { '0' }),

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

    pub fn create_device() -> Device {
        let keyboard = Arc::new(Keyboard::new());
        Device::new_auto_assign(keyboard.name.clone(), DeviceInner::Char(keyboard))
    }
}

impl SharedDeviceOps for Keyboard {
    fn name(&self) -> &str {
        &self.name
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Char
    }

    fn open(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn close(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn ioctl(&self, _cmd: u64, _arg: u64) -> Result<u64, DeviceError> {
        Err(DeviceError::NotSupported)
    }
}

impl CharDevice for Keyboard {
    fn read(&self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let mut inner = self.inner.lock();
        let mut read_count = 0;

        while read_count < buf.len() && inner.head != inner.tail {
            let c = inner.buffer[inner.head];
            inner.head = (inner.head + 1) % BUFFER_SIZE;

            let mut char_buf = [0u8; 4];
            let char_str = c.encode_utf8(&mut char_buf);
            let bytes = char_str.as_bytes();

            if read_count + bytes.len() <= buf.len() {
                buf[read_count..read_count + bytes.len()].copy_from_slice(bytes);
                read_count += bytes.len();
            } else {
                inner.head = (inner.head + BUFFER_SIZE - 1) % BUFFER_SIZE;
                break;
            }
        }

        if read_count == 0 && buf.len() > 0 {
            Err(DeviceError::WouldBlock)
        } else {
            Ok(read_count)
        }
    }

    fn write(&self, _buf: &[u8]) -> Result<usize, DeviceError> {
        Err(DeviceError::NotSupported)
    }

    fn has_data(&self) -> bool {
        let inner = self.inner.lock();
        inner.head != inner.tail
    }
}

lazy_static::lazy_static! {
    pub static ref KEYBOARD: Arc<Keyboard> = Arc::new(Keyboard::new());
}
