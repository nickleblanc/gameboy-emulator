use std::f32::consts::E;

use crate::cartridge::CartridgeType;
use crate::gpu::{stat::Mode, GameBoyMode, InterruptRequest, GPU};
use crate::interrupts::InterruptFlags;
use crate::joypad::Joypad;
use crate::timer::{Frequency, Timer};

pub const ROM_BANK_0_BEGIN: usize = 0x0000;
pub const ROM_BANK_0_END: usize = 0x3FFF;

pub const ROM_BANK_N_BEGIN: usize = 0x4000;
pub const ROM_BANK_N_END: usize = 0x7FFF;

pub const VRAM_BEGIN: usize = 0x8000;
pub const VRAM_END: usize = 0x9FFF;
pub const VRAM_SIZE: usize = VRAM_END - VRAM_BEGIN + 1;

pub const EXTERNAL_RAM_BEGIN: usize = 0xA000;
pub const EXTERNAL_RAM_END: usize = 0xBFFF;

pub const WORKING_RAM_BEGIN: usize = 0xC000;
pub const WORKING_RAM_END: usize = 0xDFFF;
pub const WORKING_RAM_SIZE: usize = WORKING_RAM_END - WORKING_RAM_BEGIN + 1;

pub const ECHO_RAM_BEGIN: usize = 0xE000;
pub const ECHO_RAM_END: usize = 0xFDFF;

pub const OAM_BEGIN: usize = 0xFE00;
pub const OAM_END: usize = 0xFE9F;
pub const OAM_SIZE: usize = OAM_END - OAM_BEGIN + 1;

pub const UNUSED_BEGIN: usize = 0xFEA0;
pub const UNUSED_END: usize = 0xFEFF;

pub const IO_REGISTERS_BEGIN: usize = 0xFF00;
pub const IO_REGISTERS_END: usize = 0xFF7F;

pub const HIGH_RAM_BEGIN: usize = 0xFF80;
pub const HIGH_RAM_END: usize = 0xFFFE;
pub const HIGH_RAM_SIZE: usize = HIGH_RAM_END - HIGH_RAM_BEGIN + 1;

const DIVIDER: usize = 0xFF04;
const TIMER_COUNTER: usize = 0xFF05;
const TIMER_MODULO: usize = 0xFF06;
const TIMER_CONTROL: usize = 0xFF07;
const INTERRUPT_FLAG: usize = 0xFF0F;
const LCD_STAT: usize = 0xFF41;
const INTERRUPT_ENABLE: usize = 0xFFFF;

#[derive(Debug, PartialEq)]
enum DmaMode {
    GDMA,
    HDMA,
}

pub struct Memory {
    wram0: [u8; 4096],
    wramn: [[u8; 4096]; 7],
    hram: [u8; HIGH_RAM_SIZE],
    pub interrupt_enable: InterruptFlags,
    pub interrupt_flags: InterruptFlags,
    timer: Timer,
    divider: Timer,
    pub gpu: GPU,
    cartridge: Box<dyn CartridgeType>,
    pub joypad: Joypad,
    key0: u8,
    wram_bank: u8,
    boot_rom: Vec<u8>,
    pub boot_active: bool,
    dma_source: u16,
    dma_destination: u16,
    dma_length: u16,
    pub dma_mode: DmaMode,
}

