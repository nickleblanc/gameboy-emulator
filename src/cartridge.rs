mod mbc1;
mod mbc3;
mod rom_only;
mod save;

use std::fs;
use std::path::Path;

use crate::cartridge::mbc1::MBC1;
use crate::cartridge::mbc3::MBC3;
use rom_only::RomOnlyCartridge;

pub trait Cartridge {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn read_ram(&self, address: u16) -> u8;
    fn write_ram(&mut self, address: u16, value: u8);
    fn get_cgb_flag(&self) -> u8;
}

pub fn new_cartridge(path: &Path) -> Box<dyn Cartridge> {
    let rom = fs::read(path).expect("failed to open rom file");
    let cartridge_type = rom[0x147];
    println!("Cartridge type: {:#04x}", cartridge_type);
    println!("CGB: {:#04x}", rom[0x143]);
    match cartridge_type {
        0x00 => Box::new(RomOnlyCartridge::new(rom)),
        0x01..=0x03 => Box::new(MBC1::new(rom, path)),
        0x0F..=0x13 => Box::new(MBC3::new(rom, path)),
        _ => panic!("Cartridge type not implemented: {:#04x}", cartridge_type),
    }
}

pub fn get_ram_size(rom: &[u8]) -> Option<usize> {
    println!("RAM size: {:#04x}", rom[0x149]);
    match rom[0x149] {
        0x01 => Some(2 * 1024),
        0x02 => Some(8 * 1024),
        0x03 => Some(32 * 1024),
        0x04 => Some(128 * 1024),
        0x05 => Some(64 * 1024),
        _ => None,
    }
}
