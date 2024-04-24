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
}

impl Timer {
    pub fn new(frequency: Frequency) -> Timer {
        Timer {
            frequency,
            counter: 0,
            modulo: 0,
            enabled: false,
            cycles: 0,
        }
    }

    pub fn step(&mut self, cycles: u8) -> bool {
        if !self.enabled {
            return false;
        }

        self.cycles += cycles as usize;
        let timer_did_overflow = if self.cycles > self.frequency.cycles_per_tick() {
            let (value, overflow) = self.counter.overflowing_add(1);
            self.counter = value;
            overflow
        } else {
            false
        };

        if timer_did_overflow {
            self.counter = self.modulo;
        }
        timer_did_overflow
    }
}
