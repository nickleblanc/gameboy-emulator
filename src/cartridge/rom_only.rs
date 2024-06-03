use crate::cartridge::{Cartridge, CartridgeType};

pub struct RomOnlyCartridge {
    cartridge: Cartridge,
}

impl RomOnlyCartridge {
    pub fn new(rom: Vec<u8>) -> RomOnlyCartridge {
        RomOnlyCartridge {
            cartridge: Cartridge::new(rom, false, None),
        }
    }
}

impl CartridgeType for RomOnlyCartridge {
    fn set_sram(&mut self, _sram: Vec<u8>) {
        return;
    }

    fn read(&self, address: u16) -> u8 {
        self.cartridge.read(address)
    }

    fn write(&mut self, _address: u16, _value: u8) {
        return;
    }

    fn read_ram(&self, _address: u16) -> u8 {
        return 0;
    }

    fn write_ram(&mut self, _address: u16, _value: u8) {
        return;
    }
    fn get_cgb_flag(&self) -> u8 {
        self.cartridge.get_cgb_flag()
    }
}
