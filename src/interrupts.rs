pub struct InterruptFlags {
    pub vblank: bool,
    pub lcd_stat: bool,
    pub timer: bool,
    pub serial: bool,
    pub joypad: bool,
}

impl InterruptFlags {
    pub fn new() -> InterruptFlags {
        InterruptFlags {
            vblank: false,
            lcd_stat: false,
            timer: false,
            serial: false,
            joypad: false,
        }
    }

    pub fn from_byte(&mut self, byte: u8) {
        self.vblank = (byte & 0b00001) != 0;
        self.lcd_stat = (byte & 0b00010) != 0;
        self.timer = (byte & 0b00100) != 0;
        self.serial = (byte & 0b01000) != 0;
        self.joypad = (byte & 0b10000) != 0;
    }

    pub fn to_byte(&self) -> u8 {
        let mut result = 0;
        if self.vblank {
            result |= 0b00001;
        }
        if self.lcd_stat {
            result |= 0b00010;
        }
        if self.timer {
            result |= 0b00100;
        }
        if self.serial {
            result |= 0b01000;
        }
        if self.joypad {
            result |= 0b10000;
        }
        result
    }
}
