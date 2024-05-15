use crate::cartridge::{get_ram_size, Cartridge, CartridgeType};

// struct RealTimeClock {}
pub struct MBC3 {
    cartridge: Cartridge,
}

impl MBC3 {
    pub fn new(rom: Vec<u8>) -> MBC3 {
        let cartridge_type = rom[0x147];
        let has_ram = match cartridge_type {
            0x10 | 0x12 | 0x13 => true,
            _ => false,
        };
        let ram_size = get_ram_size(&rom);
        // let has_timer = match cartridge_type {
        //     0x0F | 0x10 => true,
        //     _ => false,
        // };
        MBC3 {
            cartridge: Cartridge::new(rom, has_ram, ram_size),
        }
    }
}

impl CartridgeType for MBC3 {
    fn read(&self, address: u16) -> u8 {
        self.cartridge.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xA000..=0xBFFF => {
                // RTC Register 08-0C
            }
            0x0000..=0x1FFF => {
                // RAM and Timer Enable
                self.cartridge.ram_enabled = value & 0x0F == 0x0A;
                // TODO: Implement RTC
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number
                let value = if value == 0 { 1 } else { value };
                self.cartridge.rom_bank = value & 0x7F;
            }
            0x4000..=0x5FFF => {
                // RAM Bank Number or RTC Register Select
                if value <= 3 {
                    self.cartridge.ram_bank = value;
                } else {
                    // RTC Register Select
                }
            }
            0x6000..=0x7FFF => {
                // Latch Clock Data
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
