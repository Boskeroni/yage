//#![allow(unused, dead_code)]

mod processor;
mod registers;
mod memory;
mod opcodes;
mod gpu;
mod timer;

use std::{env, fs::File, io::Write};

use processor::{Cpu, handle_interrupts};
use gpu::{Ppu, update_ppu};
use memory::Memory;
use processor::run;
use timer::update_timer;

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
    let args: Vec<String> = env::args().collect();
    
    let rom_path;
    let booted;

    if args.len() == 2 {
        rom_path = &args[1];
        booted = true;
    } else if args.len() == 3 {
        rom_path = &args[1];
        booted = !(args[2] == "unbooted");
    } else {
        panic!("invalid number of arguments provided");
    }


    let mut cpu = Cpu::new(booted);
    let mut memory = Memory::new(get_rom(rom_path), booted);
    let mut ppu = Ppu::default();
    let mut timer = 0;

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
    //window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        while window_buffer.len() != 144 {
            let cycles = run(&mut cpu, &mut memory);
            
            update_timer(&mut timer, &mut memory, cycles);
            handle_interrupts(&mut cpu, &mut memory);

            // just useful for any outputs some roms may have
            serial_output(&mut memory);

            if let Some(line) = update_ppu(&mut ppu, &mut memory, cycles) {
                window_buffer.push(line);
            }
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

    let mut debug_file = File::create("debug.gb").unwrap();
    debug_file.write_all(&memory.mem).unwrap();
}

fn serial_output(mem: &mut Memory) {
    if mem.read(0xFF02) == 0x81 {
        let c = mem.read(0xFF01) as char;
        print!("{c}");
        mem.write(0xFF02, 0);
    }
}

fn to_screen_pixel(p: u8) -> u32 {
    match p {
        0 => return 0xFF000000,
        1 => return 0xFF404040,
        2 => return 0xFF808080,
        3 => return 0xFFC0C0C0,
        4 => return 0xFFFFFFFF,
        _ => unreachable!()
    }
}

fn get_rom(rom_path: &String) -> Vec<u8> {
    match std::fs::read(rom_path) {
        Err(_) => panic!("invalid file provided"),
        Ok(f) => f,
    }
}
