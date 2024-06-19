use crate::cartridge::save::Save;
use crate::cartridge::{get_ram_size, Cartridge};

use std::path::Path;

enum Mode {
    Ram,
    Rtc,
}

// struct RealTimeClock {}
pub struct MBC3 {
    rom: Vec<u8>,
    ram: Option<Save>,
    ram_bank: u8,
    rom_bank: u8,
    ram_enabled: bool,
    cgb_flag: u8,
    mode: Mode,
}

impl MBC3 {
    pub fn new(rom: Vec<u8>, path: &Path) -> MBC3 {
        let cartridge_type = rom[0x147];
        let cgb_flag = rom[0x143];
        let has_ram = matches!(cartridge_type, 0x10 | 0x12 | 0x13);
        let ram_size = get_ram_size(&rom);
        let ram = if has_ram {
            ram_size.map(|size| Save::new(path, size))
        } else {
            None
        };
        // let has_timer = match cartridge_type {
        //     0x0F | 0x10 => true,
        //     _ => false,
        // };
        MBC3 {
            rom,
            ram,
            ram_bank: 0,
            rom_bank: 1,
            ram_enabled: false,
            cgb_flag,
            mode: Mode::Ram,
        }
    }
}

impl Cartridge for MBC3 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => {
                let offset = 0x4000 * self.rom_bank as usize;
                self.rom[address as usize - 0x4000 + offset]
            }
            _ => panic!("Address not implemented: {:#06x}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            // 0xA000..=0xBFFF => {
            //     // RTC Register 08-0C
            // }
            0x0000..=0x1FFF => {
                // RAM and Timer Enable
                self.ram_enabled = (value & 0x0F) == 0x0A;
                // TODO: Implement RTC
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number
                // let value = if value == 0 { 1 } else { value };
                self.rom_bank = match value & 0x7F {
                    0x01..=0x7F => value & 0x7F,
                    _ => 0x01,
                };
            }
            0x4000..=0x5FFF => {
                // RAM Bank Number or RTC Register Select
                if value <= 3 {
                    self.mode = Mode::Ram;
                    self.ram_bank = value;
                } else {
                    // RTC Register Select
                    self.mode = Mode::Rtc;
                }
            }
            0x6000..=0x7FFF => {
                // Latch Clock Data
            }
            _ => {}
        }
    }

    fn read_ram(&self, address: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }

        match self.mode {
            Mode::Ram => {
                let offset = 0x2000 * self.ram_bank as usize;

                if let Some(ram) = &self.ram {
                    ram.ram[address as usize - 0xA000 + offset]
                } else {
                    0xFF
                }
            }
            Mode::Rtc => 0,
        }
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }

        match self.mode {
            Mode::Ram => {
                let offset = 0x2000 * self.ram_bank as usize;

                if let Some(ref mut ram) = self.ram {
                    ram.ram[address as usize - 0xA000 + offset] = value;
                }
            }
            Mode::Rtc => {}
        }
    }
    fn get_cgb_flag(&self) -> u8 {
        self.cgb_flag
    }
}
