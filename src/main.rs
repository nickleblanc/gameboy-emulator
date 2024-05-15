mod cartridge;
mod cpu;
mod gpu;
mod interrupts;
mod joypad;
mod mmu;
mod timer;

use cpu::CPU;
use mmu::Memory;

use std::io::Read;
use std::thread::sleep;
use std::{fs::File, time::Duration, time::Instant};

use joypad::Key;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::video::Window;

// use rfd::FileDialog;
// use std::path::PathBuf;

const SCALE: u32 = 5;
const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;
const WINDOW_WIDTH: u32 = SCREEN_WIDTH * SCALE;
const WINDOW_HEIGHT: u32 = SCREEN_HEIGHT * SCALE;

const DEBUG: bool = false;

fn main() {
    let mut rom = File::open("./test-roms/Pokemon - Red.gb").expect("failed to open file");
    // let mut rom = File::open("./test-roms/Pokemon - Silver.gbc").expect("failed to open file");
    // let mut rom = File::open("./test-roms/Pocket Monsters - Aka.gb").expect("failed to open file");

    let (window, sdl_context) = initialize_sdl2();

    // let mut rom =
    // File::open("./test-roms/Dr. Mario (World) (Rev 1).gb").expect("failed to open file");
    // let mut rom = File::open("./test-roms/dmg-acid2.gb").expect("failed to open file");
    // let mut rom = File::open("./test-roms/cgb-acid2.gbc").expect("failed to open file");
    // let mut rom = File::open("./test-roms/Tetris.gb").expect("failed to open file");
    // let mut rom = File::open("./test-roms/cpu_instrs.gb").expect("failed to open file");

    // let file_path = file_dialog();

    // if let Some(path) = file_path {
    //     rom = File::open(path).expect("failed to open file");
    //     window.raise();
    // }

    let mut contents = Vec::new();
    rom.read_to_end(&mut contents).unwrap();

    let cartridge = cartridge::new_cartridge(contents.clone());
    let mmu = Memory::new(cartridge);
    let mut cpu = cpu::CPU::new(mmu);

    sdl2(&mut cpu, window, sdl_context);
}

fn initialize_sdl2() -> (Window, sdl2::Sdl) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Create a window
    let window = video_subsystem
        .window(
            "Gameboy Emulator",
            WINDOW_WIDTH as u32,
            WINDOW_HEIGHT as u32,
        )
        .position_centered()
        .build()
        .unwrap();

    (window, sdl_context)
}

fn sdl2(cpu: &mut CPU, window: Window, sdl_context: sdl2::Sdl) {
    // Initialize SDL2

    let mut canvas = window.into_canvas().build().unwrap();

    // Create a texture to render to
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(
            sdl2::pixels::PixelFormatEnum::RGBA32,
            sdl2::render::TextureAccess::Streaming,
            SCREEN_WIDTH as u32,
            SCREEN_HEIGHT as u32,
        )
        .unwrap();

    // Wait for a quit event
    let mut cycles_elapsed_in_frame = 0usize;
    let mut now = Instant::now();
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Up => cpu.mem.joypad.push_button(Key::Up),
                    Keycode::Down => cpu.mem.joypad.push_button(Key::Down),
                    Keycode::Left => cpu.mem.joypad.push_button(Key::Left),
                    Keycode::Right => cpu.mem.joypad.push_button(Key::Right),
                    Keycode::Z => cpu.mem.joypad.push_button(Key::A),
                    Keycode::X => cpu.mem.joypad.push_button(Key::B),
                    Keycode::Return => cpu.mem.joypad.push_button(Key::Start),
                    Keycode::RShift => cpu.mem.joypad.push_button(Key::Select),
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Up => cpu.mem.joypad.release_button(Key::Up),
                    Keycode::Down => cpu.mem.joypad.release_button(Key::Down),
                    Keycode::Left => cpu.mem.joypad.release_button(Key::Left),
                    Keycode::Right => cpu.mem.joypad.release_button(Key::Right),
                    Keycode::Z => cpu.mem.joypad.release_button(Key::A),
                    Keycode::X => cpu.mem.joypad.release_button(Key::B),
                    Keycode::Return => cpu.mem.joypad.release_button(Key::Start),
                    Keycode::RShift => cpu.mem.joypad.release_button(Key::Select),
                    _ => {}
                },
                _ => {}
            }
        }
        let time_delta = now.elapsed().subsec_nanos();
        now = Instant::now();
        let delta = time_delta as f64 / 1_000_000_000 as f64;
        let cycles_to_run = delta * 4190000 as f64;
        let mut cycles_elapsed = 0;
        while cycles_elapsed <= cycles_to_run as usize {
            cycles_elapsed += cpu.step() as usize;
            if DEBUG {
                cpu.log();
            }
        }
        cycles_elapsed_in_frame += cycles_elapsed;
        if cycles_elapsed_in_frame >= 70224 {
            texture
                .update(
                    None,
                    &cpu.mem.gpu.canvas_buffer,
                    (SCREEN_WIDTH * 4) as usize,
                )
                .unwrap();
            canvas.clear();
            canvas
                .copy(
                    &texture,
                    None,
                    Some(Rect::new(0, 0, WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)),
                )
                .unwrap();
            canvas.present();
            cycles_elapsed_in_frame = 0;
        } else {
            sleep(Duration::from_nanos(2));
        }
    }
}

// fn file_dialog() -> Option<PathBuf> {
//     let file_path = FileDialog::new()
//         .add_filter("Gameboy ROM", &["gb", "gbc"])
//         .pick_file();
//     return file_path;
// }
