use crate::{Cpu, memory::Memory, registers::Flag};

//https://robdor.com/2016/08/10/gameboy-emulator-half-carry-flag/ goddamn is that smart
fn half_carry_add(a: u8, b: u8) -> bool {
    (((a & 0xf) + (b & 0xf)) & 0x10) == 0x10
}
fn half_carry_sub(a: u8, b: u8) -> bool {
    // the nature of maths means an overflow is possible
    (((a & 0xf).wrapping_sub(b & 0xf)) & 0x10) == 0x10
}
fn half_carry_add_u16(a: u16, b: u16) -> bool {
    (((a & 0xfff) + (b & 0xfff)) & 0x1000) == 0x1000
}

/// this handles all the opcodes for the gameboy. It returns the number of cycles which were used to 
pub fn run_opcode(cpu: &mut Cpu, memory: &mut Memory) -> u8 {
    // each opcode needs 4 when it reads from memory
    let opcode = memory.read(cpu.regs.pc());

    match opcode {
        0xCB => return prefixed_opcode(cpu, memory),
        _ => return unprefixed_opcode(cpu, memory, opcode),
    }
}

fn unprefixed_opcode(cpu: &mut Cpu, memory: &mut Memory, opcode: u8) -> u8 {
    match opcode {
        0x00 => return 4,
        0x01 => {let pc = cpu.regs.pc_word(); cpu.regs.set_bc(memory.read_word(pc)); return 12},
        0x02 => {memory.write(cpu.regs.get_bc(), cpu.regs.a); return 8},
        0x03 => {cpu.regs.set_bc(cpu.regs.get_bc().wrapping_add(1)); return 8},
        0x04 => {inc(&mut cpu.regs.b, &mut cpu.regs.f); return 4},
        0x05 => {dec(&mut cpu.regs.b, &mut cpu.regs.f); return 4},
        0x06 => {cpu.regs.b = memory.read(cpu.regs.pc()); return 8},
        0x07 => {rlc(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); return 4}
        0x08 => {memory.write_word(memory.read_word(cpu.regs.pc_word()), cpu.regs.sp); return 20},
        0x09 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_bc(), &mut cpu.regs.f); cpu.regs.set_hl(hl); return 8},
        0x0A => {cpu.regs.a = memory.read(cpu.regs.get_bc()); return 4},
        0x0B => {cpu.regs.set_bc(cpu.regs.get_bc().wrapping_sub(1)); return 8},
        0x0C => {inc(&mut cpu.regs.c, &mut cpu.regs.f); return 4},
        0x0D => {dec(&mut cpu.regs.c, &mut cpu.regs.f); return 4},
        0x0E => {cpu.regs.c = memory.read(cpu.regs.pc()); return 8},
        0x0F => {rrc(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); return 4}
        0x10 => {cpu.stopped = true; let _ = cpu.regs.pc(); return 4;} 
        0x11 => {let pc = cpu.regs.pc_word(); cpu.regs.set_de(memory.read_word(pc)); return 12},
        0x12 => {memory.write(cpu.regs.get_de(), cpu.regs.a); return 8},
        0x13 => {cpu.regs.set_de(cpu.regs.get_de().wrapping_add(1)); return 8},
        0x14 => {inc(&mut cpu.regs.d, &mut cpu.regs.f); return 4},
        0x15 => {dec(&mut cpu.regs.d, &mut cpu.regs.f); return 4},
        0x16 => {cpu.regs.d = memory.read(cpu.regs.pc()); return 8},
        0x17 => {rl(&mut cpu.regs.a, &mut cpu.regs.f); cpu.regs.f.set_z(false); return 4}
        0x18 => {let new = memory.read(cpu.regs.pc()); let cycles = jr(cpu, true, new); return cycles},
        0x19 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.get_de(), &mut cpu.regs.f); cpu.regs.set_hl(hl); return 8},
        0x1A => {cpu.regs.a = memory.read(cpu.regs.get_de()); return 4},
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
        0x2A => {cpu.regs.a = memory.read(cpu.regs.get_hli()); return 4},
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
        0x36 => {memory.write(cpu.regs.get_hl(), memory.read(cpu.regs.pc())); return 8},
        0x37 => {scf(&mut cpu.regs.f); return 4},
        0x38 => {let new = memory.read(cpu.regs.pc()); let cycles = jr(cpu, cpu.regs.f.c(), new); return cycles},
        0x39 => {let hl = add_u16(cpu.regs.get_hl(), cpu.regs.sp, &mut cpu.regs.f); cpu.regs.set_hl(hl); return 8},
        0x3A => {cpu.regs.a = memory.read(cpu.regs.get_hld()); return 4},
        0x3B => {cpu.regs.sp = cpu.regs.sp.wrapping_sub(1); return 8},
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
                _ => panic!("literally impossible"),
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
                _ => panic!("mathematically impossible"),
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
                _ => panic!("mathematically impossible again")
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
                _ => panic!("why is this always mandatory??"),
            };
            func(&mut cpu.regs.a, data, &mut cpu.regs.f);
            return if src == 6 { 8 } else { 4 };
        },
        0xC0 => {let cycles = ret(cpu, !cpu.regs.f.z(), memory.read_word(cpu.regs.sp)); return cycles},
        0xC1 => {cpu.regs.set_bc(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12;},
        0xC2 => {let new = memory.read_word(cpu.regs.pc_word()); let cycles = jp(cpu, !cpu.regs.f.z(), new); return cycles},
        0xC3 => {let new = memory.read_word(cpu.regs.pc_word()); let cycles = jp(cpu, true, new); return cycles},
        0xC4 => {let cycles = call(cpu, !cpu.regs.f.z(), memory); return cycles},
        0xC5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_bc()); cpu.regs.sp -= 2; return 16;},
        0xC6 => {let data = memory.read(cpu.regs.pc()); add(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8;}
        0xC7 => {rst(cpu, memory, 0x00); return 16},
        0xC8 => {let cycles = ret(cpu, cpu.regs.f.z(), memory.read_word(cpu.regs.sp)); return cycles},
        0xC9 => {let cycles = ret(cpu, true, memory.read_word(cpu.regs.sp)); return cycles}
        0xCA => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, cpu.regs.f.z(), memory.read_word(pc)); return cycles}
        0xCC => {let cycles = call(cpu, cpu.regs.f.z(), memory); return cycles}
        0xCD => {let cycles = call(cpu, true, memory); return cycles},
        0xCE => {let data = cpu.regs.pc(); adc(&mut cpu.regs.a, memory.read(data), &mut cpu.regs.f); return 8},
        0xCF => {rst(cpu, memory, 0x08); return 16},
        0xD0 => {let cycles = ret(cpu, !cpu.regs.f.c(), memory.read_word(cpu.regs.sp)); return cycles},
        0xD1 => {cpu.regs.set_de(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12},
        0xD2 => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, !cpu.regs.f.c(), memory.read_word(pc)); return cycles}
        0xD4 => {let cycles = call(cpu, !cpu.regs.f.c(), memory); return cycles},
        0xD5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_de()); cpu.regs.sp -= 2; return 16}
        0xD6 => {let data = memory.read(cpu.regs.pc()); sub(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8}
        0xD7 => {rst(cpu, memory, 0x10); return 16}
        0xD8 => {let cycles = ret(cpu, cpu.regs.f.c(), memory.read_word(cpu.regs.sp)); return cycles},
        0xD9 => {let cycles = ret(cpu, true, memory.read_word(cpu.regs.sp)); cpu.scheduled_ime=true; return cycles}
        0xDA => {let pc = cpu.regs.pc_word(); let cycles = jp(cpu, cpu.regs.f.c(), memory.read_word(pc)); return cycles},
        0xDC => {let cycles = call(cpu, cpu.regs.f.c(), memory); return cycles},
        0xDE => {let data = memory.read(cpu.regs.pc()); sbc(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8},
        0xDF => {rst(cpu, memory, 0x18); return 16},
        0xE0 => {let address = memory.read(cpu.regs.pc()) as u16 + 0xFF00; memory.write(address, cpu.regs.a); return 12;}
        0xE1 => {cpu.regs.set_hl(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12},
        0xE2 => {let address = cpu.regs.c as u16 + 0xFF00; memory.write(address, cpu.regs.a); return 8},
        0xE5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_hl()); cpu.regs.sp -= 2; return 16},
        0xE6 => {let data = memory.read(cpu.regs.pc()); and(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8},
        0xE7 => {rst(cpu, memory, 0x20); return 16},
        0xE8 => {let data = memory.read(cpu.regs.pc()) as i8; add_u16_i8(&mut cpu.regs.sp, data, &mut cpu.regs.f); return 16},
        0xE9 => {jp(cpu, true, cpu.regs.get_hl()); return 4},
        0xEA => {let address = memory.read_word(cpu.regs.pc_word()); memory.write(address, cpu.regs.a); return 16;}
        0xEE => {let data = memory.read(cpu.regs.pc()); xor(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8;}
        0xEF => {rst(cpu, memory, 0x28); return 16},
        0xF0 => {let data_address = memory.read(cpu.regs.pc()) as u16 + 0xFF00; cpu.regs.a = memory.read(data_address); return 12},
        0xF1 => {cpu.regs.set_af(memory.read_word(cpu.regs.sp)); cpu.regs.sp += 2; return 12;},
        0xF2 => {let data_address = cpu.regs.c as u16 + 0xFF00; cpu.regs.a = memory.read(data_address); return 8},
        0xF3 => {cpu.scheduled_ime=false; return 4},
        0xF5 => {memory.write_word(cpu.regs.sp-2, cpu.regs.get_af()); cpu.regs.sp -= 2; return 16},
        0xF6 => {let data = memory.read(cpu.regs.pc()); or(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8},
        0xF7 => {rst(cpu, memory, 0x30); return 16},
        0xF8 => {let data = memory.read(cpu.regs.pc()) as i8; set_add_u16_i8(cpu, data); return 12},
        0xF9 => {cpu.regs.sp = cpu.regs.get_hl(); return 8;},
        0xFA => {let address = memory.read_word(cpu.regs.pc_word()); cpu.regs.a = memory.read(address); return 16}
        0xFB => {cpu.scheduled_ime = true; return 4},
        0xFE => {let data = memory.read(cpu.regs.pc()); cp(&mut cpu.regs.a, data, &mut cpu.regs.f); return 8;},
        0xFF => {rst(cpu, memory, 0x38); return 16}
        _ => panic!("unsupported opcode"),
    }
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
            _ => panic!("invalid operation **literally impossible** ")
        }
    }

    let opcode = memory.read(cpu.regs.pc());

    let target = opcode % 8;
    let operation = opcode / 8;

    // meaning we are changing the [hl]
    if target == 6 {
        let mut value = memory.read(cpu.regs.get_hl());
        run_operation(&mut value, operation, &mut cpu.regs.f);
        memory.write(cpu.regs.get_hl(), value);
        if operation / 8 == 1 {
            return 12
        } else {
            return 16
        }
    }
    let src = match target {
        0 => &mut cpu.regs.b,
        1 => &mut cpu.regs.c,
        2 => &mut cpu.regs.d,
        3 => &mut cpu.regs.e,
        4 => &mut cpu.regs.h,
        5 => &mut cpu.regs.l,
        7 => &mut cpu.regs.a,
        _ => panic!("literally impossible again, we are using u8s, they do not get higher than this")
    };
    run_operation(src, operation, &mut cpu.regs.f);
    return 8;
}

