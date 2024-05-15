mod mbc1;
mod mbc3;
mod rom_only;

use crate::cartridge::mbc1::MBC1;
use crate::cartridge::mbc3::MBC3;
use rom_only::RomOnlyCartridge;

pub struct Cartridge {
    pub rom: Vec<u8>,
    pub ram: Option<Vec<u8>>,
    pub ram_bank: u8,
    pub rom_bank: u8,
    pub mode: u8,
    pub ram_enabled: bool,
}

pub trait CartridgeType {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn read_ram(&self, address: u16) -> u8;
    fn write_ram(&mut self, address: u16, value: u8);
}

pub fn new_cartridge(rom: Vec<u8>) -> Box<dyn CartridgeType> {
    let cartridge_type = rom[0x147];
    println!("Cartridge type: {:#04x}", cartridge_type);
    println!("CGB: {:#04x}", rom[0x143]);
    match cartridge_type {
        0x00 => Box::new(RomOnlyCartridge::new(rom)),
        0x01..=0x03 => Box::new(MBC1::new(rom)),
        0x0F..=0x13 => Box::new(MBC3::new(rom)),
        _ => panic!("Cartridge type not implemented: {:#04x}", cartridge_type),
    }
}

pub fn get_ram_size(rom: &Vec<u8>) -> Option<usize> {
    println!("RAM size: {:#04x}", rom[0x149]);
    match rom[0x149] {
        0x00 => None,
        0x01 => Some(2 * 1024),
        0x02 => Some(8 * 1024),
        0x03 => Some(32 * 1024),
        0x04 => Some(128 * 1024),
        0x05 => Some(64 * 1024),
        _ => None,
    }
}

impl Cartridge {
    pub fn new(rom: Vec<u8>, has_ram: bool, ram_size: Option<usize>) -> Cartridge {
        let ram = if has_ram {
            match ram_size {
                Some(size) => Some(vec![0; size]),
                None => None,
            }
        } else {
            None
        };
        Cartridge {
            rom,
            ram,
            ram_bank: 0,
            rom_bank: 1,
            mode: 0,
            ram_enabled: false,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => {
                let offset = 0x4000 * self.rom_bank as usize;
                self.rom[address as usize - 0x4000 + offset]
            }
            _ => panic!("Address not implemented: {:#06x}", address),
        }
    }

    pub fn read_ram(&self, address: u16) -> u8 {
        if !self.ram_enabled {
            return 0;
        }

        let ram_bank = self.ram_bank;
        let offset = 0x2000 * ram_bank as usize;

        if let Some(ram) = &self.ram {
            ram[address as usize - 0xA000 + offset]
        } else {
            0
        }
    }

    pub fn write_ram(&mut self, address: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }

        let ram_bank = self.ram_bank;
        let offset = 0x2000 * ram_bank as usize;

        if let Some(ref mut ram) = self.ram {
            ram[address as usize - 0xA000 + offset] = value;
        }
    }
}
