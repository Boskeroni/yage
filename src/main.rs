mod opcodes;
mod registers;
mod memory;
mod util;

use std::{env, fs::{File, OpenOptions}, io::Write};

use memory::Memory;
use opcodes::run_opcode;
use registers::Registers;

#[derive(Default, Debug)]
struct Cpu {
    regs: Registers,
    _ime: bool,
    scheduled_ime: bool,
    stopped: bool,
    halt: bool,
}

fn main() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(get_rom());

    // just clears the log file for each runthrough
    File::create("gameboy_doctor.txt").unwrap();

    let mut file = OpenOptions::new()
    .append(true)
    .open("gameboy_doctor.tXt")
    .expect("cannot open file");

    loop {
        gameboy_doctor_logger(&cpu, &memory, &mut file);
        let _cycles = run_opcode(&mut cpu, &mut memory);

        // used for outputs during blarggs tests and since thatll be
        // all the gameboy roms ill be running for a while no point
        // in it being a seperate function. itll be easily deletable later
        if memory.read(0xFF02) == 0x81 {
            let c = memory.read(0xFF01) as char;
            print!("{c}");
            memory.write(0xFF02, 0);
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

/// this is for debug purposes. converts my emulator's state into 
/// a way that the gameboy-doctor process can check for errors
fn gameboy_doctor_logger(cpu: &Cpu, memory: &Memory, file: &mut File) {
    let a = cpu.regs.a;
    let f = cpu.regs.f.into_u8();
    let b = cpu.regs.b;
    let c = cpu.regs.c;
    let d = cpu.regs.d;
    let e = cpu.regs.e;
    let h = cpu.regs.h;
    let l = cpu.regs.l;
    let sp = cpu.regs.sp;
    let pc = cpu.regs.pc;
    let mem1 = memory.read(pc);
    let mem2 = memory.read(pc+1);
    let mem3 = memory.read(pc+2);
    let mem4 = memory.read(pc+3);
    let log = format!("A:{a:02X} F:{f:02X} B:{b:02X} C:{c:02X} D:{d:02X} E:{e:02X} H:{h:02X} L:{l:02X} SP:{sp:04X} PC:{pc:04X} PCMEM:{mem1:02X},{mem2:02X},{mem3:02X},{mem4:02X}\n");
    
    file.write_all(log.as_bytes()).unwrap();
}