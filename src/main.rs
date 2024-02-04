mod processor;
mod registers;
mod memory;
mod opcodes;
mod gpu;
mod timer;
mod util;

use core::panic;
use std::env;
use macroquad::prelude::*;

use processor::{Cpu, handle_interrupts};
use gpu::{Ppu, update_ppu};
use memory::Memory;
use processor::run;
use timer::update_timer;
use util::INTERRUPT_F_ADDRESS;

pub fn joypad_interrupt(mem: &mut Memory) {
    let mut interrupt = false;
    interrupt |= is_key_pressed(KeyCode::A);
    interrupt |= is_key_pressed(KeyCode::B);
    interrupt |= is_key_pressed(KeyCode::Enter);
    interrupt |= is_key_pressed(KeyCode::Space);
    interrupt |= is_key_pressed(KeyCode::Left);
    interrupt |= is_key_pressed(KeyCode::Right);
    interrupt |= is_key_pressed(KeyCode::Up);
    interrupt |= is_key_pressed(KeyCode::Down);

    if interrupt {
        let interrupt = mem.read(INTERRUPT_F_ADDRESS);
        mem.write(INTERRUPT_F_ADDRESS, interrupt|0b0001_0000);
    }
}

pub fn joypad(joypad: u8) -> u8 {
    let mut upper_joypad = joypad & 0b1111_0000;
    // neither buttons nor d-pad is selected
    if upper_joypad & 0x30 == 0x30 {
        return upper_joypad | 0xF;
    }
    if upper_joypad & 0b0001_00000 == 0 {
        upper_joypad |= (!is_key_down(KeyCode::A) as u8) << 0;
        upper_joypad |= (!is_key_down(KeyCode::S) as u8) << 1;
        upper_joypad |= (!is_key_down(KeyCode::Space) as u8) << 2;
        upper_joypad |= (!is_key_down(KeyCode::Enter) as u8) << 3;
        return upper_joypad;
    }
    upper_joypad |= (!is_key_down(KeyCode::Right) as u8) << 0;
    upper_joypad |= (!is_key_down(KeyCode::Left) as u8) << 1;
    upper_joypad |= (!is_key_down(KeyCode::Up) as u8) << 2;
    upper_joypad |= (!is_key_down(KeyCode::Down) as u8) << 3;
    return upper_joypad;
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
    let mut booted = true;

    if args.len() == 1 {
        panic!("invalid number of arguments");
    }
    let rom_path = &args[1];
    if args.len() == 3 && args[2] == "--unbooted" {
        booted = false;
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
            
            // handles all of the interrupts
            joypad_interrupt(&mut memory);
            handle_interrupts(&mut cpu, &mut memory);


            // halt still increments the cycles and timer
            // runs the instruction otherwise
            let mut cycles = 4;
            if !cpu.halt {
                cycles = run(&mut cpu, &mut memory);
            }
            update_timer(&mut memory, cycles);
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
            break 'full;
        }
    }
}