fn dec(reg: &mut u8, f: &mut Flag) {
    f.set_h(half_carry_sub(*reg, 1));
    *reg = reg.wrapping_sub(1);
    f.set_z(*reg==0);
    f.set_n(true);   
}
fn inc(reg: &mut u8, f: &mut Flag) { 
    f.set_h(half_carry_add(*reg, 1));
    *reg = reg.wrapping_add(1);
    f.set_z(*reg==0);
    f.set_n(false);
}
fn add_u16(reg: u16, add: u16, f: &mut Flag) -> u16 {
    f.set_h(half_carry_add_u16(reg, add)); 
    let sum = reg.wrapping_add(add);
    f.set_c(sum<reg);
    f.set_n(false);
    sum
}
fn set_add_u16_i8(cpu: &mut Cpu, data: i8) {
    let temp = cpu.regs.sp;
    add_u16_i8(&mut cpu.regs.sp, data, &mut cpu.regs.f);
    cpu.regs.set_hl(cpu.regs.sp);
    cpu.regs.sp = temp;
}

fn add_u16_i8(reg: &mut u16, data: i8, f: &mut Flag) {
    let compare = *reg;

    // the half carry uses the data as an u8 and pretends like it is being added
    // took me a while to figure out
    f.set_h(half_carry_add((*reg&0xff) as u8, data as u8));
    *reg = reg.wrapping_add_signed(data as i16);
    f.set_c((*reg&0xff)<(compare&0xff));
    
    f.set_z(false);
    f.set_n(false);

}
/// this instruction is not nice. should fix it. works though
fn daa(a: &mut u8, f: &mut Flag) { 
    if !f.n() {
        let mut change = 0;
        // need to change the lower nybble
        if f.h() || *a&0xF > 0x09 {
            change |= 0x06;
        }
        // need to change the upper nybble
        // we add 0x60 since the carry will reset it
        if f.c() || *a > 0x99 {
            change |= 0x60;
            f.set_c(true);
        }
        *a = a.wrapping_add(change);
    } else {
        let mut change: u8 = 0;
        if f.c() {
            change += 0x60;
        }
        if f.h() {
            change += 6;
        }
        *a = a.wrapping_sub(change);
    }
    f.set_z(*a==0);
    f.set_h(false);
}

