// this just houes all of the opcodes (along with helper functions)
// functions just to make the processor.rs file less cluttered

use crate::Cpu;
use crate::Memory;
use crate::registers::Flag;

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

pub fn dec(reg: &mut u8, f: &mut Flag) {
    f.set_h(half_carry_sub(*reg, 1));
    *reg = reg.wrapping_sub(1);
    f.set_z(*reg==0);
    f.set_n(true);   
}
pub fn inc(reg: &mut u8, f: &mut Flag) { 
    f.set_h(half_carry_add(*reg, 1));
    *reg = reg.wrapping_add(1);
    f.set_z(*reg==0);
    f.set_n(false);
}
pub fn add_u16(reg: u16, add: u16, f: &mut Flag) -> u16 {
    f.set_h(half_carry_add_u16(reg, add)); 
    let sum = reg.wrapping_add(add);
    f.set_c(sum<reg);
    f.set_n(false);
    sum
}

pub fn set_add_u16_i8(cpu: &mut Cpu, data: i8) {
    let temp = cpu.regs.sp;
    add_u16_i8(&mut cpu.regs.sp, data, &mut cpu.regs.f);
    cpu.regs.set_hl(cpu.regs.sp);
    cpu.regs.sp = temp;
}

pub fn add_u16_i8(reg: &mut u16, data: i8, f: &mut Flag) {
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
pub fn daa(a: &mut u8, f: &mut Flag) { 
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

pub fn scf(f: &mut Flag) { 
    f.set_c(true);
    f.set_h(false);
    f.set_n(false);
}
pub fn ccf(f: &mut Flag) { 
    f.set_c(!f.c());
    f.set_h(false);
    f.set_n(false);    
}

pub fn cpl(a: &mut u8, f: &mut Flag) { 
    *a = !*a;
    f.set_n(true);
    f.set_h(true);
}
pub fn add(a: &mut u8, data: u8, f: &mut Flag) {
    let (sum, overflow) = a.overflowing_add(data);
    f.set_z(sum==0);
    f.set_n(false);
    f.set_c(overflow);
    f.set_h(half_carry_add(*a, data));
    *a = sum;
}
pub fn adc(a: &mut u8, data: u8, f: &mut Flag) {
    let c = f.c() as u8;
    add(a, data, f);
    let first_flag = f.into_u8();
    add(a, c, f);
    *f = Flag::from_u8(first_flag|f.into_u8());
    f.set_z(*a==0);
}
pub fn sub(a: &mut u8, data: u8, f: &mut Flag) { 
    let (sum, overflow) = a.overflowing_sub(data);
    f.set_z(sum==0);
    f.set_c(overflow);
    f.set_h(half_carry_sub(*a, data));
    f.set_n(true);
    *a = sum;
}
pub fn sbc(a: &mut u8, data: u8, f: &mut Flag) { 
    let c = f.c() as u8;
    sub(a, data, f);
    let first_flag = f.into_u8();
    sub(a, c, f);
    *f = Flag::from_u8(first_flag|f.into_u8());
    f.set_z(*a==0);
}
pub fn and(a: &mut u8, data: u8, f: &mut Flag) { 
    *a = *a & data;
    f.set_z(*a==0);
    f.set_n(false);
    f.set_h(true);
    f.set_c(false);
}
pub fn xor(a: &mut u8, data: u8, f: &mut Flag) { 
    *a = *a^data;
    f.set_z(*a==0);
    f.set_c(false);
    f.set_h(false);
    f.set_n(false);
}
pub fn or(a: &mut u8, data: u8, f: &mut Flag) { 
    *a = *a|data;
    f.set_c(false);
    f.set_h(false);
    f.set_n(false);
    f.set_z(*a==0);
}
pub fn cp(a: &mut u8, data: u8, f: &mut Flag) { 
    let temp = *a;
    sub(a, data, f);
    *a = temp;
}

pub fn jr(cpu: &mut Cpu, cc: bool, change: u8) -> u8 {
    if !cc {
        return 0;
    }
    cpu.regs.relative_pc(change as i8);
    return 4;
}
pub fn jp(cpu: &mut Cpu, cc: bool, new: u16) -> u8 { 
    if !cc {
        return 0;
    }
    cpu.regs.set_pc(new);
    return 4;
}

pub fn ret(cpu: &mut Cpu, cc: bool, memory: &mut Memory) -> u8 {
    if !cc {
        return 4;
    }
    let new = memory.read_word(cpu.regs.sp);
    cpu.regs.sp += 2;
    cpu.regs.set_pc(new);
    return 8;
}
/// this function adds a memory address to memory and so we need access
/// to the memory, seems a bit overkill to ask for all memory but I
/// cannot think of another way to do this. Its just a pointer anyways
pub fn call(cpu: &mut Cpu, cc: bool, memory: &mut Memory) -> u8 {
    let new_address = memory.read_word(cpu.regs.pc_word());
    if !cc {
        ;
    }
    // we jump when we call
    let fallback_address = cpu.regs.pc();
    jp(cpu, true, new_address);

    cpu.regs.sp -= 2;
    memory.write_word(cpu.regs.sp, fallback_address);
    return 24;
}
pub fn rst(cpu: &mut Cpu, memory: &mut Memory, new: u16) {
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
pub fn bit(data: &mut u8, f: &mut Flag, operation: u8) {
    let is_set = (*data & (0b0000_0001 << operation)) != 0;
    f.set_z(!is_set);
    f.set_n(false);
    f.set_h(true);
}
pub fn srl(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b0000_0001 == 1);
    f.set_h(false);
    f.set_n(false);
    *data = *data >> 1;
    f.set_z(*data == 0);
}
pub fn swap(data: &mut u8, f: &mut Flag) {
    *data = (*data << 4) | (*data >> 4);
    f.set_z(*data == 0);
    f.set_n(false);
    f.set_h(false);
    f.set_c(false);
}
pub fn sra(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b0000_0001 == 1);
    *data = *data & 0b1000_0000 | *data >> 1;
    f.set_n(false);
    f.set_h(false);
    f.set_z(*data == 0);
}
pub fn sla(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b1000_0000 != 0);
    *data = * data << 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
pub fn rr(data: &mut u8, f: &mut Flag) {
    let c = f.c() as u8;
    f.set_c(*data & 0b0000_0001 == 1);
    *data = c << 7 | *data >> 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
pub fn rl(data: &mut u8, f: &mut Flag) {
    let c = f.c() as u8;
    f.set_c(*data & 0b1000_0000 != 0);
    *data = *data << 1 | c;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
pub fn rrc(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b0000_0001 == 1);
    *data = *data << 7 | *data >> 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}
pub fn rlc(data: &mut u8, f: &mut Flag) {
    f.set_c(*data & 0b1000_0000 != 0);
    *data = *data >> 7 | *data << 1;
    f.set_h(false);
    f.set_n(false);
    f.set_z(*data == 0);
}