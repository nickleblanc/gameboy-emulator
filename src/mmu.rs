use sdl2::keyboard::Mod;

use super::interrupts::InterruptFlags;
// use super::gpu::{Interrupt, PPU};
use super::gpu::{
    stat::Mode, BackgroundAndWindowDataSelect, InterruptRequest, ObjectSize, TileMap, GPU,
};
use super::timer::{Frequency, Timer};
use crate::cartridge::CartridgeType;
use crate::joypad::Joypad;
use crate::utils::bit;

pub const BOOT_ROM_BEGIN: usize = 0x00;
pub const BOOT_ROM_END: usize = 0xFF;
pub const BOOT_ROM_SIZE: usize = BOOT_ROM_END - BOOT_ROM_BEGIN + 1;

pub const ROM_BANK_0_BEGIN: usize = 0x0000;
pub const ROM_BANK_0_END: usize = 0x3FFF;
pub const ROM_BANK_0_SIZE: usize = ROM_BANK_0_END - ROM_BANK_0_BEGIN + 1;

pub const ROM_BANK_N_BEGIN: usize = 0x4000;
pub const ROM_BANK_N_END: usize = 0x7FFF;
pub const ROM_BANK_N_SIZE: usize = ROM_BANK_N_END - ROM_BANK_N_BEGIN + 1;

pub const VRAM_BEGIN: usize = 0x8000;
pub const VRAM_END: usize = 0x9FFF;
pub const VRAM_SIZE: usize = VRAM_END - VRAM_BEGIN + 1;

pub const EXTERNAL_RAM_BEGIN: usize = 0xA000;
pub const EXTERNAL_RAM_END: usize = 0xBFFF;
pub const EXTERNAL_RAM_SIZE: usize = EXTERNAL_RAM_END - EXTERNAL_RAM_BEGIN + 1;

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

pub struct Memory {
    wram: [u8; WORKING_RAM_SIZE],
    hram: [u8; HIGH_RAM_SIZE],
    pub interrupt_enable: InterruptFlags,
    pub interrupt_flags: InterruptFlags,
    timer: Timer,
    divider: Timer,
    pub gpu: GPU,
    cartridge: Box<dyn CartridgeType>,
    pub joypad: Joypad,
}

impl Memory {
    pub fn new(cartridge: Box<dyn CartridgeType>) -> Memory {
        let mut divider = Timer::new(Frequency::F16384);
        divider.enabled = true;
        Memory {
            wram: [0; WORKING_RAM_SIZE],
            hram: [0; HIGH_RAM_SIZE],
            interrupt_enable: InterruptFlags::new(),
            interrupt_flags: InterruptFlags::new(),
            timer: Timer::new(Frequency::F4096),
            divider,
            gpu: GPU::new(),
            cartridge,
            joypad: Joypad::new(),
        }
    }

    pub fn step(&mut self, cycles: u8) {
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
            ROM_BANK_0_BEGIN..=ROM_BANK_0_END => self.cartridge.read(address as u16),
            ROM_BANK_N_BEGIN..=ROM_BANK_N_END => self.cartridge.read(address as u16),
            VRAM_BEGIN..=VRAM_END => self.gpu.vram[address - VRAM_BEGIN],
            EXTERNAL_RAM_BEGIN..=EXTERNAL_RAM_END => self.cartridge.read_ram(address as u16),
            WORKING_RAM_BEGIN..=WORKING_RAM_END => self.wram[address - WORKING_RAM_BEGIN],
            ECHO_RAM_BEGIN..=ECHO_RAM_END => self.wram[address - ECHO_RAM_BEGIN],
            OAM_BEGIN..=OAM_END => self.gpu.oam[address - OAM_BEGIN],
            IO_REGISTERS_BEGIN..=IO_REGISTERS_END => self.read_io(address),
            HIGH_RAM_BEGIN..=HIGH_RAM_END => self.hram[address - HIGH_RAM_BEGIN],
            INTERRUPT_ENABLE => self.interrupt_enable.to_byte(),
            // _ => self.mem[address],
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
            WORKING_RAM_BEGIN..=WORKING_RAM_END => {
                self.wram[address - WORKING_RAM_BEGIN] = value;
            }
            OAM_BEGIN..=OAM_END => {
                self.gpu.write_oam(address - OAM_BEGIN, value);
            }
            UNUSED_BEGIN..=UNUSED_END => {}
            IO_REGISTERS_BEGIN..=IO_REGISTERS_END => self.write_io(address, value),
            HIGH_RAM_BEGIN..=HIGH_RAM_END => {
                self.hram[address - HIGH_RAM_BEGIN] = value;
            }
            INTERRUPT_ENABLE => {
                // println!("Interrupt enable: {:#04x}", value);
                self.interrupt_enable.from_byte(value);
                // println!(
                //     "Interrupt enable register: {:#04b}",
                //     self.interrupt_enable.to_byte()
                // );
            }
            _ => {
                panic!("Memory write not implemented: {:#06x}", address)
            }
        }
    }

    fn read_io(&self, address: usize) -> u8 {
        match address {
            0xFF00 => self.joypad.read_input(),
            // 0xFF00 => 0xCF,
            // 0xFF01 => self.mem[address],
            // 0xFF02 => self.mem[address],
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
            INTERRUPT_FLAG => self.interrupt_flags.to_byte(),
            0xFF40 => self.gpu.lcdc.to_byte(),
            LCD_STAT => self.gpu.stat.to_byte(),
            0xFF42 => self.gpu.scroll_y,
            0xFF43 => self.gpu.scroll_x,
            0xFF44 => self.gpu.line,
            0xFF45 => self.gpu.line_check,
            0xFF47 => self.gpu.get_bg_palette(),
            0xFF48 => self.gpu.get_object_palette0(),
            0xFF49 => self.gpu.get_object_palette1(),
            0xFF4A => self.gpu.window_y,
            0xFF4B => self.gpu.window_x,
            // _ => panic!("IO register not implemented: {:#06x}", address),
            _ => 0,
        }
    }

    fn write_io(&mut self, address: usize, value: u8) {
        match address {
            0xFF00 => self.joypad.write(value),
            // 0xFF01 => self.mem[address] = value,
            // 0xFF02 => self.mem[address] = value,
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
            0xFF40 => {
                self.gpu.lcdc.from_byte(value);
                if value & 0x80 == 0 {
                    self.gpu.stat.mode = Mode::HorizontalBlank;
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
                println!("BG Palette 0: {:#04x}", value);
                self.gpu.set_bg_palette(value);
            }
            0xFF48 => {
                println!("Palette 0: {:#04x}", value);
                self.gpu.set_object_palette0(value);
            }
            0xFF49 => {
                println!("Palette 1: {:#04x}", value);
                self.gpu.set_object_palette1(value);
            }
            0xFF4A => {
                self.gpu.window_y = value;
            }
            0xFF4B => {
                self.gpu.window_x = value;
            }
            _ => {
                // panic!("IO register not implemented: {:#06x}", address);
            }
        }
    }
}
