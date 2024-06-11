use crate::cartridge::save::Save;
use crate::cartridge::{get_ram_size, Cartridge};

use std::path::Path;
pub struct MBC1 {
    rom: Vec<u8>,
    ram: Option<Save>,
    ram_bank: u8,
    rom_bank: u8,
    ram_enabled: bool,
    cgb_flag: u8,
    mode: u8,
    rom_size: u8,
    ram_size: usize,
}

impl MBC1 {
    pub fn new(rom: Vec<u8>, path: &Path) -> MBC1 {
        let cartridge_type = rom[0x147];
        let rom_size = rom[0x148];
        let cgb_flag = rom[0x143];
        let has_ram = match cartridge_type {
            0x02 | 0x03 => true,
            _ => false,
        };
        let ram_size = get_ram_size(&rom);
        let ram = if has_ram {
            match ram_size {
                Some(size) => Some(Save::new(path, size)),
                None => None,
            }
        } else {
            None
        };
        MBC1 {
            rom,
            ram,
            ram_bank: 0,
            rom_bank: 1,
            ram_enabled: false,
            cgb_flag,
            rom_size,
            mode: 0,
            ram_size: ram_size.unwrap_or(0),
        }
    }
}

impl Cartridge for MBC1 {
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
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x3FFF => {
                self.rom_bank = match value & 0x1F {
                    0 => 1,
                    _ => value & 0x1F,
                };
                // Need to finish implementing bit-mask for rom bank, see https://gbdev.io/pandocs/MBC1.html#20003fff--rom-bank-number-write-only
            }
            0x4000..=0x5FFF => match self.mode {
                0 => {
                    if self.rom_size >= 0x05 {
                        self.rom_bank = self.rom_bank | ((value & 0x03) << 5);
                    }
                }
                1 => {
                    if self.ram_size > 0x2000 {
                        self.ram_bank = value & 0x03;
                    }
                }
                _ => {}
            },
            0x6000..=0x7FFF => {
                self.mode = value & 0x01;
            }
            _ => {}
        }
    }

    fn read_ram(&self, address: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }

        let bank = match self.mode {
            0 => 0,
            _ => self.ram_bank,
        };

        let offset = 0x2000 * bank as usize;

        if let Some(ram) = &self.ram {
            ram.ram[address as usize - 0xA000 + offset]
        } else {
            0
        }
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }

        let bank = match self.mode {
            0 => 0,
            _ => self.ram_bank,
        };

        let offset = 0x2000 * bank as usize;

        if let Some(ref mut ram) = self.ram {
            ram.ram[address as usize - 0xA000 + offset] = value;
        }
    }

    fn get_cgb_flag(&self) -> u8 {
        self.cgb_flag
    }
}
