use super::interrupts::InterruptFlags;
use super::ppu::{Interrupt, PPU};
use super::timer::{Frequency, Timer};

pub const ROM_BANK_0_BEGIN: usize = 0x0000;
pub const ROM_BANK_0_END: usize = 0x3FFF;
pub const ROM_BANK_0_SIZE: usize = ROM_BANK_0_END - ROM_BANK_0_BEGIN + 1;

pub const ROM_BANK_N_BEGIN: usize = 0x4000;
pub const ROM_BANK_N_END: usize = 0x7FFF;
pub const ROM_BANK_N_SIZE: usize = ROM_BANK_N_END - ROM_BANK_N_BEGIN + 1;

pub const VRAM_BEGIN: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;
pub const VRAM_SIZE: u16 = VRAM_END - VRAM_BEGIN + 1;

pub const EXTERNAL_RAM_BEGIN: usize = 0xA000;
pub const EXTERNAL_RAM_END: usize = 0xBFFF;
pub const EXTERNAL_RAM_SIZE: usize = EXTERNAL_RAM_END - EXTERNAL_RAM_BEGIN + 1;

pub const WORKING_RAM_BEGIN: usize = 0xC000;
pub const WORKING_RAM_END: usize = 0xDFFF;
pub const WORKING_RAM_SIZE: usize = WORKING_RAM_END - WORKING_RAM_BEGIN + 1;

pub const ECHO_RAM_BEGIN: usize = 0xE000;
pub const ECHO_RAM_END: usize = 0xFDFF;
pub const ECHO_RAM_SIZE: usize = ECHO_RAM_END - ECHO_RAM_BEGIN + 1;

pub const OAM_BEGIN: u16 = 0xFE00;
pub const OAM_END: u16 = 0xFE9F;
pub const OAM_SIZE: u16 = OAM_END - OAM_BEGIN + 1;

pub const UNUSED_BEGIN: usize = 0xFEA0;
pub const UNUSED_END: usize = 0xFEFF;
pub const UNUSED_SIZE: usize = UNUSED_END - UNUSED_BEGIN + 1;

pub const IO_REGISTERS_BEGIN: usize = 0xFF00;
pub const IO_REGISTERS_END: usize = 0xFF7F;
pub const IO_REGISTERS_SIZE: usize = IO_REGISTERS_END - IO_REGISTERS_BEGIN + 1;

const DIVIDER: u16 = 0xFF04;
const TIMER_COUNTER: u16 = 0xFF05;
const TIMER_MODULO: u16 = 0xFF06;
const TIMER_CONTROL: u16 = 0xFF07;
const INTERRUPT_FLAG: u16 = 0xFF0F;
const LCD_STAT: u16 = 0xFF41;
const INTERRUPT_ENABLE: u16 = 0xFFFF;

pub struct Memory {
    mem: [u8; 0xFFFF],
    pub interrupt_enable: InterruptFlags,
    pub interrupt_flags: InterruptFlags,
    timer: Timer,
    divider: Timer,
    pub ppu: PPU,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            mem: [0; 0xFFFF],
            interrupt_enable: InterruptFlags::new(),
            interrupt_flags: InterruptFlags::new(),
            timer: Timer::new(Frequency::F4096),
            divider: Timer::new(Frequency::F16384),
            ppu: PPU::new(),
        }
    }

    pub fn step(&mut self, cycles: u8) {
        if self.timer.step(cycles) {
            self.interrupt_flags.timer = true;
        }
        self.divider.step(cycles);
        let (vblank, lcd) = match self.ppu.step(cycles) {
            Interrupt::None => (false, false),
            Interrupt::VBlank => (true, false),
            Interrupt::LCDStat => (false, true),
            Interrupt::Both => (true, true),
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
        match address {
            VRAM_BEGIN..=VRAM_END => self.ppu.read_vram(address - VRAM_BEGIN),
            OAM_BEGIN..=OAM_END => self.ppu.read_oam(address - OAM_BEGIN),
            DIVIDER => self.divider.counter,
            INTERRUPT_FLAG => self.interrupt_flags.to_byte(),
            LCD_STAT => self.ppu.status.to_byte(),
            0xFF44 => self.ppu.current_line,
            INTERRUPT_ENABLE => self.interrupt_enable.to_byte(),
            _ => self.mem[address as usize],
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            VRAM_BEGIN..=VRAM_END => {
                self.ppu.write_vram(address - VRAM_BEGIN, value);
            }
            OAM_BEGIN..=OAM_END => {
                self.ppu.write_oam(address - OAM_BEGIN, value);
            }
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
            LCD_STAT => {
                self.ppu.status.from_byte(value);
            }
            INTERRUPT_ENABLE => {
                self.interrupt_enable.from_byte(value);
            }
            _ => self.mem[address as usize] = value,
        }
    }
}
