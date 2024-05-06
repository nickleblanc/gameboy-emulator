use crate::cartridge::{get_ram_size, Cartridge, CartridgeType};
pub struct MBC1 {
    cartridge: Cartridge,
}

impl MBC1 {
    pub fn new(rom: Vec<u8>) -> MBC1 {
        let cartridge_type = rom[0x147];
        let has_ram = match cartridge_type {
            0x02 | 0x03 => true,
            _ => false,
        };
        let ram_size = get_ram_size(&rom);
        MBC1 {
            cartridge: Cartridge::new(rom, has_ram, ram_size),
        }
    }
}

impl CartridgeType for MBC1 {
    fn read(&self, address: u16) -> u8 {
        self.cartridge.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                self.cartridge.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x3FFF => {
                let value = if value == 0 { 1 } else { value };
                self.cartridge.rom_bank = (self.cartridge.rom_bank & 0xE0) | (value & 0x1F);
            }
            0x4000..=0x5FFF => match self.cartridge.mode {
                0 => {
                    self.cartridge.rom_bank = (self.cartridge.rom_bank) | ((value & 0x03) << 5);
                }
                1 => {
                    self.cartridge.ram_bank = value;
                }
                _ => {}
            },
            0x6000..=0x7FFF => {
                self.cartridge.mode = value & 0x01;
            }
            _ => {}
        }
    }

    fn read_ram(&self, address: u16) -> u8 {
        self.cartridge.read_ram(address)
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        self.cartridge.write_ram(address, value);
    }
}