impl Memory {
    pub fn new(cartridge: Box<dyn CartridgeType>, boot: Option<Vec<u8>>) -> Memory {
        let mut divider = Timer::new(Frequency::F16384);
        divider.enabled = true;

        let boot_active;
        let boot_rom;

        let gb_mode;

        match boot {
            Some(boot) => {
                boot_rom = boot;
                boot_active = true;
                gb_mode = GameBoyMode::CGB;
            }
            None => {
                boot_rom = vec![];
                boot_active = false;
                gb_mode = match cartridge.get_cgb_flag() {
                    0x80 | 0xC0 => GameBoyMode::CGB,
                    _ => GameBoyMode::DMG,
                };
            }
        }

        Memory {
            wram0: [0; 4096],
            wramn: [[0; 4096]; 7],
            hram: [0; HIGH_RAM_SIZE],
            interrupt_enable: InterruptFlags::new(),
            interrupt_flags: InterruptFlags::new(),
            timer: Timer::new(Frequency::F4096),
            divider,
            gpu: GPU::new(gb_mode, boot_active),
            cartridge,
            joypad: Joypad::new(),
            key0: 0,
            wram_bank: 1,
            boot_rom,
            boot_active,
            dma_source: 0,
            dma_destination: 0,
            dma_length: 0,
            dma_mode: DmaMode::GDMA,
        }
    }

    pub fn step(&mut self, cycles: u8) {
        // if self.dma_mode == DmaMode::HDMA {
        //     // self.hdma_step();
        //     panic!("HDMA not implemented");
        // } else {
        //     self.gdma_step();
        // }
        self.gdma_step();
        if self.timer.step(cycles) {
            self.interrupt_flags.timer = true;
        }
        self.divider.step(cycles);
        let (vblank, lcd) = match self.gpu.step(cycles) {
            InterruptRequest::None => (false, false),
            InterruptRequest::VBlank => (true, false),
            InterruptRequest::LCDStat => (false, true),
            InterruptRequest::Both => (true, true),
        };

        if vblank {
            self.interrupt_flags.vblank = true;
        }

        if lcd {
            self.interrupt_flags.lcd_stat = true;
        }
    }

    pub fn gdma_step(&mut self) {
        if self.dma_length == 0 {
            return;
        }
        println!("DMA Step: {:#04x}", self.dma_length);

        let source = self.dma_source;
        let destination = self.dma_destination;

        for i in 0..(self.dma_length * 16) {
            let byte = self.read_byte(source + i as u16);
            // println!(
            //     "DMA: {:#06x} -> {:#06x} = {:#04x}",
            //     source + i as u16,
            //     destination,
            //     byte
            // );
            // println!(
            //     "Destination: {:#06x}",
            //     0x8000 | ((destination & 0x1FFF) + i as u16) as u16
            // );
            self.write_byte(0x8000 | (destination & 0x1FFF) + i as u16, byte);
        }

        self.dma_length = 0;
    }

    pub fn hdma_step(&mut self) {
        println!("HDMA Step");
        if self.dma_length == 0 {
            return;
        }

        let source = self.dma_source;
        let destination = self.dma_destination;

        for i in 0..16 {
            let byte = self.read_byte(source + i as u16);
            self.write_byte(0x8000 | (destination & 0x1FFF) + i as u16, byte);
        }
        self.dma_length -= 1;
    }

