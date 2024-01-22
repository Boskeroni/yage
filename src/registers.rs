use crate::{split, combine};

#[derive(Debug)]
pub struct Registers {
    pub a: u8,
    pub f: Flag,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pc: u16,
    pub sp: u16,
}
impl Default for Registers {
    fn default() -> Self {
        Self {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            f: Flag::from_u8(0xB0),
            h: 0x01,
            l: 0x4D,
            pc: 0x0100,
            sp: 0xFFFE,
        }
    }
}
impl Registers {
    pub fn set_bc(&mut self, bc: u16) { (self.b, self.c) = split(bc) }
    pub fn get_bc(&self) -> u16 { combine(self.b, self.c) }

    pub fn set_de(&mut self, de: u16) { (self.d, self.e) = split(de) }
    pub fn get_de(&self) -> u16 { combine(self.d, self.e) }

    pub fn set_hl(&mut self, hl: u16) { (self.h, self.l) = split(hl) }
    pub fn get_hl(&self) -> u16 { combine(self.h, self.l) }
    pub fn get_hli(&mut self) -> u16 { 
        let hl = self.get_hl();
        self.set_hl(hl.wrapping_add(1));
        hl
    }
    pub fn get_hld(&mut self) -> u16 {
        let hl = self.get_hl();
        self.set_hl(hl.wrapping_sub(1));
        hl
    }

    pub fn set_af(&mut self, af: u16) { 
        let (a, f) = split(af);
        self.a = a;
        self.f = Flag::from_u8(f);
    }
    pub fn get_af(&self) -> u16 {
        combine(self.a, self.f.into_u8())
    }

    /// this returns the CPUs pc counter while also incrementing it 
    /// for the next time round. makes it overall more convenient as I 
    /// would like to keep it private without increment just like the gameboy
    pub fn pc(&mut self) -> u16 {
        self.pc += 1;
        self.pc-1
    }
    pub fn set_pc(&mut self, address: u16) {
        self.pc = address;
    }
    pub fn relative_pc(&mut self, address: i8) {
        self.pc = self.pc.wrapping_add_signed(address as i16);
    }

    /// some functions require the CPU to get the next two pieces of data from 
    /// memory. I could call the `cpu.regs.pc()` function twice but this is more convenient
    /// as my word fetching functions all just use one parameter (the original)
    pub fn pc_word(&mut self) -> u16 {
        self.pc += 2;
        self.pc-2
    }
}


#[derive(Debug, Default)]
pub struct Flag {
    zero_flag: bool,
    sub_flag: bool,
    half_carry_flag: bool,
    carry_flag: bool,
}
impl Flag {
    pub fn z(&self) -> bool { self.zero_flag }
    pub fn set_z(&mut self, z: bool) { self.zero_flag = z}

    pub fn n(&self) -> bool { self.sub_flag }
    pub fn set_n(&mut self, n: bool) { self.sub_flag = n}

    pub fn h(&self) -> bool { self.half_carry_flag }
    pub fn set_h(&mut self, h: bool) { self.half_carry_flag = h}

    pub fn c(&self) -> bool { self.carry_flag }
    pub fn set_c(&mut self, c: bool) { self.carry_flag = c}

    pub fn from_u8(value: u8) -> Self {
        Self {
            zero_flag: (value & 0b1000_0000) != 0,
            sub_flag: (value & 0b0100_0000) != 0,
            half_carry_flag: (value & 0b0010_0000) != 0,
            carry_flag: (value & 0b0001_0000) != 0
        }
    }
    pub fn into_u8(&self) -> u8 {
        (self.zero_flag as u8) << 7 |
        (self.sub_flag as u8) << 6 |
        (self.half_carry_flag as u8) << 5 |
        (self.carry_flag as u8) << 4
    }
}