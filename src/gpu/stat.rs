#[derive(Copy, Clone)]
pub enum Mode {
    HorizontalBlank = 0,
    VerticalBlank = 1,
    OAMAccess = 2,
    VRAMAccess = 3,
}

pub struct Stat {
    pub coincidence_interrupt: bool,
    pub oam_interrupt: bool,
    pub v_blank_interrupt: bool,
    pub h_blank_interrupt: bool,
    pub coincidence_flag: bool,
    pub mode: Mode,
}

impl Stat {
    pub fn new() -> Stat {
        Stat {
            coincidence_interrupt: false,
            oam_interrupt: false,
            v_blank_interrupt: false,
            h_blank_interrupt: false,
            coincidence_flag: false,
            mode: Mode::HorizontalBlank,
        }
    }

    pub fn to_byte(&self) -> u8 {
        let mut byte = 0;
        if self.coincidence_interrupt {
            byte |= 1 << 6;
        }
        if self.oam_interrupt {
            byte |= 1 << 5;
        }
        if self.v_blank_interrupt {
            byte |= 1 << 4;
        }
        if self.h_blank_interrupt {
            byte |= 1 << 3;
        }
        if self.coincidence_flag {
            byte |= 1 << 2;
        }
        byte | self.mode as u8
    }

    pub fn from_byte(&mut self, byte: u8) {
        self.coincidence_interrupt = byte & (1 << 6) != 0;
        self.oam_interrupt = byte & (1 << 5) != 0;
        self.v_blank_interrupt = byte & (1 << 4) != 0;
        self.h_blank_interrupt = byte & (1 << 3) != 0;
    }
}
