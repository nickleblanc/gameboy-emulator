pub struct Lcdc {
    pub display_enabled: bool,
    pub window_tile_map: bool,
    pub window_display_enabled: bool,
    pub bg_window_tile_data: bool,
    pub bg_tile_map: bool,
    pub sprite_size: bool,
    pub object_display_enabled: bool,
    pub bg_window_enabled: bool,
}

impl Lcdc {
    pub fn new() -> Lcdc {
        Lcdc {
            display_enabled: false,
            window_tile_map: false,
            window_display_enabled: false,
            bg_window_tile_data: false,
            bg_tile_map: false,
            sprite_size: false,
            object_display_enabled: false,
            bg_window_enabled: false,
        }
    }

    pub fn to_byte(&self) -> u8 {
        let mut byte = 0;
        if self.display_enabled {
            byte |= 1 << 7;
        }
        if self.window_tile_map {
            byte |= 1 << 6;
        }
        if self.window_display_enabled {
            byte |= 1 << 5;
        }
        if self.bg_window_tile_data {
            byte |= 1 << 4;
        }
        if self.bg_tile_map {
            byte |= 1 << 3;
        }
        if self.sprite_size {
            byte |= 1 << 2;
        }
        if self.object_display_enabled {
            byte |= 1 << 1;
        }
        if self.bg_window_enabled {
            byte |= 1;
        }
        byte
    }

    pub fn from_byte(&mut self, byte: u8) {
        self.display_enabled = byte & (1 << 7) != 0;
        self.window_tile_map = byte & (1 << 6) != 0;
        self.window_display_enabled = byte & (1 << 5) != 0;
        self.bg_window_tile_data = byte & (1 << 4) != 0;
        self.bg_tile_map = byte & (1 << 3) != 0;
        self.sprite_size = byte & (1 << 2) != 0;
        self.object_display_enabled = byte & (1 << 1) != 0;
        self.bg_window_enabled = byte & 1 != 0;
    }
}
