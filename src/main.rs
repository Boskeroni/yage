mod processor;
mod registers;
mod memory;
mod opcodes;
mod gpu;
mod timer;
mod util;

use core::panic;
use std::{env, fs::File, io::Write};
use macroquad::prelude::*;

use util::{JOYPAD_ADDRESS, INTERRUPT_FLAG_ADDRESS};
use processor::{Cpu, handle_interrupts};
use gpu::{Ppu, update_ppu};
use memory::Memory;
use processor::run;
use timer::update_timer;


fn joypad(mem: &mut Memory) {
    let mut current_joypad = mem.read(JOYPAD_ADDRESS);
    let old_joypad = current_joypad;
    
    let mut pressed = false;

    

    mem.unchecked_write(JOYPAD_ADDRESS, current_joypad);
    todo!();
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
        4 => return BLANK,
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

const SCALE_FACTOR: i32 = 3;
fn window_conf() -> Conf {
    Conf {
        window_title: "gameboy emulator".to_owned(),
        window_resizable: false,
        window_height: 144*SCALE_FACTOR,
        window_width: 160*SCALE_FACTOR,
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

    let mut pixel_buffer: Vec<u8> = Vec::new();

    'full: loop {
        while pixel_buffer.len() != 23040 {
            if cpu.regs.pc == 0x100 && !booted {
                break 'full;
            }

            handle_interrupts(&mut cpu, &mut memory);
            let mut cycles = 4;
            if !cpu.halt {
                cycles = run(&mut cpu, &mut memory);
            }

            update_timer(&mut memory, cycles);
            joypad(&mut memory);

            // just useful for any outputs some roms may have
            serial_output(&mut memory);

            if let Some(line) = update_ppu(&mut ppu, &mut memory, cycles) {
                pixel_buffer.extend::<Vec<u8>>(line);
            }
        }
        // all of the actual rendering to the screen
        clear_background(BLACK);
        for (j, pixel) in pixel_buffer.iter().enumerate() {
            let pixel = to_screen_pixel(*pixel);
            draw_rectangle(
                ((j%160)*SCALE_FACTOR as usize) as f32, //pos
                ((j/160)*SCALE_FACTOR as usize) as f32, //pos
                SCALE_FACTOR as f32, //width
                SCALE_FACTOR as f32, //height
                pixel // color
            );
        }
        pixel_buffer.clear();
        next_frame().await;

        // debug section of the emulator
        if !booted && cpu.regs.pc == 0xE9 {
            dump_memory(&memory);
            break 'full;
        }
    }
}
