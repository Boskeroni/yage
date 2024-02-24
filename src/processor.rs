use crate::memory::Memory;
use crate::cpu::*;

use crate::opcodes::*;
use crate::util::INTERRUPT_E_ADDRESS;
use crate::util::INTERRUPT_F_ADDRESS;

const VEC_ADDRESSES: [u16; 5] = [0x40, 0x48, 0x50, 0x58, 0x60];

/// check if the interrupt handler is memory  or not
/// could be automatically done without needing timer updates
pub fn handle_interrupts(cpu: &mut Cpu, memory: &mut Memory) -> u8 {
    let interrupts_called = memory.read(INTERRUPT_F_ADDRESS);
    let possible_interrupts = interrupts_called & memory.read(INTERRUPT_E_ADDRESS);

    if cpu.halt && possible_interrupts != 0 {
        cpu.halt = false;
        return 0;
    }
    if !cpu.ime || possible_interrupts == 0 {
        return 0;
    }

    // interrupts are handled right to left
    let priority = possible_interrupts.trailing_zeros();
    let address = VEC_ADDRESSES[priority as usize];

    rst(cpu, memory, address);

    // reset the ime, schedule, and unset the interrupt
    cpu.ime = false;
    cpu.scheduled_ime = false;

    // unset this interrupt bit
    let new_interrupt = interrupts_called & !(1<<priority);
    memory.write(INTERRUPT_F_ADDRESS, new_interrupt);
    return 20;
}

/// this handles all the opcodes for the gameboy. It returns the number of T-cycles which were used to 
pub fn run(cpu: &mut Cpu, memory: &mut Memory) -> u8 {
    // the scheduled ime only takes place after the next instruction
    let temp_ime = cpu.scheduled_ime;
    let opcode = memory.read(cpu.regs.pc());

    let used_cycles = match opcode {
        0xCB => {prefixed_opcode(cpu, memory); 8},
        _ => unprefixed_opcode(cpu, memory, opcode),
    };

    // the scheduled ime didnt change during this instruction
    if temp_ime == cpu.scheduled_ime {
        cpu.ime = cpu.scheduled_ime;
    }
    used_cycles
}

