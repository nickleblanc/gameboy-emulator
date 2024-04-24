use crate::mmu::OAM_BEGIN;

const OAM_SIZE: usize = 160;
const VRAM_SIZE: usize = 8192;

struct LCDC {
    display_enabled: bool,
    window_tile_map: bool,
    window_enabled: bool,
    bg_window_tile_data: bool,
    bg_tile_map: bool,
    sprite_size: bool,
    sprite_enabled: bool,
    bg_window_enabled: bool,
}

impl LCDC {
    fn new() -> LCDC {
        LCDC {
            display_enabled: false,
            window_tile_map: false,
            window_enabled: false,
            bg_window_tile_data: false,
            bg_tile_map: false,
            sprite_size: false,
            sprite_enabled: false,
            bg_window_enabled: false,
        }
    }

    pub fn from_byte(&mut self, byte: u8) {
        self.display_enabled = (byte & 0b10000000) != 0;
        self.window_tile_map = (byte & 0b01000000) != 0;
        self.window_enabled = (byte & 0b00100000) != 0;
        self.bg_window_tile_data = (byte & 0b00010000) != 0;
        self.bg_tile_map = (byte & 0b00001000) != 0;
        self.sprite_size = (byte & 0b00000100) != 0;
        self.sprite_enabled = (byte & 0b00000010) != 0;
        self.bg_window_enabled = (byte & 0b00000001) != 0;
    }

    // Add reading and writing to LCDC to mmu.rs

    pub fn to_byte(&self) -> u8 {
        (if self.display_enabled { 0x80 } else { 0 })
            | (if self.window_tile_map { 0x40 } else { 0 })
            | (if self.window_enabled { 0x20 } else { 0 })
            | (if self.bg_window_tile_data { 0x10 } else { 0 })
            | (if self.bg_tile_map { 0x08 } else { 0 })
            | (if self.sprite_size { 0x04 } else { 0 })
            | (if self.sprite_enabled { 0x02 } else { 0 })
            | (if self.bg_window_enabled { 0x01 } else { 0 })
    }
}

#[derive(Copy, Clone)]
struct Sprite {
    y: u8,
    x: u8,
    tile_number: u8,
    y_flip: bool,
    x_flip: bool,
    palette: bool,
    priority: bool,
}

impl Sprite {
    fn new() -> Sprite {
        Sprite {
            y: 0,
            x: 0,
            tile_number: 0,
            y_flip: false,
            x_flip: false,
            palette: false,
            priority: false,
        }
    }
}

#[derive(Copy, Clone)]
enum Mode {
    HBlank,
    VBlank,
    OAM,
    VRAM,
}

// #[derive(Copy, Clone)]
pub struct Status {
    mode: Mode,
    coincidence: bool,
    hblank_interrupt: bool,
    vblank_interrupt: bool,
    oam_interrupt: bool,
    coincidence_interrupt: bool,
}

impl Status {
    fn new() -> Status {
        Status {
            mode: Mode::HBlank,
            coincidence: false,
            hblank_interrupt: false,
            vblank_interrupt: false,
            oam_interrupt: false,
            coincidence_interrupt: false,
        }
    }
    pub fn from_byte(&mut self, byte: u8) {
        // self.coincidence = (byte & 0b100) != 0;
        self.hblank_interrupt = (byte & 0b1000) != 0;
        self.vblank_interrupt = (byte & 0b10000) != 0;
        self.oam_interrupt = (byte & 0b100000) != 0;
        self.coincidence_interrupt = (byte & 0b1000000) != 0;
    }
    pub fn to_byte(&self) -> u8 {
        self.mode as u8
            | if self.coincidence { 0x04 } else { 0 }
            | if self.hblank_interrupt { 0x08 } else { 0 }
            | if self.vblank_interrupt { 0x10 } else { 0 }
            | if self.oam_interrupt { 0x20 } else { 0 }
            | if self.coincidence_interrupt { 0x40 } else { 0 }
    }
}

pub enum Interrupt {
    None,
    VBlank,
    LCDStat,
    Both,
}

impl Interrupt {
    fn add(&mut self, interrupt: Interrupt) {
        match self {
            Interrupt::None => *self = interrupt,
            Interrupt::VBlank => match interrupt {
                Interrupt::LCDStat => *self = Interrupt::Both,
                _ => {}
            },
            Interrupt::LCDStat => match interrupt {
                Interrupt::VBlank => *self = Interrupt::Both,
                _ => {}
            },
            _ => {}
        }
    }
}

