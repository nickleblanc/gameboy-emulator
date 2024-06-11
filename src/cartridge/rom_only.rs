use crate::cartridge::Cartridge;

pub struct RomOnlyCartridge {
    rom: Vec<u8>,
    cgb_flag: u8,
}

impl RomOnlyCartridge {
    pub fn new(rom: Vec<u8>) -> RomOnlyCartridge {
        let cgb_flag = rom[0x143];
        RomOnlyCartridge { rom, cgb_flag }
    }
}

impl Cartridge for RomOnlyCartridge {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.rom[address as usize],
            _ => panic!("Address not implemented: {:#06x}", address),
        }
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
        self.cgb_flag
    }
}
