pub enum Frequency {
    F4096,
    F262144,
    F65536,
    F16384,
}

impl Frequency {
    fn cycles_per_tick(&self) -> usize {
        match self {
            Frequency::F4096 => 1024,
            Frequency::F262144 => 16,
            Frequency::F65536 => 64,
            Frequency::F16384 => 256,
        }
    }
}

pub struct Timer {
    pub frequency: Frequency,
    pub counter: u8,
    pub modulo: u8,
    pub enabled: bool,
    cycles: usize,
    has_overflowed: bool,
}

impl Timer {
    pub fn new(frequency: Frequency) -> Timer {
        Timer {
            frequency,
            counter: 0,
            modulo: 0,
            enabled: false,
            cycles: 0,
            has_overflowed: false,
        }
    }

    pub fn step(&mut self, cycles: u8) -> bool {
        if !self.enabled {
            return false;
        }

        if self.has_overflowed {
            self.has_overflowed = false;
            return true;
        }

        self.cycles += cycles as usize;
        self.has_overflowed = if self.cycles > self.frequency.cycles_per_tick() {
            self.cycles = self.cycles % self.frequency.cycles_per_tick();
            let (value, overflow) = self.counter.overflowing_add(1);
            self.counter = value;
            overflow
        } else {
            false
        };

        false
    }
}
