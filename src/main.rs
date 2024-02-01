//#![allow(unused, dead_code)]

mod processor;
mod registers;
mod memory;
mod opcodes;
mod gpu;
mod timer;

use std::{env, fs::File, io::Write, thread, time::Duration};

use processor::{Cpu, handle_interrupts};
use gpu::{Ppu, update_ppu};
use memory::Memory;
use processor::run;
use timer::update_timer;

use macroquad::prelude::*;

pub fn little_endian_combine(a: u8, b: u8) -> u16 {
    (b as u16) << 8 | (a as u16)
}
pub fn combine(a: u8, b: u8) -> u16 {
    (a as u16) << 8 | b as u16
}
pub fn split(a: u16) -> (u8, u8) {
    (((a & 0xFF00) >> 8) as u8, (a & 0xFF) as u8)
}


fn joypad(mem: &mut Memory) {
    // just some useful testing keys
    if is_key_pressed(KeyCode::Q) {
        dump_memory(&mem);
    }

    let mut current_joypad = mem.read(0xFF00);
    let mut button_pressed = false;


    if is_key_down(KeyCode::A) {
        button_pressed = true;
        current_joypad &= 0b1111_1110;
    } else {
        current_joypad |= 0b0000_0001;
    }
    if is_key_down(KeyCode::S) {
        button_pressed = true;
        current_joypad &= 0b1111_1101;
    } else {
        current_joypad |= 0b0000_0010;
    }
    if is_key_down(KeyCode::Z) {
        button_pressed = true;
        current_joypad &= 0b1111_1011
    } else {
        current_joypad |= 0b0000_0100;
    }
    if is_key_down(KeyCode::X) {
        button_pressed = true;
        current_joypad &= 0b1111_0111;
    } else {
        current_joypad |= 0b0000_1000;
    }

    if button_pressed {
        current_joypad &= 0b1101_1111;
    }
    mem.write(0xFF00, current_joypad);
}

fn serial_output(mem: &mut Memory) {
    if mem.read(0xFF02) == 0x81 {
        let c = mem.read(0xFF01) as char;
        print!("{c}");
        mem.write(0xFF02, 0);
    }
}

fn to_screen_pixel(p: u8) -> Color {
    match p {
        0 => return WHITE,
        1 => return LIGHTGRAY,
        2 => return DARKGRAY,
        3 => return BLACK,
        // error color
        4 => return YELLOW,
        _ => unreachable!()
    }
}

fn get_rom(rom_path: &String) -> Vec<u8> {
    match std::fs::read(rom_path) {
        Err(_) => panic!("invalid file provided"),
        Ok(f) => f,
    }
}

fn dump_memory(mem: &Memory) {
    let mut debug_file = File::create("debug.gb").unwrap();
    debug_file.write_all(&mem.mem).unwrap();
}

fn window_conf() -> Conf {
    Conf {
        window_title: "gameboy emulator".to_owned(),
        window_resizable: false,
        window_height: 144,
        window_width: 160,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
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

    let mut current_line = 0;
    let mut pixel_buffer: Vec<u8> = Vec::new();

    'full: loop {
        while current_line != 144 {
            if cpu.regs.pc == 0x100 && !booted {
                break 'full;
            }

            let mut cycles = 4;
            if !cpu.halt {
                cycles = run(&mut cpu, &mut memory);
            }

            update_timer(&mut memory, cycles);
            joypad(&mut memory);
            handle_interrupts(&mut cpu, &mut memory);


            // just useful for any outputs some roms may have
            serial_output(&mut memory);

            if let Some(line) = update_ppu(&mut ppu, &mut memory, cycles) {
                pixel_buffer.extend::<Vec<u8>>(line);
                current_line+=1;
            }
        }

        // all of the actual rendering to the screen
        clear_background(BLACK);
        println!("{}", pixel_buffer.len());
        for (j, pixel) in pixel_buffer.iter().enumerate() {
            let pixel = to_screen_pixel(*pixel);
            draw_rectangle(
                (j%160) as f32, //pos
                (j/160) as f32, //pos
                1., //width
                1., //height
                pixel // color
            );
        }
        current_line = 0;
        pixel_buffer.clear();
        next_frame().await;
    
        thread::sleep(Duration::from_millis(60));

        // debug section of the emulator


        if !booted && cpu.regs.pc == 0xE9 {
            dump_memory(&memory);
            break 'full;
        }
    }
}