    pub fn interrupt_called(&mut self) -> bool {
        (self.interrupt_enable.joypad && self.interrupt_flags.joypad)
            || (self.interrupt_enable.lcd_stat && self.interrupt_flags.lcd_stat)
            || (self.interrupt_enable.serial && self.interrupt_flags.serial)
            || (self.interrupt_enable.timer && self.interrupt_flags.timer)
            || (self.interrupt_enable.vblank && self.interrupt_flags.vblank)
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        let address = address as usize;
        match address {
            ROM_BANK_0_BEGIN..=ROM_BANK_0_END => {
                if self.boot_active && address <= 0xFF {
                    return self.boot_rom[address];
                }
                if self.boot_active && address >= 0x200 && address <= 0x8FF {
                    return self.boot_rom[address];
                }
                self.cartridge.read(address as u16)
            }
            ROM_BANK_N_BEGIN..=ROM_BANK_N_END => self.cartridge.read(address as u16),
            VRAM_BEGIN..=VRAM_END => self.gpu.read_vram(address - VRAM_BEGIN),
            EXTERNAL_RAM_BEGIN..=EXTERNAL_RAM_END => self.cartridge.read_ram(address as u16),
            WORKING_RAM_BEGIN..=0xCFFF => {
                // println!("WRAM Bank: {:#04x}", self.wram_bank);
                self.wram0[address - WORKING_RAM_BEGIN]
            }
            0xD000..=0xDFFF => self.wramn[self.wram_bank as usize - 1][address - 0xD000],
            // 0xD000..=0xDFFF => 0x00,
            // ECHO_RAM_BEGIN..=0xEFFF => self.wram0[address - ECHO_RAM_BEGIN],
            // 0xF000..=0xFDFF => self.wramn[self.wram_bank as usize - 1][address - 0xF000],
            OAM_BEGIN..=OAM_END => self.gpu.oam[address - OAM_BEGIN],
            IO_REGISTERS_BEGIN..=IO_REGISTERS_END => self.read_io(address),
            HIGH_RAM_BEGIN..=HIGH_RAM_END => self.hram[address - HIGH_RAM_BEGIN],
            INTERRUPT_ENABLE => self.interrupt_enable.to_byte(),
            _ => 0,
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        let address = address as usize;
        match address {
            ROM_BANK_0_BEGIN..=ROM_BANK_0_END => {
                self.cartridge.write(address as u16, value);
            }
            ROM_BANK_N_BEGIN..=ROM_BANK_N_END => {
                self.cartridge.write(address as u16, value);
            }
            VRAM_BEGIN..=VRAM_END => {
                self.gpu
                    .write_vram(address as usize - VRAM_BEGIN as usize, value);
            }
            EXTERNAL_RAM_BEGIN..=EXTERNAL_RAM_END => {
                self.cartridge.write_ram(address as u16, value);
            }
            WORKING_RAM_BEGIN..=0xCFFF => {
                self.wram0[address - WORKING_RAM_BEGIN] = value;
            }
            0xD000..=0xDFFF => {
                self.wramn[self.wram_bank as usize - 1][address - 0xD000] = value;
            }
            // ECHO_RAM_BEGIN..=0xEFFF => {
            //     self.wram0[address - ECHO_RAM_BEGIN] = value;
            // }
            // 0xF000..=0xFDFF => {
            //     self.wramn[self.wram_bank as usize - 1][address - 0xF000] = value;
            // }
            OAM_BEGIN..=OAM_END => {
                self.gpu.write_oam(address - OAM_BEGIN, value);
            }
            UNUSED_BEGIN..=UNUSED_END => {}
            IO_REGISTERS_BEGIN..=IO_REGISTERS_END => self.write_io(address, value),
            HIGH_RAM_BEGIN..=HIGH_RAM_END => {
                self.hram[address - HIGH_RAM_BEGIN] = value;
            }
            INTERRUPT_ENABLE => {
                self.interrupt_enable.from_byte(value);
            }
            _ => {
                panic!("Memory write not implemented: {:#06x}", address)
            }
        }
    }

    fn read_io(&self, address: usize) -> u8 {
        match address {
            0xFF00 => {
                if self.joypad.read_input() == 0x07 {
                    // println!("Read Joypad: {:#04x}", self.joypad.read_input());
                }
                self.joypad.read_input()
            }
            // 0xFF00 => 0xCF,
            // 0xFF01 => self.mem[address],
            // 0xFF02 => self.mem[address],
            0xFF01..=0xFF02 => 0,
            DIVIDER => self.divider.counter,
            TIMER_COUNTER => self.timer.counter,
            TIMER_MODULO => self.timer.modulo,
            TIMER_CONTROL => {
                let enabled = if self.timer.enabled { 0b100 } else { 0 };
                let frequency = match self.timer.frequency {
                    Frequency::F4096 => 0b00,
                    Frequency::F262144 => 0b01,
                    Frequency::F65536 => 0b10,
                    Frequency::F16384 => 0b11,
                };
                enabled | frequency
            }
            0xFF08 => 0,
            INTERRUPT_FLAG => self.interrupt_flags.to_byte(),
            0xFF10..=0xFF3F => {
                // println!("Sound register not implemented: {:#06x}", address);
                0
            }
            0xFF40 => self.gpu.lcdc.to_byte(),
            LCD_STAT => self.gpu.stat.to_byte(),
            0xFF42 => self.gpu.scroll_y,
            0xFF43 => self.gpu.scroll_x,
            0xFF44 => self.gpu.line,
            0xFF45 => self.gpu.line_check,
            // 0xFF46 => 0,
            0xFF47 => {
                // self.gpu.get_bg_palette()
                // println!("BG Palette: {:#04x}", self.gpu.get_bg_palette());
                self.gpu.palettes[0]
                // 0xFC
            }
            0xFF48 => {
                // self.gpu.get_object_palette0()
                self.gpu.palettes[1]
                // 0xFF
            }
            0xFF49 => {
                // self.gpu.get_object_palette1()
                self.gpu.palettes[2]
                // 0xFF
            }
            0xFF4A => self.gpu.window_y,
            0xFF4B => self.gpu.window_x,
            0xFF4C => {
                println!("Key0 Read: {:#04x}", self.key0);
                self.key0
            }
            0xFF4D => {
                println!("Speed: {:#04x}", self.gpu.speed);
                self.gpu.speed
            }
            0xFF4F => {
                // println!("VRAM Bank: {:#04x}", self.gpu.vram_bank);
                return self.gpu.vram_bank | 0xFE;
            }
            0xFF55 => {
                // println!("DMA Length: {:#04x}", self.dma_length);
                self.dma_length as u8
            }
            0xFF68 => {
                self.gpu.bgpi
                    | if self.gpu.auto_increment_bg {
                        0x80
                    } else {
                        0x00
                    }
            }
            0xFF69 => {
                println!("BGPD: {:#04x}", self.gpu.bgpi);
                self.gpu.bg_palette[self.gpu.bgpi as usize]
            }
            0xFF6A => {
                self.gpu.obpi
                    | if self.gpu.auto_increment_object {
                        0x80
                    } else {
                        0x00
                    }
            }
            0xFF6B => {
                println!("OBPD: {:#04x}", self.gpu.obpi);
                self.gpu.object_palette[self.gpu.obpi as usize]
            }
            0xFF70 => self.wram_bank,
            0xFF7E => 0,
            // _ => panic!("IO register not implemented: {:#06x}", address),
            _ => 0,
        }
    }

    fn write_io(&mut self, address: usize, value: u8) {
        match address {
            0xFF00 => self.joypad.write(value),
            0xFF01 => {}
            0xFF02 => {}
            DIVIDER => {
                self.divider.counter = 0;
            }
            TIMER_COUNTER => {
                self.timer.counter = value;
            }
            TIMER_MODULO => {
                self.timer.modulo = value;
            }
            TIMER_CONTROL => {
                self.timer.enabled = (value & 0b100) != 0;
                self.timer.frequency = match value & 0b11 {
                    0b00 => Frequency::F4096,
                    0b01 => Frequency::F262144,
                    0b10 => Frequency::F65536,
                    _ => Frequency::F16384,
                };
            }
            INTERRUPT_FLAG => {
                self.interrupt_flags.from_byte(value);
            }
            0xFF10..=0xFF3F => {
                // println!("Sound register not implemented: {:#06x}", address);
            }
            0xFF40 => {
                self.gpu.lcdc.from_byte(value);
                if value & 0x80 == 0 {
                    self.gpu.stat.mode = Mode::HorizontalBlank;
                    // self.gpu.stat.reset();
                    // self.gpu.line = 0;
                    // self.gpu.cycles = 0;
                    // self.gpu.wly = 0;
                }
            }
            LCD_STAT => {
                self.gpu.stat.from_byte(value);
            }
            0xFF42 => {
                // Viewport Y Offset
                self.gpu.scroll_y = value;
            }
            0xFF43 => {
                // Viewport X Offset
                self.gpu.scroll_x = value;
            }
            0xFF45 => {
                self.gpu.line_check = value;
            }
            0xFF46 => {
                let start = (value as u16) << 8;
                for i in 0..160 {
                    let byte = self.read_byte(start + i);
                    self.write_byte(0xFE00 + i, byte);
                }
            }
            0xFF47 => {
                self.gpu.set_bg_palette(value);
            }
            0xFF48 => {
                // self.gpu.set_object_palette0(value);
                self.gpu.set_dmg_object_palette(value, 0)
            }
            0xFF49 => {
                // println!("Object Palette 1: {:#04x}", value);
                // self.gpu.set_object_palette1(value);
                self.gpu.set_dmg_object_palette(value, 1)
            }
            0xFF4A => {
                self.gpu.window_y = value;
            }
            0xFF4B => {
                self.gpu.window_x = value;
            }
            0xFF4C => {
                println!("Key0 Write: {:#04x}", value);
                println!("mode: {:?}", self.gpu.gb_mode);
                self.gpu.gb_mode = if value == 0x80 || value == 0xC0 {
                    GameBoyMode::CGB
                } else {
                    GameBoyMode::DMG
                };

                println!("mode: {:?}", self.gpu.gb_mode);

                self.key0 = value;
            }
            0xFF4F => {
                self.gpu.vram_bank = value & 0x01;
                // println!("VRAM Bank: {:#04x}", value);
            }
            0xFF4D => {
                // self.gpu.speed = value;
                println!("Speed: {:#04x}", value)
            }
            0xFF50 => {
                // println!("Boot ROM disabled");
                self.boot_active = false;
            }
            0xFF51 => {
                self.dma_source = (self.dma_source & 0x00FF) | ((value as u16) << 8);
                // println!("HDMA1: {:#04x}", value)
            }
            0xFF52 => {
                self.dma_source = (self.dma_source & 0xFF00) | (value as u16 & 0xF0);
                // println!("HDMA2: {:#04x}", value)
            }
            0xFF53 => {
                self.dma_destination = (self.dma_destination & 0x00FF) | (value as u16) << 8;
                // println!("HDMA3: {:#04x}", value)
            }
            0xFF54 => {
                self.dma_destination = (self.dma_destination & 0xFF00) | (value as u16 & 0xF0);
                // println!("HDMA4: {:#04x}", value)
            }
            0xFF55 => {
                // self.dma_length = (((value & 0x7F) + 1) as u16) << 4;
                self.dma_mode = if value & 0x80 != 0 {
                    DmaMode::HDMA
                } else {
                    DmaMode::GDMA
                };
                // println!("DMA Mode: {:?}", self.dma_mode);
                self.dma_length = (value & 0x7F) as u16;
                // println!("HDMA5: {:#04x}", value)
            }
            0xFF56 => {}
            0xFF68 => {
                self.gpu.bgpi = value & 0x3f;
                self.gpu.auto_increment_bg = value & 0x80 != 0;
                // println!("BGPI: {:#04x}", value)
            }
            0xFF69 => {
                self.gpu.set_cgb_bg_palette(value);
                // println!("BGPD: {:#04x}", value)
            }
            0xFF6A => {
                self.gpu.obpi = value & 0x3f;
                self.gpu.auto_increment_object = value & 0x80 != 0;
                // println!("OBPI: {:#04x}", value)
            }
            0xFF6B => {
                self.gpu.set_cgb_object_palette(value);
                // println!("OBPD: {:#04x}", value)
            }
            0xFF6C => {}
            0xFF70 => {
                if value == 0 {
                    self.wram_bank = 1;
                } else {
                    self.wram_bank = value & 0x07;
                }
                // println!("WRAM Bank: {:#04x}", value);
            }
            _ => {
                // panic!("IO register not implemented: {:#06x}", address);
            }
        }
    }
}