pub struct PPU {
    pub screen_buffer: [u8; 160 * 144],
    lcdc: LCDC,
    oam: [u8; OAM_SIZE],
    vram: [u8; VRAM_SIZE],
    sprites: [Sprite; 40],
    pub status: Status,
    cycles: u16,
    pub current_line: u8,
}

impl PPU {
    pub fn new() -> PPU {
        PPU {
            screen_buffer: [0; 160 * 144],
            lcdc: LCDC::new(),
            oam: [0; OAM_SIZE],
            vram: [0; VRAM_SIZE],
            sprites: [Sprite::new(); 40],
            status: Status::new(),
            cycles: 0,
            current_line: 0,
        }
    }

    // TODO: Increment current line and check for LYC=LY interrupt
    pub fn step(&mut self, cycles: u8) -> Interrupt {
        let mut interrupt_request = Interrupt::None;
        self.cycles += cycles as u16;
        match self.status.mode {
            Mode::OAM => {
                if self.cycles >= 80 {
                    self.status.mode = Mode::VRAM;
                    self.cycles = self.cycles % 80;
                }
            }
            Mode::VRAM => {
                if self.cycles >= 172 {
                    self.status.mode = Mode::HBlank;
                    self.cycles = self.cycles % 172;
                    if self.status.hblank_interrupt {
                        interrupt_request.add(Interrupt::LCDStat);
                    }
                    // render
                    self.render_scanline();
                }
            }
            Mode::HBlank => {
                if self.cycles >= 204 {
                    self.current_line += 1;
                    self.cycles = self.cycles % 204;
                    if self.current_line >= 144 {
                        self.status.mode = Mode::VBlank;
                        interrupt_request.add(Interrupt::VBlank);
                        if self.status.vblank_interrupt {
                            interrupt_request.add(Interrupt::LCDStat);
                        }
                    } else {
                        self.status.mode = Mode::OAM;
                        if self.status.oam_interrupt {
                            interrupt_request.add(Interrupt::LCDStat);
                        }
                    }
                }
            }
            Mode::VBlank => {
                if self.cycles >= 456 {
                    self.current_line += 1;
                    self.cycles = self.cycles % 456;
                    if self.current_line > 153 {
                        self.current_line = 0;
                        self.status.mode = Mode::OAM;
                        if self.status.oam_interrupt {
                            interrupt_request.add(Interrupt::LCDStat);
                        }
                    }
                }
            }
        }
        interrupt_request
    }

    pub fn read_vram(&self, address: u16) -> u8 {
        self.vram[address as usize]
    }

    pub fn write_vram(&mut self, address: u16, value: u8) {
        // println!("address: {}, value: {}", address, value);
        self.vram[address as usize] = value;
    }

    pub fn read_oam(&self, address: u16) -> u8 {
        self.oam[address as usize]
    }

    pub fn write_oam(&mut self, address: u16, value: u8) {
        self.oam[address as usize] = value;
    }

    pub fn render_scanline(&mut self) {
        let current_line = self.current_line as i16;
        let sprite_height = if self.lcdc.sprite_size { 16 } else { 8 };

        for sprite in (0..40) {
            let sprite_begin_address = sprite * 4;

            let y = self.read_oam(sprite_begin_address) as i16 - 16;
            let x = self.read_oam(sprite_begin_address + 1) as i16 - 8;

            // let z = self.read_vram(0x8010 - 0x8000);
            // // println!("y: {}, x: {}", y, x);
            // println!("z: {}", z);

            if current_line >= y && current_line < y + sprite_height {
                let tile_number = self.read_oam(sprite_begin_address + 2);
                let y_offset = current_line - y;

                let tile_begin_address = 0x8000 + tile_number as u16 * 16;

                let tile_address = (y_offset * 2) as u16;
                let tile = self.read_vram(tile_address);
                for i in 0..8 {
                    let pixel_x_offset = i as usize;
                    let x_offset = x + i;
                    // let pixel = tile[pixel_x_offset];
                    let canvas_y_offset = current_line as i32 * 160 as i32;
                    let canvas_offset = ((canvas_y_offset + x as i32) * 4) as usize;

                    // let color = if tile & (1 << (7 - i)) != 0 { 255 } else { 0 };
                    let color = 255;

                    self.screen_buffer[canvas_offset] = color as u8;
                    self.screen_buffer[canvas_offset + 1] = color as u8;
                    self.screen_buffer[canvas_offset + 2] = color as u8;
                }
            }
        }
    }
}
