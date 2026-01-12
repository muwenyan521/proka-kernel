use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

const PIT_BASE_FREQ: u64 = 1_193_182;

lazy_static! {
    pub static ref PIT: Mutex<Pit> = Mutex::new(Pit::new());
}

pub struct Pit {
    _channel0: Port<u8>,
    _channel1: Port<u8>,
    channel2: Port<u8>,
    command: Port<u8>,
    speaker_port: Port<u8>,
}

impl Pit {
    const fn new() -> Self {
        Pit {
            _channel0: Port::new(0x40),
            _channel1: Port::new(0x41),
            channel2: Port::new(0x42),
            command: Port::new(0x43),
            speaker_port: Port::new(0x61),
        }
    }
    pub fn sleep_blocking(&mut self, us: u64) {
        // Calculate ticks
        // ticks = us * 1193182 / 1000000
        //      = us * 1.193182
        let ticks = (us * PIT_BASE_FREQ) / 1_000_000;

        let mut remaining_ticks = ticks;

        while remaining_ticks > 0 {
            let current_ticks = if remaining_ticks > 0xFFFF {
                0xFFFF
            } else {
                remaining_ticks as u16
            };

            self.wait_ch2_ticks(current_ticks);
            remaining_ticks -= current_ticks as u64;
        }
    }

    /// Read the current count of Channel 2
    pub fn read_count(&mut self) -> u16 {
        unsafe {
            // Send Latch Command for Channel 2
            // Command: 10 00 000 0 = 0x80
            // Channel 2 (10)
            // Latch Count (00) - snapshot current count to internal latch
            // Mode X (don't care)
            // BCD X (don't care)
            self.command.write(0x80);

            // Read LSB then MSB from Channel 2 data port
            let low = self.channel2.read();
            let high = self.channel2.read();

            u16::from_le_bytes([low, high])
        }
    }

    /// Prepare Channel 2 for one-shot counting (Mode 0)
    /// Returns the timer to the starting state but does not wait
    pub fn start_one_shot(&mut self, ticks: u16) {
        unsafe {
            // 1. Enable Channel 2 Gate (Port 0x61 Bit 0)
            let mut port61_val = self.speaker_port.read();
            // Set Bit 0 (Gate 2) to 1 to enable counting
            // Set Bit 1 (Speaker) to 0 to disable speaker output
            port61_val = (port61_val | 1) & !2;
            self.speaker_port.write(port61_val);

            // 2. Configure Channel 2
            // Select Channel 2 (10)
            // Access Mode LSB/MSB (11)
            // Mode 0: Interrupt on Terminal Count (000)
            // Binary (0)
            // Command: 10 11 000 0 = 0xB0
            self.command.write(0xB0);

            // 3. Write Count
            self.channel2.write((ticks & 0xFF) as u8);
            self.channel2.write((ticks >> 8) as u8);
        }
    }

    fn wait_ch2_ticks(&mut self, ticks: u16) {
        self.start_one_shot(ticks);

        // 4. Wait for Out 2 (Bit 5 of Port 0x61) to go HIGH
        unsafe {
            loop {
                let status = self.speaker_port.read();
                if (status & 0x20) != 0 {
                    break;
                }
                core::hint::spin_loop();
            }
        }
    }
}