fn scf(f: &mut Flag) { 
    f.set_c(true);
    f.set_h(false);
    f.set_n(false);
}
fn ccf(f: &mut Flag) { 
    f.set_c(!f.c());
    f.set_h(false);
    f.set_n(false);    
}

fn cpl(a: &mut u8, f: &mut Flag) { 
    *a = !*a;
    f.set_n(true);
    f.set_h(true);
}
fn add(a: &mut u8, data: u8, f: &mut Flag) {
    let (sum, overflow) = a.overflowing_add(data);
    f.set_z(sum==0);
    f.set_n(false);
    f.set_c(overflow);
    f.set_h(half_carry_add(*a, data));
    *a = sum;
}
fn adc(a: &mut u8, data: u8, f: &mut Flag) {
    let c = f.c() as u8;
    add(a, data, f);
    let first_flag = f.into_u8();
    add(a, c, f);
    *f = Flag::from_u8(first_flag|f.into_u8());
    f.set_z(*a==0);
}
fn sub(a: &mut u8, data: u8, f: &mut Flag) { 
    let (sum, overflow) = a.overflowing_sub(data);
    f.set_z(sum==0);
    f.set_c(overflow);
    f.set_h(half_carry_sub(*a, data));
    f.set_n(true);
    *a = sum;
}
fn sbc(a: &mut u8, data: u8, f: &mut Flag) { 
    let c = f.c() as u8;
    sub(a, data, f);
    let first_flag = f.into_u8();
    sub(a, c, f);
    *f = Flag::from_u8(first_flag|f.into_u8());
    f.set_z(*a==0);
}
fn and(a: &mut u8, data: u8, f: &mut Flag) { 
    *a = *a & data;
    f.set_z(*a==0);
    f.set_n(false);
    f.set_h(true);
    f.set_c(false);
}
fn xor(a: &mut u8, data: u8, f: &mut Flag) { 
    *a = *a^data;
    f.set_z(*a==0);
    f.set_c(false);
    f.set_h(false);
    f.set_n(false);
}
fn or(a: &mut u8, data: u8, f: &mut Flag) { 
    *a = *a|data;
    f.set_c(false);
    f.set_h(false);
    f.set_n(false);
    f.set_z(*a==0);
}
fn cp(a: &mut u8, data: u8, f: &mut Flag) { 
    let temp = *a;
    sub(a, data, f);
    *a = temp;
}

