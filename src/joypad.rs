#[derive(Debug, PartialEq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

pub struct Joypad {
    action_buttons: u8,
    direction_buttons: u8,
    selected_buttons: u8,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            action_buttons: 0x0F,
            direction_buttons: 0x0F,
            selected_buttons: 0xF0,
        }
    }

    pub fn push_button(&mut self, key: Key) {
        match key {
            Key::A => self.action_buttons &= 0x01 ^ 0xF,
            Key::B => self.action_buttons &= 0x02 ^ 0xF,
            Key::Select => self.action_buttons &= 0x04 ^ 0xF,
            Key::Start => self.action_buttons &= 0x08 ^ 0xF,
            Key::Right => self.direction_buttons &= 0x01 ^ 0xF,
            Key::Left => self.direction_buttons &= 0x02 ^ 0xF,
            Key::Up => self.direction_buttons &= 0x04 ^ 0xF,
            Key::Down => self.direction_buttons &= 0x08 ^ 0xF,
        }
    }

    pub fn release_button(&mut self, key: Key) {
        match key {
            Key::A => self.action_buttons |= 0x01,
            Key::B => self.action_buttons |= 0x02,
            Key::Select => self.action_buttons |= 0x04,
            Key::Start => self.action_buttons |= 0x08,
            Key::Right => self.direction_buttons |= 0x01,
            Key::Left => self.direction_buttons |= 0x02,
            Key::Up => self.direction_buttons |= 0x04,
            Key::Down => self.direction_buttons |= 0x08,
        }
    }

    pub fn read_input(&self) -> u8 {
        let value = self.selected_buttons & 0x30;
        match value {
            0x10 => self.action_buttons,
            0x20 => self.direction_buttons,
            0x30 => 0xCF,
            _ => 0x0,
        }
    }

    pub fn write(&mut self, value: u8) {
        // println!("Joypad write: {:#04x}", value);
        self.selected_buttons = value;
    }
}
