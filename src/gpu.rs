use std;

mod lcdc;
pub mod stat;

use lcdc::Lcdc;
use stat::{Mode, Stat};

use crate::mmu::{OAM_SIZE, VRAM_BEGIN, VRAM_SIZE};

const TILESET_FIRST_BEGIN_ADDRESS: u16 = 0x8000;
const TILESET_SECOND_BEGIN_ADDRESS: u16 = 0x9000;

const NUMBER_OF_OBJECTS: usize = 40;
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Color {
    White = 255,
    LightGray = 192,
    DarkGray = 96,
    Black = 0,
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

#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BackgroundColors(Color, Color, Color, Color);

impl BackgroundColors {
    fn new() -> BackgroundColors {
        BackgroundColors(
            Color::White,
            Color::LightGray,
            Color::DarkGray,
            Color::Black,
        )
    }
}

impl std::convert::From<u8> for BackgroundColors {
    fn from(value: u8) -> Self {
        BackgroundColors(
            (value & 0b11).into(),
            ((value >> 2) & 0b11).into(),
            ((value >> 4) & 0b11).into(),
            (value >> 6).into(),
        )
    }
}

#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TileMap {
    X9800,
    X9C00,
}

#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BackgroundAndWindowDataSelect {
    X8000,
    X8800,
}

#[cfg_attr(feature = "serialize", derive(Serialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ObjectSize {
    OS8X8,
    OS8X16,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TilePixelValue {
    Zero,
    One,
    Two,
    Three,
}
impl Default for TilePixelValue {
    fn default() -> Self {
        TilePixelValue::Zero
    }
}

