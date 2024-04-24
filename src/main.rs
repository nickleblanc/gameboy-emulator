mod cpu;
mod interrupts;
mod mmu;
mod ppu;
mod timer;

use std::io::Read;
use std::thread::sleep;
use std::{fs::File, time::Duration, time::Instant};

use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

// use cpu::CPU;

fn main() {
    File::create("log.txt").expect("failed to create log file");

    // // Test ROMs
    // // 01-special - PASS
    // // 02-interrupts - EI Failed #2
    // // 03-op sp,hl - PASS
    // // 04-op r,imm - PASS
    // // 05-op rp - PASS
    // // 06-ld r,r - PASS
    // // 07-jr,jp,call,ret,rst - PASS
    // // 08-misc instrs - PASS
    // // 09-op r,r - PASS
    // // 10-bit ops - PASS
    // // 11-op a,(hl) - PASS

    let mut rom = File::open("./test-roms/01-special.gb").expect("failed to open file");
    let mut contents = Vec::new();
    rom.read_to_end(&mut contents).unwrap();

    let mut cpu = cpu::CPU::new();
    cpu.load(contents);

    let screen_buffer = cpu.mem.ppu.screen_buffer;

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut buffer = [0; 23040];
    let mut cycles_elapsed_in_frame = 0usize;
    let mut now = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let time_delta = now.elapsed().subsec_nanos();
        now = Instant::now();
        let delta = time_delta as f64 / 1_000_000_000 as f64;
        let cycles_to_run = delta * 4190000 as f64;
        let mut cycles_elapsed = 0;
        while cycles_elapsed <= cycles_to_run as usize {
            cycles_elapsed += cpu.step() as usize;
        }
        cycles_elapsed_in_frame += cycles_elapsed;
        if cycles_elapsed_in_frame >= 70224 {
            for (i, pixel) in screen_buffer.chunks(4).enumerate() {
                buffer[i] = (pixel[3] as u32) << 24
                    | (pixel[2] as u32) << 16
                    | (pixel[1] as u32) << 8
                    | (pixel[0] as u32)
            }
            window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
            cycles_elapsed_in_frame = 0;
        } else {
            sleep(Duration::from_nanos(2));
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
    }
}
