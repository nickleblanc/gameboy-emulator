mod lcdc;
pub mod stat;

use lcdc::Lcdc;
use stat::{Mode, Stat};

use crate::mmu::{OAM_SIZE, VRAM_BEGIN, VRAM_SIZE};

const TILESET_FIRST_BEGIN_ADDRESS: u16 = 0x8000;
const TILESET_SECOND_BEGIN_ADDRESS: u16 = 0x9000;

const NUMBER_OF_OBJECTS: usize = 40;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Color {
    White = 255,
    LightGray = 170,
    DarkGray = 85,
    Black = 0,
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Pixel {
    r: u8,
    g: u8,
    b: u8,
}

impl Default for Pixel {
    fn default() -> Self {
        Pixel {
            r: 255,
            g: 255,
            b: 255,
        }
    }
}

impl std::convert::From<Color> for Pixel {
    fn from(color: Color) -> Self {
        Pixel {
            r: color as u8,
            g: color as u8,
            b: color as u8,
        }
    }
}

impl std::convert::From<u8> for Color {
    fn from(n: u8) -> Self {
        match n {
            0 => Color::White,
            1 => Color::LightGray,
            2 => Color::DarkGray,
            3 => Color::Black,
            _ => panic!("Cannot convert {} to color", n),
        }
    }
}

pub struct ObjectData {
    x: i16,
    y: i16,
    tile: u8,
    palette: ObjectPalette,
    xflip: bool,
    yflip: bool,
    priority: bool,
    cgb_palette: u8,
    bank: bool,
}

impl Default for ObjectData {
    fn default() -> Self {
        ObjectData {
            x: -16,
            y: -8,
            tile: Default::default(),
            palette: Default::default(),
            xflip: Default::default(),
            yflip: Default::default(),
            priority: Default::default(),
            cgb_palette: Default::default(),
            bank: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct BackgroundPriority {
    priority: bool,
    color: PriorityFlag,
}

impl Default for BackgroundPriority {
    fn default() -> Self {
        BackgroundPriority {
            priority: false,
            color: PriorityFlag::None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
enum ObjectPalette {
    #[default]
    Zero,
    One,
}

pub enum Interrupt {
    VBlank = 0x01,
    LCDStat = 0x02,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum PriorityFlag {
    None,
    Color0,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GameBoyMode {
    Dmg,
    Cgb,
}

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

pub struct Gpu {
    pub canvas_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
    pub vram: [u8; VRAM_SIZE],
    pub vram1: [u8; VRAM_SIZE],
    pub oam: [u8; OAM_SIZE],
    pub line_check: u8,
    pub line: u8,
    pub cycles: u16,
    pub window_x: u8,
    pub window_y: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub lcdc: Lcdc,
    pub stat: Stat,
    pub wly: u8,
    bg_priority_map: [BackgroundPriority; 65536],
    pub palettes: [u8; 3],
    palette_bg: [Pixel; 4],
    dmg_object_palettes: [[Pixel; 4]; 2],
    bg_map_attributes0: [u8; 1024],
    bg_map_attributes1: [u8; 1024],
    pub bgpi: u8,
    pub obpi: u8,
    pub bg_palette: [u8; 64],
    palettes_bg: [[Pixel; 4]; 8],
    pub object_palette: [u8; 64],
    palettes_object: [[Pixel; 4]; 8],
    pub auto_increment_bg: bool,
    pub auto_increment_object: bool,
    pub vram_bank: u8,
    pub speed: u8,
    pub gb_mode: GameBoyMode,
    boot_rom: bool,
    pub interrupts_fired: u8,
}

impl Gpu {
    pub fn new(gb_mode: GameBoyMode, boot_rom: bool) -> Gpu {
        Gpu {
            canvas_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
            vram: [0; VRAM_SIZE],
            vram1: [0; VRAM_SIZE],
            oam: [0; OAM_SIZE],
            line_check: 0,
            line: 0,
            cycles: 0,
            window_x: 0,
            window_y: 0,
            scroll_x: 0,
            scroll_y: 0,
            lcdc: Lcdc::new(),
            stat: Stat::new(),
            wly: 0,
            bg_priority_map: [Default::default(); 65536],
            palettes: [0; 3],
            palette_bg: [Default::default(); 4],
            dmg_object_palettes: [[Default::default(); 4]; 2],
            bg_map_attributes0: [0; 1024],
            bg_map_attributes1: [0; 1024],
            bgpi: 0,
            obpi: 0,
            bg_palette: [0x00; 64],
            palettes_bg: [[Default::default(); 4]; 8],
            object_palette: [0x00; 64],
            palettes_object: [[Default::default(); 4]; 8],
            auto_increment_bg: false,
            auto_increment_object: false,
            vram_bank: 0,
            speed: 0x00,
            gb_mode,
            boot_rom,
            interrupts_fired: 0,
        }
    }

    pub fn write_vram(&mut self, index: usize, value: u8) {
        if self.vram_bank == 1 {
            match index {
                0x0000..=0x17FF => {
                    self.vram1[index] = value;
                }
                0x1800..=0x1BFF => {
                    let addr = index - 0x1800;
                    self.bg_map_attributes0[addr] = value;
                }
                0x1C00..=0x1FFF => {
                    let addr = index - 0x1C00;
                    self.bg_map_attributes1[addr] = value;
                }
                _ => panic!("Invalid VRAM address: {:#04x}", index),
            }
        } else {
            self.vram[index] = value;
        }
    }

    pub fn read_vram(&self, index: usize) -> u8 {
        if self.vram_bank == 1 {
            match index {
                0x0000..=0x17FF => self.vram1[index],
                0x1800..=0x1BFF => self.bg_map_attributes0[index - 0x1800],
                0x1C00..=0x1FFF => self.bg_map_attributes1[index - 0x1C00],
                _ => panic!("Invalid VRAM address: {:#04x}", index),
            }
        } else {
            self.vram[index]
        }
    }

    pub fn write_oam(&mut self, index: usize, value: u8) {
        self.oam[index] = value;
    }

    pub fn set_bg_palette(&mut self, value: u8) {
        self.palettes[0] = value;
        self.palette_bg = [
            Color::from(value & 0b11).into(),
            Color::from((value >> 2) & 0b11).into(),
            Color::from((value >> 4) & 0b11).into(),
            Color::from(value >> 6).into(),
        ];
    }

    pub fn set_dmg_object_palette(&mut self, value: u8, index: usize) {
        self.palettes[index + 1] = value;
        self.dmg_object_palettes[index] = [
            Color::from(value & 0b11).into(),
            Color::from((value >> 2) & 0b11).into(),
            Color::from((value >> 4) & 0b11).into(),
            Color::from(value >> 6).into(),
        ]
    }

    pub fn set_cgb_bg_palette(&mut self, value: u8) {
        self.bg_palette[self.bgpi as usize] = value;

        let palette_number = self.bgpi / 8;
        let color_index = (self.bgpi as usize % 8) / 2;

        let palette = &mut self.palettes_bg[palette_number as usize];

        let palette_offset = (palette_number * 8) as usize;
        let color_offset = color_index * 2;

        let color = rgb555_to_rgb888(
            self.bg_palette[palette_offset + color_offset],
            self.bg_palette[palette_offset + color_offset + 1],
        );
        palette[color_index] = color;

        if self.auto_increment_bg {
            self.bgpi = (self.bgpi + 1) & 0x3F;
        }
    }

    pub fn set_cgb_object_palette(&mut self, value: u8) {
        self.object_palette[self.obpi as usize] = value;

        let palette_number = self.obpi as usize / 8;

        let palette = &mut self.palettes_object[palette_number];
        let color_index = (self.obpi as usize % 8) / 2;

        let palette_offset = palette_number * 8;
        let color_offset = color_index * 2;

        let color = rgb555_to_rgb888(
            self.object_palette[palette_offset + color_offset],
            self.object_palette[palette_offset + color_offset + 1],
        );
        palette[color_index] = color;

        if self.auto_increment_object {
            self.obpi = (self.obpi + 1) & 0x3F;
        }
    }

    pub fn step(&mut self, cycles: u8) {
        if !self.lcdc.display_enabled {
            return;
        }
        self.cycles += cycles as u16;

        match self.stat.mode {
            Mode::HorizontalBlank => {
                if self.cycles >= 204 {
                    self.cycles %= 204;

                    if self.line >= 143 {
                        self.set_mode(Mode::VerticalBlank);
                        self.fire_interrupt(Interrupt::VBlank);
                        self.bg_priority_map = [Default::default(); 65536];
                    } else {
                        self.set_current_line(self.line + 1);
                        self.set_mode(Mode::OAMAccess)
                    }
                    // self.set_equal_lines_check(&mut request);
                }
            }
            Mode::VerticalBlank => {
                if self.cycles >= 456 {
                    self.cycles %= 456;
                    self.set_current_line(self.line + 1);
                    if self.line > 153 {
                        self.wly = 0;
                        self.set_mode(Mode::OAMAccess);
                        self.set_current_line(0);
                    }
                    // self.set_equal_lines_check(&mut request);
                }
            }
            Mode::OAMAccess => {
                if self.cycles >= 80 {
                    self.cycles %= 80;
                    self.set_mode(Mode::VRAMAccess);
                }
            }
            Mode::VRAMAccess => {
                if self.cycles >= 172 {
                    self.cycles %= 172;
                    self.set_mode(Mode::HorizontalBlank);
                    self.render_line()
                }
            }
        }
    }

    fn fire_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupts_fired |= interrupt as u8;
    }

    fn set_mode(&mut self, mode: Mode) {
        self.stat.mode = mode;
        self.fire_stat_interrupt();
    }

    fn fire_stat_interrupt(&mut self) {
        match self.stat.mode {
            Mode::OAMAccess => {
                if self.stat.oam_interrupt {
                    self.fire_interrupt(Interrupt::LCDStat);
                }
            }
            Mode::VerticalBlank => {
                if self.stat.v_blank_interrupt {
                    self.fire_interrupt(Interrupt::LCDStat);
                }
            }
            Mode::HorizontalBlank => {
                if self.stat.h_blank_interrupt {
                    self.fire_interrupt(Interrupt::LCDStat);
                }
            }
            _ => {}
        }
    }

    fn set_current_line(&mut self, value: u8) {
        self.line = value;
        self.set_equal_lines_check();
    }

    fn set_equal_lines_check(&mut self) {
        self.stat.coincidence_flag = false;
        if self.line == self.line_check {
            self.stat.coincidence_flag = true;
            if self.stat.coincidence_interrupt {
                self.fire_interrupt(Interrupt::LCDStat);
            }
        }
    }

    fn render_line(&mut self) {
        if self.gb_mode == GameBoyMode::Dmg {
            self.render_scan_line();
        } else {
            self.render_scan_line_cgb();
        }
    }

    fn render_scan_line(&mut self) {
        if self.lcdc.bg_window_enabled {
            self.render_background_line();
        } else {
            for x in 0..SCREEN_WIDTH {
                let canvas_buffer_offset = (x * 4) + (self.line as usize * SCREEN_WIDTH * 4);
                self.canvas_buffer[canvas_buffer_offset] = 255;
                self.canvas_buffer[canvas_buffer_offset + 1] = 255;
                self.canvas_buffer[canvas_buffer_offset + 2] = 255;
                self.canvas_buffer[canvas_buffer_offset + 3] = 255;
            }
        }

        if self.lcdc.window_display_enabled {
            self.render_window_line();
        }

        if self.lcdc.object_display_enabled {
            self.render_object_line();
        }
    }

    fn render_background_line(&mut self) {
        let tile_y_index = self.line.wrapping_add(self.scroll_y);

        for x in 0..SCREEN_WIDTH as u8 {
            let tile_x_index = x.wrapping_add(self.scroll_x);

            let tile_address = self.calculate_bg_address(tile_y_index, tile_x_index);
            let tile_number = self.vram[(tile_address - VRAM_BEGIN as u16) as usize];
            let tile = self.calculate_tile_address(tile_number) - VRAM_BEGIN as u16;

            let pixel_index = 7 - (tile_x_index % 8);

            let y_address_offset = (tile_y_index % 8 * 2) as u16;

            let tile_data_address = tile + y_address_offset;

            let tile_data = self.vram[tile_data_address as usize];
            let tile_color_data = self.vram[tile_data_address as usize + 1];

            let color_index = get_color_index(tile_data, tile_color_data, pixel_index);

            let palette = match self.boot_rom {
                true => &self.palettes_bg[0],
                false => &self.palette_bg,
            };

            let pixel = palette[color_index as usize];

            if color_index == 0 {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::Color0;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority = false;
            } else {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::None;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority = false;
            }

            self.draw_pixel_to_buffer(x as usize, self.line as usize, pixel);
        }
    }

    fn render_window_line(&mut self) {
        if self.line < self.window_y
            || self.window_x < 7
            || self.window_x >= 167
            || self.line >= 144
        {
            return;
        }

        let screen_x = self.window_x.wrapping_sub(7);

        for x in screen_x..SCREEN_WIDTH as u8 {
            let tile_address = self.calculate_window_address(self.wly, x);
            let tile_number = self.vram[(tile_address - VRAM_BEGIN as u16) as usize];

            let tile = self.calculate_tile_address(tile_number) - VRAM_BEGIN as u16;

            let pixel_index = self.window_x.wrapping_sub(x) % 8;

            let y_address_offset = ((self.wly) % 8 * 2) as u16;

            let tile_data_address = tile + y_address_offset;

            let tile_data = self.vram[tile_data_address as usize];
            let tile_color_data = self.vram[tile_data_address as usize + 1];

            let color_index = get_color_index(tile_data, tile_color_data, pixel_index);

            let palette = match self.boot_rom {
                true => &self.palettes_bg[0],
                false => &self.palette_bg,
            };

            let pixel = palette[color_index as usize];

            if color_index == 0 {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::Color0;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority = false;
            } else {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::None;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority = false;
            }

            self.draw_pixel_to_buffer(x as usize, self.line as usize, pixel);
        }
        self.wly = self.wly.wrapping_add(1);
    }

    fn render_object_line(&mut self) {
        let object_height = if self.lcdc.sprite_size { 16 } else { 8 };
        let objs = self.fetch_objects();
        for object in objs.iter().rev() {
            let line_offset = if object.yflip {
                object_height - 1 - (self.line as i16 - object.y)
            } else {
                self.line as i16 - object.y
            };

            let tile_address = object.tile as u16 * 16;

            let tile_data = self.vram[tile_address as usize + (line_offset * 2) as usize];
            let tile_color_data = self.vram[tile_address as usize + (line_offset * 2) as usize + 1];

            for x in 0..8 {
                let x_offset = object.x + x as i16;

                if x_offset < 0 || x_offset >= SCREEN_WIDTH as i16 {
                    continue;
                }

                let pixel_index = if object.xflip { x } else { 7 - x };

                let color_index = get_color_index(tile_data, tile_color_data, pixel_index);

                let palette_index = if object.palette == ObjectPalette::One {
                    1
                } else {
                    0
                };

                let palette = match self.boot_rom {
                    true => self.palettes_object[palette_index],
                    false => self.dmg_object_palettes[palette_index],
                };

                let palette_index =
                    (self.palettes[object.palette as usize + 1] >> (color_index * 2)) & 0x03;

                if color_index != 0 {
                    let offset = self.line as usize + 256 * x_offset as usize;
                    let pixel = palette[palette_index as usize];

                    if !self.background_has_priority(object.priority, offset) {
                        self.draw_pixel_to_buffer(x_offset as usize, self.line as usize, pixel);
                    }
                }
            }
        }
    }

    fn render_scan_line_cgb(&mut self) {
        self.render_background_line_cgb();

        if self.lcdc.window_display_enabled {
            self.render_window_line_cgb();
        }

        if self.lcdc.object_display_enabled {
            self.render_object_line_cgb();
        }
    }

    fn render_background_line_cgb(&mut self) {
        let tile_y_index = self.line.wrapping_add(self.scroll_y);

        for x in 0..SCREEN_WIDTH as u8 {
            let tile_x_index = x.wrapping_add(self.scroll_x);

            let tile_address = self.calculate_bg_address(tile_y_index, tile_x_index);
            let tile_number = self.vram[(tile_address - VRAM_BEGIN as u16) as usize];
            let tile = self.calculate_tile_address(tile_number) - VRAM_BEGIN as u16;

            let pixel_index = 7 - (tile_x_index % 8);

            let y_address_offset = (tile_y_index % 8 * 2) as u16;

            let map_offset = if self.lcdc.bg_tile_map {
                0x9C00
            } else {
                0x9800
            };

            let bg_map_attributes = if self.lcdc.bg_tile_map {
                self.bg_map_attributes1
            } else {
                self.bg_map_attributes0
            };

            let bg_attributes = bg_map_attributes[(tile_address - map_offset) as usize];

            let pixel_index = if bg_attributes & 0x20 != 0 {
                7 - pixel_index
            } else {
                pixel_index
            };

            let y_address_offset = if bg_attributes & 0x40 != 0 {
                ((7 - (tile_y_index % 8)) * 2) as u16
            } else {
                y_address_offset
            };

            let palette = bg_attributes & 0x07;
            let palette_row = self.palettes_bg[palette as usize];

            let vram_bank = bg_attributes & 0x08 != 0;
            let vram = if vram_bank { self.vram1 } else { self.vram };

            let priority = bg_attributes & 0x80 != 0;

            let tile_data_address = tile + y_address_offset;

            let tile_data = vram[tile_data_address as usize];
            let tile_color_data = vram[tile_data_address as usize + 1];

            let color_index = get_color_index(tile_data, tile_color_data, pixel_index);
            let color = palette_row[color_index as usize];

            if color == palette_row[0] {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::Color0;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority = priority;
            } else {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::None;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority = priority;
            }

            self.draw_pixel_to_buffer(x as usize, self.line as usize, color);
        }
    }

    fn render_window_line_cgb(&mut self) {
        if self.line < self.window_y
            || self.window_x < 7
            || self.window_x >= 167
            || self.line >= 144
        {
            return;
        }

        let screen_x = self.window_x.wrapping_sub(7);

        for x in screen_x..SCREEN_WIDTH as u8 {
            let tile_address = self.calculate_window_address(self.wly, x);
            let tile_number = self.vram[(tile_address - VRAM_BEGIN as u16) as usize];

            let tile = self.calculate_tile_address(tile_number) - VRAM_BEGIN as u16;

            let pixel_index = self.window_x.wrapping_sub(x) % 8;

            let y_address_offset = ((self.wly) % 8 * 2) as u16;

            let map_offset = if self.lcdc.window_tile_map {
                0x9C00
            } else {
                0x9800
            };

            let window_map_attributes = if self.lcdc.window_tile_map {
                self.bg_map_attributes1
            } else {
                self.bg_map_attributes0
            };

            let bg_attributes = window_map_attributes[(tile_address - map_offset) as usize];

            let pixel_index = if bg_attributes & 0x20 != 0 {
                7 - pixel_index
            } else {
                pixel_index
            };

            let y_address_offset = if bg_attributes & 0x40 != 0 {
                ((7 - (self.wly % 8)) * 2) as u16
            } else {
                y_address_offset
            };

            let palette = bg_attributes & 0x07;

            let palette_row = self.palettes_bg[palette as usize];

            let vram_bank = bg_attributes & 0x08 != 0;

            let vram = if vram_bank { self.vram1 } else { self.vram };

            let tile_data_address = tile + y_address_offset;

            let tile_data = vram[tile_data_address as usize];
            let tile_color_data = vram[tile_data_address as usize + 1];

            let color_index = get_color_index(tile_data, tile_color_data, pixel_index);
            let color = palette_row[color_index as usize];

            if color == palette_row[0] {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::Color0;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority =
                    bg_attributes & 0x80 != 0;
            } else {
                self.bg_priority_map[self.line as usize + 256 * x as usize].color =
                    PriorityFlag::None;
                self.bg_priority_map[self.line as usize + 256 * x as usize].priority =
                    bg_attributes & 0x80 != 0;
            }

            self.draw_pixel_to_buffer(x as usize, self.line as usize, color);
        }
        self.wly = self.wly.wrapping_add(1);
    }

    fn render_object_line_cgb(&mut self) {
        let object_height = if self.lcdc.sprite_size { 16 } else { 8 };
        let objs = self.fetch_objects();
        for object in objs.iter().rev() {
            let line_offset = if object.yflip {
                object_height - 1 - (self.line as i16 - object.y)
            } else {
                self.line as i16 - object.y
            };

            let tile_address = object.tile as u16 * 16;

            let vram_bank = object.bank;

            let vram = if vram_bank { self.vram1 } else { self.vram };

            let tile_data = vram[tile_address as usize + (line_offset * 2) as usize];
            let tile_color_data = vram[tile_address as usize + (line_offset * 2) as usize + 1];

            for x in 0..8 {
                let x_offset = object.x + x as i16;

                if x_offset < 0 || x_offset >= SCREEN_WIDTH as i16 {
                    continue;
                }

                let pixel_index = if object.xflip { x } else { 7 - x };

                let color_index = get_color_index(tile_data, tile_color_data, pixel_index);

                let object_palette = object.cgb_palette;
                let palette = self.palettes_object[object_palette as usize];

                if color_index != 0 {
                    let offset = self.line as usize + 256 * x_offset as usize;
                    let color = palette[color_index as usize];

                    if !self.background_has_priority(object.priority, offset) {
                        self.draw_pixel_to_buffer(x_offset as usize, self.line as usize, color);
                    }
                }
            }
        }
    }

    fn fetch_objects(&self) -> Vec<ObjectData> {
        let object_height = if self.lcdc.sprite_size { 16 } else { 8 };
        let mut objects: Vec<ObjectData> = vec![];
        for object in 0..NUMBER_OF_OBJECTS {
            if objects.len() >= 10 {
                break;
            }
            let object_address = object * 4;
            let y = self.oam[object_address].wrapping_sub(16);
            let object_x = self.oam[object_address + 1] as i16 - 8;
            if self.line >= y && self.line < y + object_height {
                let mut tile = self.oam[object_address + 2];
                let options = self.oam[object_address + 3];
                if object_height == 16 {
                    tile &= 0xFE;
                }
                objects.push(ObjectData {
                    x: object_x,
                    y: y as i16,
                    tile,
                    palette: if options & 0x10 != 0 {
                        ObjectPalette::One
                    } else {
                        ObjectPalette::Zero
                    },
                    xflip: options & 0x20 != 0,
                    yflip: options & 0x40 != 0,
                    priority: options & 0x80 != 0,
                    cgb_palette: options & 0x07,
                    bank: options & 0x08 != 0,
                });
            }
        }
        if self.gb_mode == GameBoyMode::Dmg {
            objects.sort_by_key(|object| object.x);
        }
        objects
    }

    fn background_has_priority(&self, priority: bool, offset: usize) -> bool {
        if self.gb_mode == GameBoyMode::Dmg {
            if priority && self.bg_priority_map[offset].color == PriorityFlag::Color0 {
                return false;
            } else {
                return priority;
            }
        }

        if self.bg_priority_map[offset].color == PriorityFlag::Color0 {
            false
        } else if !self.lcdc.bg_window_enabled {
            return false;
        } else {
            !(!priority && !self.bg_priority_map[offset].priority)
        }
    }

    fn calculate_window_address(&self, y: u8, x: u8) -> u16 {
        let tile_map = if self.lcdc.window_tile_map {
            0x9C00
        } else {
            0x9800
        };
        let x_offset = x.wrapping_sub(self.window_x.wrapping_sub(7));

        calculate_address(tile_map, y, x_offset)
    }

    fn calculate_bg_address(&self, y: u8, x: u8) -> u16 {
        let tile_map = if self.lcdc.bg_tile_map {
            0x9C00
        } else {
            0x9800
        };

        calculate_address(tile_map, y, x)
    }

    fn calculate_tile_address(&self, tile_number: u8) -> u16 {
        if self.lcdc.bg_window_tile_data {
            return TILESET_FIRST_BEGIN_ADDRESS + tile_number as u16 * 16;
        }
        TILESET_SECOND_BEGIN_ADDRESS.wrapping_add(((tile_number as i8) as u16).wrapping_mul(16))
    }

    fn draw_pixel_to_buffer(&mut self, x: usize, y: usize, pixel: Pixel) {
        let canvas_buffer_offset = (x * 4) + (y * SCREEN_WIDTH * 4);

        self.canvas_buffer[canvas_buffer_offset] = pixel.r;
        self.canvas_buffer[canvas_buffer_offset + 1] = pixel.g;
        self.canvas_buffer[canvas_buffer_offset + 2] = pixel.b;
        self.canvas_buffer[canvas_buffer_offset + 3] = 255;
    }
}

fn calculate_address(address: u16, y: u8, x: u8) -> u16 {
    address + (y as u16 / 8 * 32) + (x as u16 / 8)
}

fn get_color_index(tile_data: u8, tile_color_data: u8, pixel_index: u8) -> u8 {
    (if tile_data & (1 << pixel_index) > 0 {
        1
    } else {
        0
    }) | (if tile_color_data & (1 << pixel_index) > 0 {
        1
    } else {
        0
    }) << 1
}

pub fn rgb555_to_rgb888(first: u8, second: u8) -> Pixel {
    let r_5 = first & 0x1F;
    let g_5 = (first >> 5) | ((second & 0x03) << 3);
    let b_5 = second >> 2;

    let r = (r_5 << 3) | (r_5 >> 2);
    let g = (g_5 << 3) | (g_5 >> 2);
    let b = (b_5 << 3) | (b_5 >> 2);
    Pixel { r, g, b }
}
