#![allow(unused, dead_code)]

mod processor;
mod registers;
mod memory;
mod opcodes;
mod gpu;

use std::{env, fs::{File, OpenOptions}, io::Write};

use processor::Cpu;
use gpu::Ppu;
use memory::Memory;
use processor::{run_opcode, handle_interrupts};
use registers::Registers;

const SCANLINE_CYCLES: u32 = 453;

pub fn little_endian_combine(a: u8, b: u8) -> u16 {
    (a as u16) | (b as u16) << 8
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

    let mut global_cycles = 0;
    let mut temp_cycles = 0;
    loop {
        temp_cycles = run_opcode(&mut cpu, &mut memory);


        // used for outputs during blarggs tests and since thatll be
        // all the gameboy roms ill be running for a while no point
        // in it being a seperate function. itll be easily deletable later
        if memory.read_timed(0xFF02) == 0x81 {
            let c = memory.read_timed(0xFF01) as char;
            print!("{c}");
            memory.write_timed(0xFF02, 0);
        }
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