fn jr(cpu: &mut Cpu, cc: bool, change: u8) -> u8 {
    if !cc {
        return 8;
    }
    cpu.regs.relative_pc(change as i8);
    return 12;
}
fn jp(cpu: &mut Cpu, cc: bool, new: u16) -> u8 { 
    if !cc {
        return 12;
    }
    cpu.regs.set_pc(new);
    return 16;
}

fn ret(cpu: &mut Cpu, cc: bool, new: u16) -> u8 {
    if !cc {
        return 8;
    }
    cpu.regs.sp += 2;
    cpu.regs.set_pc(new);
    return 20;
}
/// this function adds a memory address to memory and so we need access
/// to the memory, seems a bit overkill to ask for all memory but I
/// cannot think of another way to do this. Its just a pointer anyways
fn call(cpu: &mut Cpu, cc: bool, memory: &mut Memory) -> u8 {
    let new_address = memory.read_word(cpu.regs.pc_word());
    if !cc {
        return 12;
    }
    // we jump when we call
    let fallback_address = cpu.regs.pc();
    jp(cpu, true, new_address);

    cpu.regs.sp -= 2;
    memory.write_word(cpu.regs.sp, fallback_address);
    return 24;
}
fn rst(cpu: &mut Cpu, memory: &mut Memory, new: u16) {
    let fallback_address = cpu.regs.pc();
    memory.write_word(cpu.regs.sp-2, fallback_address);
    cpu.regs.sp -= 2;

    jp(cpu, true, new);
}

