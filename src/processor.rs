use crate::memory::Memory;
use crate::registers::*;

use crate::opcodes::*;

#[derive(Default, Debug)]
pub struct Cpu {
    pub regs: Registers,
    ime: bool,
    scheduled_ime: bool,
    stopped: bool,
    pub halt: bool,
}
impl Cpu {
    pub fn new(booted: bool) -> Self {
        let regs;
        if booted {
            regs = Registers::default();
        } else {
            regs = Registers::new();
        }

        Self {
            regs,
            ..Default::default()
        }
    }
}

/// check if the interrupt handler is memory  or not
/// could be automatically done without needing timer updates
pub fn handle_interrupts(cpu: &mut Cpu, memory: &mut Memory) {
    let possible_interrupts = memory.read(0xFFFF) & memory.read(0xFF0F);

    if cpu.halt && possible_interrupts != 0 {
        cpu.halt = false;
    }
    if !cpu.ime || possible_interrupts == 0 {
        return
    }

    // interrupts are handled right to left
    let priority = possible_interrupts.trailing_zeros();
    let address = match priority {
        0 => 0x40,
        1 => 0x48,
        2 => 0x50,
        3 => 0x58,
        4 => 0x60,
        _ => panic!("invalid interrupt request made"),
    };
    rst(cpu, memory, address);

    // reset the ime, schedule, and unset the interrupt
    cpu.ime = false;
    cpu.scheduled_ime = false;

    // unset this interrupt bit
    let new_interrupt = memory.read(0xFF0F) & !(1<<priority);
    memory.write(0xFF0F, new_interrupt);
}

/// this handles all the opcodes for the gameboy. It returns the number of T-cycles which were used to 
pub fn run(cpu: &mut Cpu, memory: &mut Memory) -> u8 {
    // the scheduled ime only takes place after the next instruction
    let temp_ime = cpu.scheduled_ime;
    let opcode = memory.read(cpu.regs.pc());

    let used_cycles = match opcode {
        0xCB => prefixed_opcode(cpu, memory),
        _ => unprefixed_opcode(cpu, memory, opcode),
    };

    // the scheduled ime didnt change during this instruction
    if temp_ime == cpu.scheduled_ime {
        cpu.ime = cpu.scheduled_ime;
    }

    return used_cycles;
}