fn unprefixed_opcode(cpu: &mut Cpu, mem: &mut Memory, opcode: u8) -> u8 {
    match opcode {
        0x00 => { 4 } //NOP
        0x01 => {let pc = cpu.regs.pc_word(); cpu.regs.set_bc(mem.read_word(pc)); 12}, // LC BC, n16
        0x02 => {mem.write(cpu.regs.get_bc(), cpu.regs.a); 8}, // LD [BC], A
        0x03 => {cpu.regs.set_bc(cpu.regs.get_bc().wrapping_add(1)); 8}, // INC BC
        0x04 => {inc(&mut cpu.regs.b, &mut cpu.regs.f); 4}, // INC B
        0x05 => {dec(&mut cpu.regs.b, &mut cpu.regs.f); 4}, // DEC B
        0x06 => {cpu.regs.b = mem.read(cpu.regs.pc()); 8}, // LD B, n8
        0x07 => {rlc(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); 4} // RLCA
        0x08 => {let addr = mem.read_word(cpu.regs.pc_word()); mem.write_word(addr, cpu.regs.sp); 20}, // LD [a16], SP
        0x09 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_bc(), &mut cpu.regs.f); cpu.regs.set_hl(hl); 8}, // ADD HL, BC
        0x0A => {cpu.regs.a = mem.read(cpu.regs.get_bc()); 8}, // LD A, [BC]
        0x0B => {cpu.regs.set_bc(cpu.regs.get_bc().wrapping_sub(1)); 8}, // DEC BC
        0x0C => {inc(&mut cpu.regs.c, &mut cpu.regs.f); 4}, // INC C
        0x0D => {dec(&mut cpu.regs.c, &mut cpu.regs.f); 4}, // DEC C
        0x0E => {cpu.regs.c = mem.read(cpu.regs.pc()); 8}, // LC C, n8
        0x0F => {rrc(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); 4}, // RRCA
        0x10 => {cpu.stopped = true; cpu.regs.pc(); mem.write(0xFF04, 0); 4} // STOP n8
        0x11 => {let pc = cpu.regs.pc_word(); cpu.regs.set_de(mem.read_word(pc)); 12}, // LD DE, n16
        0x12 => {mem.write(cpu.regs.get_de(), cpu.regs.a); 8}, // LD [DE], A
        0x13 => {cpu.regs.set_de(cpu.regs.get_de().wrapping_add(1)); 8}, // INC DE
        0x14 => {inc(&mut cpu.regs.d, &mut cpu.regs.f); 4}, // INC D
        0x15 => {dec(&mut cpu.regs.d, &mut cpu.regs.f); 4}, // DEC D
        0x16 => {cpu.regs.d = mem.read(cpu.regs.pc()); 8}, // LD D, n8
        0x17 => {rl(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); 4} // RLA
        0x18 => {let new = mem.read(cpu.regs.pc()); jr(cpu, true, new); 12}, // JR e8
        0x19 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_de(), &mut cpu.regs.f); cpu.regs.set_hl(hl); 8}, // ADD HL, DE
        0x1A => {cpu.regs.a = mem.read(cpu.regs.get_de()); 8}, // LD A, [DE]
        0x1B => {cpu.regs.set_de(cpu.regs.get_de().wrapping_sub(1)); 8}, // DEC DE
        0x1C => {inc(&mut cpu.regs.e, &mut cpu.regs.f); 4}, // INC E
        0x1D => {dec(&mut cpu.regs.e, &mut cpu.regs.f); 4}, // DEC E
        0x1E => {cpu.regs.e = mem.read(cpu.regs.pc()); 8}, // LD E, n8
        0x1F => {rr(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); 4} // RRA
        0x20 => {let new = mem.read(cpu.regs.pc()); let cycles = jr(cpu, !cpu.regs.f.z(), new); cycles}, // JR NZ, e8
        0x21 => {let pc = cpu.regs.pc_word(); cpu.regs.set_hl(mem.read_word(pc)); 12}, // LD HL, n16
        0x22 => {mem.write(cpu.regs.get_hli(), cpu.regs.a); 8}, // LD [HL+], A
        0x23 => {cpu.regs.get_hli(); 8}, // INC HL
        0x24 => {inc(&mut cpu.regs.h, &mut cpu.regs.f); 4}, // INC H
        0x25 => {dec(&mut cpu.regs.h, &mut cpu.regs.f); 4}, // DEC H
        0x26 => {cpu.regs.h = mem.read(cpu.regs.pc()); 8}, // LD H, n8
        0x27 => {daa(&mut cpu.regs.a, &mut cpu.regs.f); 4}, // DAA
        0x28 => {let new = mem.read(cpu.regs.pc()); let cycles = jr(cpu, cpu.regs.f.z(), new); cycles}, // JR Z, e8
        0x29 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_hl(), &mut cpu.regs.f); cpu.regs.set_hl(hl); 8}, // ADD HL, HL
        0x2A => {cpu.regs.a = mem.read(cpu.regs.get_hli()); 8}, // LD A, [HL+]
        0x2B => {cpu.regs.get_hld(); 8}, // DEC HL
        0x2C => {inc(&mut cpu.regs.l, &mut cpu.regs.f); 4}, // INC L
        0x2D => {dec(&mut cpu.regs.l, &mut cpu.regs.f); 4}, // DEC L
        0x2E => {cpu.regs.l = mem.read(cpu.regs.pc()); 8}, // LD L, n8
        0x2F => {cpl(&mut cpu.regs.a, &mut cpu.regs.f); 4}, // CPL
        0x30 => {let new = mem.read(cpu.regs.pc()); let cycles = jr(cpu, !cpu.regs.f.c(), new); cycles}, // JR NC, e8
        0x31 => {cpu.regs.sp = mem.read_word(cpu.regs.pc_word()); 12}, // LD SP, n16
        0x32 => {mem.write(cpu.regs.get_hld(), cpu.regs.a); 8}, // LD [HL-], A
        0x33 => {cpu.regs.sp = cpu.regs.sp.wrapping_add(1); 8}, // INC SP
        0x34 => {let mut hl = mem.read(cpu.regs.get_hl()); inc(&mut hl, &mut cpu.regs.f); mem.write(cpu.regs.get_hl(), hl); 12}, // INC [HL]
        0x35 => {let mut hl = mem.read(cpu.regs.get_hl()); dec(&mut hl, &mut cpu.regs.f); mem.write(cpu.regs.get_hl(), hl); 12}, // DEC [HL]
        0x36 => {let new = mem.read(cpu.regs.pc()); mem.write(cpu.regs.get_hl(), new); 12}, // LD [HL], n8
        0x37 => {scf(&mut cpu.regs.f); 4}, // SCF
        0x38 => {let new = mem.read(cpu.regs.pc()); let cycles = jr(cpu, cpu.regs.f.c(), new); cycles}, // JR C, e8
        0x39 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.sp, &mut cpu.regs.f); cpu.regs.set_hl(hl); 8}, // ADD HL, SP
        0x3A => {cpu.regs.a = mem.read(cpu.regs.get_hld()); 8}, // LD A, [HL-]
        0x3B => {cpu.regs.sp = cpu.regs.sp.wrapping_sub(1); 4}, // DEC SP
        0x3C => {inc(&mut cpu.regs.a, &mut cpu.regs.f); 4}, // INC A
        0x3D => {dec(&mut cpu.regs.a, &mut cpu.regs.f); 4}, // DEC A
        0x3E => {cpu.regs.a = mem.read(cpu.regs.pc()); 8}, // LD A, n8
        0x3F => {ccf(&mut cpu.regs.f); 4}, // CCD
        0x76 => {
            if cpu.ime {
                cpu.halt = true;
                return 4
            }
            if mem.read(INTERRUPT_E_ADDRESS) & mem.read(INTERRUPT_F_ADDRESS) == 0 { 
                cpu.halt = true; 
            }
            // halt bug occured
            return 4
        }, // HALT
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
                6 => mem.read(cpu.regs.get_hl()),
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
                    mem.write(cpu.regs.get_hl(), data);
                    return 8
                }
                7 => &mut cpu.regs.a,
                _ => unreachable!(),
            };
            *transfer = data;
            if src == 6 { 8 } else { 4 }
        }, // LD {reg}, {reg}
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
                6 => mem.read(cpu.regs.get_hl()),
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
            if src == 6 { 8 } else { 4 }
        }, // {func} A, {reg}
        0xC0 => {let cycles = ret(cpu, !cpu.regs.f.z(), mem); cycles}, // RET NZ
        0xC1 => {cpu.regs.set_bc(mem.read_word(cpu.regs.sp)); cpu.regs.sp += 2; 12}, // POP BC
        0xC2 => {let new = mem.read_word(cpu.regs.pc_word()); let cycles = jp(cpu, !cpu.regs.f.z(), new); cycles}, // JP NZ, a16
        0xC3 => {let new = mem.read_word(cpu.regs.pc_word()); let cycles = jp(cpu, true, new); cycles}, // JP a16
        0xC4 => {let cycles = call(cpu, !cpu.regs.f.z(), mem); cycles}, // CALL NZ, a16
        0xC5 => {mem.write_word(cpu.regs.sp-2, cpu.regs.get_bc()); cpu.regs.sp -= 2; 16}, // PUSH BC
        0xC6 => {let data = mem.read(cpu.regs.pc()); add(&mut cpu.regs.a, data, &mut cpu.regs.f); 8} // ADD A, n8
        0xC7 => {rst(cpu, mem, 0x00); 16}, // RST $00
        0xC8 => {let cycles = ret(cpu, cpu.regs.f.z(), mem); cycles}, // RET Z
        0xC9 => {let cycles = ret(cpu, true, mem); cycles} // RET
        0xCA => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, cpu.regs.f.z(), mem.read_word(pc)); cycles} // JP Z, a16
        0xCC => {let cycles = call(cpu, cpu.regs.f.z(), mem); cycles} // CALL Z, a16
        0xCD => {let cycles = call(cpu, true, mem); cycles}, // CALL a16
        0xCE => {let data = cpu.regs.pc(); adc(&mut cpu.regs.a, mem.read(data), &mut cpu.regs.f); 8}, // ADC A, n8
        0xCF => {rst(cpu, mem, 0x08); 16}, // RST $08
        0xD0 => {let cycles = ret(cpu, !cpu.regs.f.c(), mem); cycles}, // RET NC
        0xD1 => {cpu.regs.set_de(mem.read_word(cpu.regs.sp)); cpu.regs.sp += 2; 12}, // POP DE
        0xD2 => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, !cpu.regs.f.c(), mem.read_word(pc)); cycles}, // JP NC, a16
        0xD4 => {let cycles = call(cpu, !cpu.regs.f.c(), mem); cycles}, // CALL NC, a16
        0xD5 => {mem.write_word(cpu.regs.sp-2, cpu.regs.get_de()); cpu.regs.sp -= 2; 16} // PUSH DE
        0xD6 => {let data = mem.read(cpu.regs.pc()); sub(&mut cpu.regs.a, data, &mut cpu.regs.f); 8} // SUB A, n8
        0xD7 => {rst(cpu, mem, 0x10); 16} // RST $10
        0xD8 => {let cycles = ret(cpu, cpu.regs.f.c(), mem); cycles}, // RET C
        0xD9 => {let cycles = ret(cpu, true, mem); cpu.scheduled_ime=true; cpu.ime = true; cycles} // RETI
        0xDA => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, cpu.regs.f.c(), mem.read_word(pc)); cycles}, // JP C, a16
        0xDC => {let cycles = call(cpu, cpu.regs.f.c(), mem); cycles}, // CALL C, a16
        0xDE => {let data = mem.read(cpu.regs.pc()); sbc(&mut cpu.regs.a, data, &mut cpu.regs.f); 8}, // SBC A, n8
        0xDF => {rst(cpu, mem, 0x18); 16}, // RST $18
        0xE0 => {let address = mem.read(cpu.regs.pc()) as u16 + 0xFF00; mem.write(address, cpu.regs.a); 12} // LDH [a8], A
        0xE1 => {cpu.regs.set_hl(mem.read_word(cpu.regs.sp)); cpu.regs.sp += 2; 12}, // POP HL
        0xE2 => {let address = cpu.regs.c as u16 + 0xFF00; mem.write(address, cpu.regs.a); 8}, // LD [C], A
        0xE5 => {mem.write_word(cpu.regs.sp-2, cpu.regs.get_hl()); cpu.regs.sp -= 2; 16}, // PUSH HL
        0xE6 => {let data = mem.read(cpu.regs.pc()); and(&mut cpu.regs.a, data, &mut cpu.regs.f); 8}, // AND A, n8
        0xE7 => {rst(cpu, mem, 0x20); 16}, // RST $20 
        0xE8 => {let data = mem.read(cpu.regs.pc()) as i8; add_u16_i8(&mut cpu.regs.sp, data, &mut cpu.regs.f); 16}, // ADD SP, e8
        0xE9 => {jp(cpu, true, cpu.regs.get_hl()); 4}, // JP HL
        0xEA => {let address = mem.read_word(cpu.regs.pc_word()); mem.write(address, cpu.regs.a); 16} // LD [a16], A
        0xEE => {let data = mem.read(cpu.regs.pc()); xor(&mut cpu.regs.a, data, &mut cpu.regs.f); 8} // XOR A, n8
        0xEF => {rst(cpu, mem, 0x28); 16}, // RST $28
        0xF0 => {let data_address = mem.read(cpu.regs.pc()) as u16 + 0xFF00; cpu.regs.a = mem.read(data_address); 12}, // LDH A, [a8]
        0xF1 => {cpu.regs.set_af(mem.read_word(cpu.regs.sp)); cpu.regs.sp += 2; 12}, // POP AF
        0xF2 => {let data_address = cpu.regs.c as u16 + 0xFF00; cpu.regs.a = mem.read(data_address); 8}, // LD A, [C]
        0xF3 => {cpu.scheduled_ime=false; cpu.ime = false; 4}, // DI
        0xF5 => {mem.write_word(cpu.regs.sp-2, cpu.regs.get_af()); cpu.regs.sp -= 2; 16}, // PUSH AF
        0xF6 => {let data = mem.read(cpu.regs.pc()); or(&mut cpu.regs.a, data, &mut cpu.regs.f); 8}, // OR A, n8
        0xF7 => {rst(cpu, mem, 0x30); 16}, // RST $30
        0xF8 => {let data = mem.read(cpu.regs.pc()) as i8; set_add_u16_i8(cpu, data); 12}, // LD HL, SP + e8
        0xF9 => {cpu.regs.sp = cpu.regs.get_hl(); 8}, // LD SP, HL
        0xFA => {let address = mem.read_word(cpu.regs.pc_word()); cpu.regs.a = mem.read(address); 16} // LD A, [a16]
        0xFB => {cpu.scheduled_ime = true; 4}, // EI
        0xFE => {let data = mem.read(cpu.regs.pc()); cp(&mut cpu.regs.a, data, &mut cpu.regs.f); 8}, // CP A, n8
        0xFF => {rst(cpu, mem, 0x38); 16} // RST $38
        _ => panic!("unsupported opcode"),
    }
}
fn prefixed_opcode(cpu: &mut Cpu, memory: &mut Memory) {
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

    let opcode = memory.read(cpu.regs.pc());
    let target = opcode % 8;
    let operation = opcode / 8;

    if target == 6 {
        let hl = cpu.regs.get_hl();

        let mut value = memory.read(hl);
        run_operation(&mut value, operation, &mut cpu.regs.f);
        memory.write(hl, value);
        return;
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
}
