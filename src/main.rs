//#![allow(unused, dead_code)]

mod processor;
mod registers;
mod memory;
mod opcodes;
mod gpu;

use std::env;

use processor::{Cpu, handle_interrupts};
use gpu::{Ppu, update_ppu};
use memory::{Memory, update_timer};
use processor::run;

use minifb::{Key, Scale, Window, WindowOptions};

const HEIGHT: usize = 144;
const WIDTH: usize = 160;


pub fn little_endian_combine(a: u8, b: u8) -> u16 {
    (b as u16) << 8 | (a as u16)
}
pub fn combine(a: u8, b: u8) -> u16 {
    (a as u16) << 8 | b as u16
}
pub fn split(a: u16) -> (u8, u8) {
    (((a & 0xFF00) >> 8) as u8, (a & 0xFF) as u8)
}

fn main() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(get_rom());
    let mut ppu = Ppu::default();


    let mut window_buffer = Vec::new();
    let mut window = Window::new(
        "gameboy emulator",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: false,
            scale: Scale::X1,
            ..Default::default()
        },
    ).expect("unable to create window");
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        while window_buffer.len() != 144 {
            handle_interrupts(&mut cpu, &mut memory);
            let temp_cycles = run(&mut cpu, &mut memory);
            update_timer(&mut memory, temp_cycles);
    
            if let Some(line) = update_ppu(&mut ppu, &mut memory, temp_cycles) {
                window_buffer.push(line);
            }
            println!("{window_buffer:?}");
        }
        // need to convert the Vec<Vec<u8>> into Vec<u8>
        let flat_buffer = window_buffer.concat();
        let mut parsed_buffer = Vec::new();
        for i in flat_buffer {
            parsed_buffer.push(to_screen_pixel(i));
        }
        window.update_with_buffer(&parsed_buffer, WIDTH, HEIGHT).unwrap();
        window_buffer.clear();
    }
}

fn to_screen_pixel(p: u8) -> u32 {
    match p {
        0 => return 0x000000,
        1 => return 0x404040,
        2 => return 0x808080,
        3 => return 0xC0C0C0,
        4 => return 0xFFFFFF,
        _ => unreachable!()
    }
}

fn get_rom() -> Vec<u8> {
    let args: Vec<String> = env::args().collect();

    // no file path provided
    if args.len() == 1 {
        panic!("no file path was provided");
    }
    let rom_path = &args[1];
    match std::fs::read(rom_path) {
        Err(_) => panic!("invalid file provided"),
        Ok(f) => f,
    }
}