fn unprefixed_opcode(cpu: &mut Cpu, memory: &mut Memory, opcode: u8) -> u8 {
    match opcode {
        0x00 => { return 4}
        0x01 => {let pc = cpu.regs.pc_word(); cpu.regs.set_bc(memory.read_word(pc)); return 12},
        0x02 => {memory.write(cpu.regs.get_bc(), cpu.regs.a); return 8},
        0x03 => {cpu.regs.set_bc(cpu.regs.get_bc().wrapping_add(1)); return 8},
        0x04 => {inc(&mut cpu.regs.b, &mut cpu.regs.f); return 4},
        0x05 => {dec(&mut cpu.regs.b, &mut cpu.regs.f); return 4},
        0x06 => {cpu.regs.b = memory.read(cpu.regs.pc()); return 8},
        0x07 => {rlc(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); return 4}
        0x08 => {let addr = memory.read_word(cpu.regs.pc_word()); memory.write_word(addr, cpu.regs.sp); return 20},
        0x09 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_bc(), &mut cpu.regs.f); cpu.regs.set_hl(hl); return 8},
        0x0A => {cpu.regs.a = memory.read(cpu.regs.get_bc()); return 8},
        0x0B => {cpu.regs.set_bc(cpu.regs.get_bc().wrapping_sub(1)); return 8},
        0x0C => {inc(&mut cpu.regs.c, &mut cpu.regs.f); return 4},
        0x0D => {dec(&mut cpu.regs.c, &mut cpu.regs.f); return 4},
        0x0E => {cpu.regs.c = memory.read(cpu.regs.pc()); return 8},
        0x0F => {rrc(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); return 4}
        0x10 => {cpu.stopped = true; cpu.regs.pc(); memory.write(0xFF05, 0); return 4} 
        0x11 => {let pc = cpu.regs.pc_word(); cpu.regs.set_de(memory.read_word(pc)); return 12},
        0x12 => {memory.write(cpu.regs.get_de(), cpu.regs.a); return 8},
        0x13 => {cpu.regs.set_de(cpu.regs.get_de().wrapping_add(1)); return 8},
        0x14 => {inc(&mut cpu.regs.d, &mut cpu.regs.f); return 4},
        0x15 => {dec(&mut cpu.regs.d, &mut cpu.regs.f); return 4},
        0x16 => {cpu.regs.d = memory.read(cpu.regs.pc()); return 8},
        0x17 => {rl(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); return 4}
        0x18 => {let new = memory.read(cpu.regs.pc()); jr(cpu, true, new); return 12},
        0x19 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_de(), &mut cpu.regs.f); cpu.regs.set_hl(hl); return 8;},
        0x1A => {cpu.regs.a = memory.read(cpu.regs.get_de()); return 8},
        0x1B => {cpu.regs.set_de(cpu.regs.get_de().wrapping_sub(1)); return 8},
        0x1C => {inc(&mut cpu.regs.e, &mut cpu.regs.f); return 4},
        0x1D => {dec(&mut cpu.regs.e, &mut cpu.regs.f); return 4},
        0x1E => {cpu.regs.e = memory.read(cpu.regs.pc()); return 8},
        0x1F => {rr(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); return 4}
        0x20 => {let new = memory.read(cpu.regs.pc()); let cycles = jr(cpu, !cpu.regs.f.z(), new); return cycles}, 
        0x21 => {let pc = cpu.regs.pc_word(); cpu.regs.set_hl(memory.read_word(pc)); return 12},
        0x22 => {memory.write(cpu.regs.get_hli(), cpu.regs.a); return 8},
        0x23 => {cpu.regs.get_hli(); return 8},
        0x24 => {inc(&mut cpu.regs.h, &mut cpu.regs.f); return 4},
        0x25 => {dec(&mut cpu.regs.h, &mut cpu.regs.f); return 4},
        0x26 => {cpu.regs.h = memory.read(cpu.regs.pc()); return 8},
        0x27 => {daa(&mut cpu.regs.a, &mut cpu.regs.f); return 4},
        0x28 => {let new = memory.read(cpu.regs.pc()); let cycles = jr(cpu, cpu.regs.f.z(), new); return cycles},
        0x29 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_hl(), &mut cpu.regs.f); cpu.regs.set_hl(hl); return 8},
        0x2A => {cpu.regs.a = memory.read(cpu.regs.get_hli()); return 8},
        0x2B => {cpu.regs.get_hld(); return 8},
        0x2C => {inc(&mut cpu.regs.l, &mut cpu.regs.f); return 4},
        0x2D => {dec(&mut cpu.regs.l, &mut cpu.regs.f); return 4},
        0x2E => {cpu.regs.l = memory.read(cpu.regs.pc()); return 8},
        0x2F => {cpl(&mut cpu.regs.a, &mut cpu.regs.f); return 4},
        0x30 => {let new = memory.read(cpu.regs.pc()); let cycles = jr(cpu, !cpu.regs.f.c(), new); return cycles},
        0x31 => {cpu.regs.sp = memory.read_word(cpu.regs.pc_word()); return 12},
        0x32 => {memory.write(cpu.regs.get_hld(), cpu.regs.a); return 8},
        0x33 => {cpu.regs.sp = cpu.regs.sp.wrapping_add(1); return 8},
        0x34 => {let mut hl = memory.read(cpu.regs.get_hl()); inc(&mut hl, &mut cpu.regs.f); memory.write(cpu.regs.get_hl(), hl); return 12},
        0x35 => {let mut hl = memory.read(cpu.regs.get_hl()); dec(&mut hl, &mut cpu.regs.f); memory.write(cpu.regs.get_hl(), hl); return 12},
        0x36 => {let new = memory.read(cpu.regs.pc()); memory.write(cpu.regs.get_hl(), new); return 12},
        0x37 => {scf(&mut cpu.regs.f); return 4},
        0x38 => {let new = memory.read(cpu.regs.pc()); let cycles = jr(cpu, cpu.regs.f.c(), new); return cycles},
        0x39 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.sp, &mut cpu.regs.f); cpu.regs.set_hl(hl); return 8},
        0x3A => {cpu.regs.a = memory.read(cpu.regs.get_hld()); return 8},
        0x3B => {cpu.regs.sp = cpu.regs.sp.wrapping_sub(1); return 4},
        0x3C => {inc(&mut cpu.regs.a, &mut cpu.regs.f); return 4},
        0x3D => {dec(&mut cpu.regs.a, &mut cpu.regs.f); return 4},
        0x3E => {cpu.regs.a = memory.read(cpu.regs.pc()); return 8},
        0x3F => {ccf(&mut cpu.regs.f); return 4},
        0x76 => {cpu.halt = true; return 4},
        0x40..=0x7F => {
            let adjusted_opcode = opcode - 0x40;
            let src = adjusted_opcode % 8;
            let dst = adjusted_opcode / 8;

            let data = match src {
                0 => cpu.regs.b,
                1 => cpu.regs.c,
                2 => cpu.regs.d,
                3 => cpu.regs.e,
                4 => cpu.regs.h,
                5 => cpu.regs.l,
                6 => memory.read(cpu.regs.get_hl()),
                7 => cpu.regs.a,
                _ => unreachable!(),
            };
            let transfer = match dst {
                0 => &mut cpu.regs.b,
                1 => &mut cpu.regs.c,
                2 => &mut cpu.regs.d,
                3 => &mut cpu.regs.e,
                4 => &mut cpu.regs.h,
                5 => &mut cpu.regs.l,
                6 => {
                    memory.write(cpu.regs.get_hl(), data);
                    return 8;
                }
                7 => &mut cpu.regs.a,
                _ => unreachable!(),
            };
            *transfer = data;
            return if src == 6 { 8 } else { 4 };
        },
        0x80..=0xBF => {
            let adjusted_opcode = opcode - 0x80;
            let src = adjusted_opcode % 8;
            let operation = adjusted_opcode / 8;

            let data = match src {
                0 => cpu.regs.b,
                1 => cpu.regs.c,
                2 => cpu.regs.d,
                3 => cpu.regs.e,
                4 => cpu.regs.h,
                5 => cpu.regs.l,
                6 => memory.read(cpu.regs.get_hl()),
                7 => cpu.regs.a,
                _ => unreachable!()
            };
            let func = match operation {
                0 => add,
                1 => adc,
                2 => sub,
                3 => sbc,
                4 => and,
                5 => xor,
                6 => or,
                7 => cp,
                _ => unreachable!(),
            };
            func(&mut cpu.regs.a, data, &mut cpu.regs.f);
            return if src == 6 { 8 } else { 4 };
        },
        0xC0 => {let cycles = ret(cpu, !cpu.regs.f.z(), memory); return cycles},
        0xC1 => {cpu.regs.set_bc(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12},
        0xC2 => {let new = memory.read_word(cpu.regs.pc_word()); let cycles = jp(cpu, !cpu.regs.f.z(), new); return cycles},
        0xC3 => {let new = memory.read_word(cpu.regs.pc_word()); let cycles = jp(cpu, true, new); return cycles},
        0xC4 => {let cycles = call(cpu, !cpu.regs.f.z(), memory); return cycles},
        0xC5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_bc()); cpu.regs.sp -= 2; return 16},
        0xC6 => {let data = memory.read(cpu.regs.pc()); add(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8}
        0xC7 => {rst(cpu, memory, 0x00); return 16},
        0xC8 => {let cycles = ret(cpu, cpu.regs.f.z(), memory); return cycles},
        0xC9 => {let cycles = ret(cpu, true, memory); cpu.ime = true; return cycles}
        0xCA => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, cpu.regs.f.z(), memory.read_word(pc)); return cycles}
        0xCC => {let cycles = call(cpu, cpu.regs.f.z(), memory); return cycles}
        0xCD => {let cycles = call(cpu, true, memory); return cycles},
        0xCE => {let data = cpu.regs.pc(); adc(&mut cpu.regs.a, memory.read(data), &mut cpu.regs.f); },
        0xCF => {rst(cpu, memory, 0x08); },
        0xD0 => {let cycles = ret(cpu, !cpu.regs.f.c(), memory); return cycles},
        0xD1 => {cpu.regs.set_de(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12},
        0xD2 => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, !cpu.regs.f.c(), memory.read_word(pc)); return cycles}
        0xD4 => {let cycles = call(cpu, !cpu.regs.f.c(), memory); return cycles},
        0xD5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_de()); cpu.regs.sp -= 2; return 16}
        0xD6 => {let data = memory.read(cpu.regs.pc()); sub(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8}
        0xD7 => {rst(cpu, memory, 0x10); return 16}
        0xD8 => {let cycles = ret(cpu, cpu.regs.f.c(), memory); return cycles},
        0xD9 => {let cycles = ret(cpu, true, memory); cpu.scheduled_ime=true; return cycles}
        0xDA => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, cpu.regs.f.c(), memory.read_word(pc)); return cycles},
        0xDC => {let cycles = call(cpu, cpu.regs.f.c(), memory); return cycles},
        0xDE => {let data = memory.read(cpu.regs.pc()); sbc(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8},
        0xDF => {rst(cpu, memory, 0x18); return 16},
        0xE0 => {let address = memory.read(cpu.regs.pc()) as u16 + 0xFF00; memory.write(address, cpu.regs.a); return 12}
        0xE1 => {cpu.regs.set_hl(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12},
        0xE2 => {let address = cpu.regs.c as u16 + 0xFF00; memory.write(address, cpu.regs.a); return 8},
        0xE5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_hl()); cpu.regs.sp -= 2; return 16},
        0xE6 => {let data = memory.read(cpu.regs.pc()); and(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8},
        0xE7 => {rst(cpu, memory, 0x20); return 16},
        0xE8 => {let data = memory.read(cpu.regs.pc()) as i8; add_u16_i8(&mut cpu.regs.sp, data, &mut cpu.regs.f); return 16},
        0xE9 => {jp(cpu, true, cpu.regs.get_hl()); return 4},
        0xEA => {let address = memory.read_word(cpu.regs.pc_word()); memory.write(address, cpu.regs.a); return 16}
        0xEE => {let data = memory.read(cpu.regs.pc()); xor(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8}
        0xEF => {rst(cpu, memory, 0x28); return 16},
        0xF0 => {let data_address = memory.read(cpu.regs.pc()) as u16 + 0xFF00; cpu.regs.a = memory.read(data_address); return 12},
        0xF1 => {cpu.regs.set_af(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12},
        0xF2 => {let data_address = cpu.regs.c as u16 + 0xFF00; cpu.regs.a = memory.read(data_address); return 8},
        0xF3 => {cpu.scheduled_ime=false; return 4},
        0xF5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_af()); cpu.regs.sp -= 2; return 16},
        0xF6 => {let data = memory.read(cpu.regs.pc()); or(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8},
        0xF7 => {rst(cpu, memory, 0x30); return 16},
        0xF8 => {let data = memory.read(cpu.regs.pc()) as i8; set_add_u16_i8(cpu, data); return 12},
        0xF9 => {cpu.regs.sp = cpu.regs.get_hl(); return 8},
        0xFA => {let address = memory.read_word(cpu.regs.pc_word()); cpu.regs.a = memory.read(address); return 16}
        0xFB => {cpu.scheduled_ime = true; return 4},
        0xFE => {let data = memory.read(cpu.regs.pc()); cp(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8},
        0xFF => {rst(cpu, memory, 0x38); return 16}
        _ => panic!("unsupported opcode"),
    }
    return 0;
}





fn prefixed_opcode(cpu: &mut Cpu, memory: &mut Memory) -> u8 {
    fn run_operation(data: &mut u8, operation: u8, flag: &mut Flag) {
        match operation {
            0 => rlc(data, flag),
            1 => rrc(data, flag),
            2 => rl(data, flag),
            3 => rr(data, flag),
            4 => sla(data, flag),
            5 => sra(data, flag),
            6 => swap(data, flag),
            7 => srl(data, flag),
            8..=15 => bit(data, flag, operation - 8),
            16..=23 => *data &= !(0b0000_0001 << (operation - 16)),
            24..=31 => *data |= 0b0000_0001 << (operation - 24),
            _ => unreachable!()
        }
    }

    // the new opcode pretty much
    let opcode = memory.read(cpu.regs.pc());

    // the register which the operation is going to be performed on
    // selected through match statement
    let target = opcode % 8;

    // the operation which will be performed. again used in match statement.
    let operation = opcode / 8;

    // meaning we are changing the [hl]
    if target == 6 {
        let hl = cpu.regs.get_hl();

        let mut value = memory.read(hl);
        run_operation(&mut value, operation, &mut cpu.regs.f);

        // these operations only modify one bit of the memory and so
        // the gameboy doesnt require the full 4 cycles to run it.
        if operation >= 8 && operation <= 15 {
            memory.write(hl, value);
        } else {
            memory.write(hl, value);
        }
        return 0;
    }
    let src = match target {
        0 => &mut cpu.regs.b,
        1 => &mut cpu.regs.c,
        2 => &mut cpu.regs.d,
        3 => &mut cpu.regs.e,
        4 => &mut cpu.regs.h,
        5 => &mut cpu.regs.l,
        7 => &mut cpu.regs.a,
        _ => unreachable!(),
    };
    run_operation(src, operation, &mut cpu.regs.f);
    return 0;
}