/// this is all the prefixed opcodes. Some of the instructions here are used
/// by the upper half but that is an exception and not the rule. it is even possible that the two 
/// halves have different ways of doing each of their respective methods.
/// these are all bit manipulation instructions
/// all the information was gathered from: https://rgbds.gbdev.io/v0.4.2/gbz80.7
fn bit(data: &mut u8, f: &mut Flag, operation: u8) {
    let is_set = (*data & (0b0000_0001 << operation)) != 0;
    f.set_z(!is_set);
    f.set_n(false);
    f.set_h(true);
}
fn srl(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b0000_0001 == 1);
    f.set_h(false);
    f.set_n(false);
    *data = *data >> 1;
    f.set_z(*data == 0);
}
fn swap(data: &mut u8, f: &mut Flag) {
    *data = (*data << 4) | (*data >> 4);
    f.set_z(*data == 0);
    f.set_n(false);
    f.set_h(false);
    f.set_c(false);
}
/// CHECK THIS LATER TODO
fn sra(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b0000_0001 == 1);
    *data = *data & 0b1000_0000 | *data >> 1;
    f.set_n(false);
    f.set_h(false);
    f.set_z(*data == 0);
}
/// CHECK THIS LATER TODO
fn sla(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b1000_0000 != 0);
    *data = * data << 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
fn rr(data: &mut u8, f: &mut Flag) {
    let c = f.c() as u8;
    f.set_c(*data & 0b0000_0001 == 1);
    *data = c << 7 | *data >> 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
fn rl(data: &mut u8, f: &mut Flag) {
    let c = f.c() as u8;
    f.set_c(*data & 0b1000_0000 != 0);
    *data = *data << 1 | c;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
fn rrc(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b0000_0001 == 1);
    *data = *data << 7 | *data >> 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
fn rlc(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b1000_0000 != 0);
    *data = *data >> 7 | *data << 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}