type TileRow = [TilePixelValue; 8];
type Tile = [TileRow; 8];
#[inline(always)]
fn empty_tile() -> Tile {
    [[Default::default(); 8]; 8]
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ObjectData {
    x: i16,
    y: i16,
    tile: u8,
    palette: ObjectPalette,
    xflip: bool,
    yflip: bool,
    priority: bool,
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
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ObjectPalette {
    Zero,
    One,
}

impl Default for ObjectPalette {
    fn default() -> Self {
        ObjectPalette::Zero
    }
}

#[derive(Eq, PartialEq)]
pub enum InterruptRequest {
    None,
    VBlank,
    LCDStat,
    Both,
}

impl InterruptRequest {
    fn add(&mut self, other: InterruptRequest) {
        match self {
            InterruptRequest::None => *self = other,
            InterruptRequest::VBlank if other == InterruptRequest::LCDStat => {
                *self = InterruptRequest::Both
            }
            InterruptRequest::LCDStat if other == InterruptRequest::VBlank => {
                *self = InterruptRequest::Both
            }
            _ => {}
        };
    }
}

#[derive(Copy, Clone)]
enum PriorityFlag {
    None,
    Color0,
}

#[cfg_attr(feature = "serialize", derive(Serialize))]

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct GPU {
    #[cfg_attr(feature = "serialize", serde(skip_serializing))]
    pub canvas_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
    #[cfg_attr(feature = "serialize", serde(skip_serializing))]
    pub tile_set: [Tile; 384],
    #[cfg_attr(feature = "serialize", serde(skip_serializing))]
    pub object_data: [ObjectData; NUMBER_OF_OBJECTS],
    #[cfg_attr(feature = "serialize", serde(skip_serializing))]
    pub vram: [u8; VRAM_SIZE],
    #[cfg_attr(feature = "serialize", serde(skip_serializing))]
    pub oam: [u8; OAM_SIZE],
    pub background_colors: BackgroundColors,
    pub line_check: u8,
    pub line: u8,
    pub cycles: u16,
    pub window_x: u8,
    pub window_y: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub lcdc: Lcdc,
    pub stat: Stat,
    wly: u8,
    bg_priority_map: [PriorityFlag; 65792],
    sprite_palette0: [Color; 4],
    sprite_palette1: [Color; 4],
}

impl GPU {
    pub fn new() -> GPU {
        GPU {
            canvas_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
            tile_set: [empty_tile(); 384],
            object_data: [Default::default(); NUMBER_OF_OBJECTS],
            vram: [0; VRAM_SIZE],
            oam: [0; OAM_SIZE],
            background_colors: BackgroundColors::from(0xFC),
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
            bg_priority_map: [PriorityFlag::None; 65792],
            sprite_palette0: [Color::Black; 4],
            sprite_palette1: [Color::White; 4],
        }
    }

    pub fn write_vram(&mut self, index: usize, value: u8) {
        self.vram[index] = value;
        if index >= 0x1800 {
            return;
        }

        // Tiles rows are encoded in two bytes with the first byte always
        // on an even address. Bitwise ANDing the address with 0xffe
        // gives us the address of the first byte.
        // For example: `12 & 0xFFFE == 12` and `13 & 0xFFFE == 12`
        let normalized_index = index & 0xFFFE;

        // First we need to get the two bytes that encode the tile row.
        let byte1 = self.vram[normalized_index];
        let byte2 = self.vram[normalized_index + 1];

        // A tiles is 8 rows tall. Since each row is encoded with two bytes a tile
        // is therefore 16 bytes in total.
        let tile_index = index / 16;
        // Every two bytes is a new row
        let row_index = (index % 16) / 2;

        // Now we're going to loop 8 times to get the 8 pixels that make up a given row.
        for pixel_index in 0..8 {
            // To determine a pixel's value we must first find the corresponding bit that encodes
            // that pixels value:
            // 1111_1111
            // 0123 4567
            //
            // As you can see the bit that corresponds to the nth pixel is the bit in the nth
            // position *from the left*. Bits are normally indexed from the right.
            //
            // To find the first pixel (a.k.a pixel 0) we find the left most bit (a.k.a bit 7). For
            // the second pixel (a.k.a pixel 1) we first the second most left bit (a.k.a bit 6) and
            // so on.
            //
            // We then create a mask with a 1 at that position and 0s everywhere else.
            //
            // Bitwise ANDing this mask with our bytes will leave that particular bit with its
            // original value and every other bit with a 0.
            let mask = 1 << (7 - pixel_index);
            let lsb = byte1 & mask;
            let msb = byte2 & mask;

            // If the masked values are not 0 the masked bit must be 1. If they are 0, the masked
            // bit must be 0.
            //
            // Finally we can tell which of the four tile values the pixel is. For example, if the least
            // significant byte's bit is 1 and the most significant byte's bit is also 1, then we
            // have tile value `Three`.
            let value = match (lsb != 0, msb != 0) {
                (true, true) => TilePixelValue::Three,
                (false, true) => TilePixelValue::Two,
                (true, false) => TilePixelValue::One,
                (false, false) => TilePixelValue::Zero,
            };

            self.tile_set[tile_index][row_index][pixel_index] = value;
        }
    }

    pub fn write_oam(&mut self, index: usize, value: u8) {
        self.oam[index] = value;
        let object_index = index / 4;
        if object_index > NUMBER_OF_OBJECTS {
            return;
        }

        let byte = index % 4;

        let mut data = self.object_data.get_mut(object_index).unwrap();
        match byte {
            0 => data.y = (value as i16) - 0x10,
            1 => data.x = (value as i16) - 0x8,
            2 => data.tile = value,
            _ => {
                data.palette = if (value & 0x10) != 0 {
                    ObjectPalette::One
                } else {
                    ObjectPalette::Zero
                };
                data.xflip = (value & 0x20) != 0;
                data.yflip = (value & 0x40) != 0;
                data.priority = (value & 0x80) == 0;
            }
        }
    }

    pub fn set_bg_palette(&mut self, value: u8) {
        self.background_colors = BackgroundColors::from(value);
    }

    pub fn set_object_palette0(&mut self, value: u8) {
        self.sprite_palette0 = [
            Color::from(value & 0b11),
            Color::from((value >> 2) & 0b11),
            Color::from((value >> 4) & 0b11),
            Color::from(value >> 6),
        ];
    }

    pub fn set_object_palette1(&mut self, value: u8) {
        self.sprite_palette1 = [
            Color::from(value & 0b11),
            Color::from((value >> 2) & 0b11),
            Color::from((value >> 4) & 0b11),
            Color::from(value >> 6),
        ];
    }

    pub fn get_bg_palette(&self) -> u8 {
        let mut value = 0;
        value |= self.background_colors.0 as u8;
        value |= (self.background_colors.1 as u8) << 2;
        value |= (self.background_colors.2 as u8) << 4;
        value |= (self.background_colors.3 as u8) << 6;
        value
    }

    pub fn get_object_palette0(&self) -> u8 {
        let mut value = 0;
        value |= self.sprite_palette0[0] as u8;
        value |= (self.sprite_palette0[1] as u8) << 2;
        value |= (self.sprite_palette0[2] as u8) << 4;
        value |= (self.sprite_palette0[3] as u8) << 6;
        value
    }

    pub fn get_object_palette1(&self) -> u8 {
        let mut value = 0;
        value |= self.sprite_palette1[0] as u8;
        value |= (self.sprite_palette1[1] as u8) << 2;
        value |= (self.sprite_palette1[2] as u8) << 4;
        value |= (self.sprite_palette1[3] as u8) << 6;
        value
    }

    pub fn step(&mut self, cycles: u8) -> InterruptRequest {
        let mut request = InterruptRequest::None;
        if !self.lcdc.display_enabled {
            return request;
        }
        self.cycles += cycles as u16;

        match self.stat.mode {
            Mode::HorizontalBlank => {
                if self.cycles >= 200 {
                    self.cycles = self.cycles % 200;
                    self.line += 1;

                    // if self.line >= self.window_y && self.window_x < 166 && self.window_x > 0 {
                    //     self.wly += 1;
                    // }

                    if self.line >= 144 {
                        self.stat.mode = Mode::VerticalBlank;
                        if self.stat.v_blank_interrupt {
                            request.add(InterruptRequest::LCDStat)
                        }
                        request.add(InterruptRequest::VBlank);
                        self.bg_priority_map = [PriorityFlag::None; 65792];
                    } else {
                        self.stat.mode = Mode::OAMAccess;
                        if self.stat.oam_interrupt {
                            request.add(InterruptRequest::LCDStat)
                        }
                    }
                    self.set_equal_lines_check(&mut request);
                }
            }
            Mode::VerticalBlank => {
                if self.cycles >= 456 {
                    self.cycles = self.cycles % 456;
                    self.line += 1;
                    if self.line == 154 {
                        self.stat.mode = Mode::OAMAccess;
                        self.line = 0;
                        self.wly = 0;
                        if self.stat.oam_interrupt {
                            request.add(InterruptRequest::LCDStat)
                        }
                    }
                    self.set_equal_lines_check(&mut request);
                }
            }
            Mode::OAMAccess => {
                if self.cycles >= 80 {
                    self.cycles = self.cycles % 80;
                    self.stat.mode = Mode::VRAMAccess;
                }
            }
            Mode::VRAMAccess => {
                if self.cycles >= 172 {
                    self.cycles = self.cycles % 172;
                    if self.stat.h_blank_interrupt {
                        request.add(InterruptRequest::LCDStat)
                    }
                    self.stat.mode = Mode::HorizontalBlank;
                    self.render_scan_line()
                }
            }
        }
        request
    }

    fn set_equal_lines_check(&mut self, request: &mut InterruptRequest) {
        let line_equals_line_check = self.line == self.line_check;
        if line_equals_line_check && self.stat.coincidence_interrupt {
            request.add(InterruptRequest::LCDStat);
        }
        self.stat.coincidence_flag = line_equals_line_check;
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

            // let tile_address = self.calculate_bg_address(tile_y_index, tile_x_index);

            let tile_number = self.vram[(tile_address - VRAM_BEGIN as u16) as usize];

            let tile = self.calculate_tile_address(tile_number) - VRAM_BEGIN as u16;

            // let pixel_index = self.scroll_x.wrapping_add(x as u8) % 8;
            let pixel_index = 7 - (tile_x_index % 8) as u8;

            // let pixel_index = if column_is_window && line_is_window {
            //     self.window_x.wrapping_sub(x) % 8
            // } else {
            //     7 - (tile_x_index % 8)
            // };

            let y_address_offset = (tile_y_index % 8 * 2) as u16;
            // let y_address_offset = if line_is_window && column_is_window {
            //     ((self.line - self.window_y) % 8 * 2) as u16
            // } else {
            //     (tile_y_index % 8 * 2) as u16
            // };

            let tile_data_address = tile + y_address_offset;

            let tile_data = self.vram[tile_data_address as usize];
            let tile_color_data = self.vram[tile_data_address as usize + 1];

            // let tile_value =
            //     self.tile_set[tile as usize][row_y_offset as usize][pixel_index as usize];
            // let color = self.tile_value_to_background_color(&tile_value);

            // let color = if line_is_window && column_is_window {
            //     // self.get_tile_color(tile_data, tile_color_data, pixel_index)
            //     self.background_colors.3
            // } else {
            //     self.get_tile_color(tile_data, tile_color_data, pixel_index)
            // };
            let color = self.get_tile_color(tile_data, tile_color_data, pixel_index);

            if color == self.background_colors.0 {
                self.bg_priority_map[self.line as usize + 256 * x as usize] = PriorityFlag::Color0;
            }

            let canvas_buffer_offset = (x as usize * 4) + (self.line as usize * SCREEN_WIDTH * 4);

            self.canvas_buffer[canvas_buffer_offset] = color as u8;
            self.canvas_buffer[canvas_buffer_offset + 1] = color as u8;
            self.canvas_buffer[canvas_buffer_offset + 2] = color as u8;
            self.canvas_buffer[canvas_buffer_offset + 3] = 255;
        }
    }

    fn render_window_line(&mut self) {
        let tile_y_index = self.line.wrapping_add(self.window_y);
        // let tile_y_index = self.wly.wrapping_add(self.scroll_y);
        // let tile_y_index = self.wly;

        if (self.line < self.window_y) {
            return;
        }

        if (self.window_x < 7) {
            return;
        }

        if (self.window_x >= 167) {
            return;
        }

        if (self.line >= 144) {
            return;
        }

        let screen_x = self.window_x.wrapping_sub(7);

        for x in screen_x..SCREEN_WIDTH as u8 {
            // let tile_address = self.calculate_bg_address(tile_y_index, tile_x_index);
            let tile_address = self.calculate_window_address(self.wly, x);

            let tile_number = self.vram[(tile_address - VRAM_BEGIN as u16) as usize];

            let tile = self.calculate_tile_address(tile_number) - VRAM_BEGIN as u16;

            let pixel_index = self.window_x.wrapping_sub(x) % 8;

            // let y_address_offset = (tile_y_index % 8 * 2) as u16;
            let y_address_offset = ((self.wly) % 8 * 2) as u16;

            let tile_data_address = tile + y_address_offset;

            let tile_data = self.vram[tile_data_address as usize];
            let tile_color_data = self.vram[tile_data_address as usize + 1];

            let color = self.get_tile_color(tile_data, tile_color_data, pixel_index);

            let canvas_buffer_offset = (x as usize * 4) + (self.line as usize * SCREEN_WIDTH * 4);

            self.canvas_buffer[canvas_buffer_offset] = color as u8;
            self.canvas_buffer[canvas_buffer_offset + 1] = color as u8;
            self.canvas_buffer[canvas_buffer_offset + 2] = color as u8;
            self.canvas_buffer[canvas_buffer_offset + 3] = 255;
        }
        self.wly = self.wly.wrapping_add(1);
    }

    fn background_has_priority(&self, priority: bool, offset: usize) -> bool {
        if !priority {
            return false;
        }

        match self.bg_priority_map[offset] {
            PriorityFlag::None => true,
            PriorityFlag::Color0 => false,
        }
    }

    fn calculate_window_address(&self, y: u8, x: u8) -> u16 {
        let tile_map = if self.lcdc.window_tile_map {
            0x9C00
        } else {
            0x9800
        };
        // let y_offset = y.wrapping_sub(self.window_y);
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

    fn get_tile_color(&self, tile_data: u8, tile_color_data: u8, pixel_index: u8) -> Color {
        let value = (if tile_data & (1 << pixel_index) > 0 {
            1
        } else {
            0
        }) | (if tile_color_data & (1 << pixel_index) > 0 {
            1
        } else {
            0
        }) << 1;

        let value = match value {
            0 => TilePixelValue::Zero,
            1 => TilePixelValue::One,
            2 => TilePixelValue::Two,
            3 => TilePixelValue::Three,
            _ => unreachable!(),
        };

        self.tile_value_to_background_color(&value)
    }

    fn calculate_tile_address(&self, tile_number: u8) -> u16 {
        if self.lcdc.bg_window_tile_data {
            return TILESET_FIRST_BEGIN_ADDRESS + tile_number as u16 * 16;
        }
        TILESET_SECOND_BEGIN_ADDRESS.wrapping_add(((tile_number as i8) as u16).wrapping_mul(16))
    }

    // fn render_object_line(&mut self) {
    //     let mut scan_line: [TilePixelValue; SCREEN_WIDTH] = [Default::default(); SCREEN_WIDTH];
    //     let mut num_objects = 0;
    //     let object_height = if self.lcdc.sprite_size { 16 } else { 8 };
    //     for object in self.object_data.iter() {
    //         if num_objects >= 10 {
    //             break;
    //         }
    //         let line = self.line as i16;
    //         if object.y <= line && object.y + object_height > line {
    //             let pixel_y_offset = line - object.y;
    //             let tile_index = if object_height == 16 && (!object.yflip && pixel_y_offset > 7)
    //                 || (object.yflip && pixel_y_offset <= 7)
    //             {
    //                 object.tile + 1
    //             } else {
    //                 object.tile
    //             };

    //             let tile = self.tile_set[tile_index as usize];
    //             let tile_row = if object.yflip {
    //                 tile[(7 - (pixel_y_offset % 8)) as usize]
    //             } else {
    //                 tile[(pixel_y_offset % 8) as usize]
    //             };

    //             let canvas_y_offset = line as i32 * SCREEN_WIDTH as i32;
    //             let mut canvas_offset = ((canvas_y_offset + object.x as i32) * 4) as usize;
    //             for x in 0..8i16 {
    //                 let pixel_x_offset = if object.xflip { (7 - x) } else { x } as usize;
    //                 let x_offset = object.x + x;
    //                 let pixel = tile_row[pixel_x_offset];
    //                 if x_offset >= 0
    //                     && x_offset < SCREEN_WIDTH as i16
    //                     && pixel != TilePixelValue::Zero
    //                     && (object.priority || scan_line[x_offset as usize] == TilePixelValue::Zero)
    //                 {
    //                     let color = self.tile_value_to_background_color(&pixel);

    //                     self.canvas_buffer[canvas_offset + 0] = color as u8;
    //                     self.canvas_buffer[canvas_offset + 1] = color as u8;
    //                     self.canvas_buffer[canvas_offset + 2] = color as u8;
    //                     self.canvas_buffer[canvas_offset + 3] = 255;
    //                 }
    //                 canvas_offset = canvas_offset.wrapping_add(4);
    //             }
    //             num_objects += 1;
    //         }
    //     }
    // }

    fn fetch_objects(&self) -> Vec<ObjectData> {
        let object_height = if self.lcdc.sprite_size { 16 } else { 8 };
        let mut objects: Vec<ObjectData> = vec![];
        for object in 0..40 {
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
                });
            }
        }
        objects.sort_by_key(|object| object.x);
        objects
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

                let object_palette = if object.palette == ObjectPalette::One {
                    self.sprite_palette1
                } else {
                    self.sprite_palette0
                };

                if color_index != 0 {
                    let offset = self.line as usize + 256 * x_offset as usize;
                    let color = object_palette[color_index as usize];

                    // if !self.background_has_priority(object.priority, offset) {
                    let canvas_buffer_offset =
                        (x_offset as usize * 4) + (self.line as usize * SCREEN_WIDTH * 4);

                    self.canvas_buffer[canvas_buffer_offset] = color as u8;
                    self.canvas_buffer[canvas_buffer_offset + 1] = color as u8;
                    self.canvas_buffer[canvas_buffer_offset + 2] = color as u8;
                    self.canvas_buffer[canvas_buffer_offset + 3] = 255;
                    // }
                }
            }
        }
    }

    fn tile_value_to_background_color(&self, tile_value: &TilePixelValue) -> Color {
        match tile_value {
            TilePixelValue::Zero => self.background_colors.0,
            TilePixelValue::One => self.background_colors.1,
            TilePixelValue::Two => self.background_colors.2,
            TilePixelValue::Three => self.background_colors.3,
        }
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
