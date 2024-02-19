use core::panic;

use rand::{distributions::Standard, Rng};
use crate::joypad;
use crate::mbc::{create_mbc, MBC};
use crate::util::{little_endian_combine, JOYPAD_ADDRESS};
use crate::util::NINTENDO_LOGO;


// makes the ram random, more accurate to the gameboy
fn random_padding(amount: usize) -> Vec<u8> {
    rand::thread_rng().sample_iter(Standard).take(amount).collect()
}

pub struct Memory {
    mem: Vec<u8>,
    mbc: MBC,
    div: u8,
}
impl Memory {
    pub fn new(rom: Vec<u8>, booted: bool) -> Self {
        let mut memory = Memory::new_unbooted(rom);
        if !booted {
            // just so it passes the boot rom's check
            for (i, val) in NINTENDO_LOGO.iter().enumerate() {
                memory.mem[0x104 + i] = *val;
            }
            return memory
        }

        memory.mem[0xFF00] = 0xFF;
        memory.mem[0xFF02] = 0x7E;
        memory.mem[0xFF04] = 0x18;
        memory.mem[0xFF07] = 0xF8;
        memory.mem[0xFF0F] = 0xE1;
        memory.mem[0xFF40] = 0x91;
        memory.mem[0xFF41] = 0x81;
        memory.mem[0xFF44] = 0x00;
        memory.mem[0xFF46] = 0xFF;

        return memory;
    }

    fn new_unbooted(rom: Vec<u8>) -> Self {
        let mut memory = random_padding(0xFE00);
        memory.extend(vec![0; 0x200]);

        let mbc = create_mbc(&rom);

        Self { mem: memory, div: 0, mbc }
    }

    /// this completes a write to memory and follows the rules of writing
    /// to memory. Currently doesnt handle memory bank controllers but I will
    /// eventually implement them
    pub fn write(&mut self, address: u16, data: u8) {
        let address = address as usize;
        if is_within_rom(address) {
            self.mbc.write_rom_bank(address, data);
            return;
        }
        if is_within_ram(address) {
            self.mbc.write_ram_bank(address, data);
            return;
        }

        if address == 0xFF40 && data & 0b1000_0000 == 0 {
            self.mem[0xFF41] &= 0b1111_1100;
            self.mem[0xFF44] = 0;
            return;
        }
        // only the upper bits of joypad register are writable
        if address == JOYPAD_ADDRESS {
            self.mem[JOYPAD_ADDRESS] &=  0x0F;
            self.mem[JOYPAD_ADDRESS] |= data & 0xF0;
            return;
        }
        if address == 0xFF46 {
            process_dma(self, data);
            return;
        }
        // the internal DIV 
        if address == 0xFF04 {
            self.div = 0;
            self.mem[0xFF04] = 0x00;
            return;
        }

        self.mem[address] = data;
        if address >= 0xC000 && address <= 0xDE00 {
            self.mem[address+0x2000] = data;
        } else if address >= 0xE000 && address <= 0xFE00 {
            self.mem[address-0x2000] = data;
        }
    }

    /// this follows the little endian encoding which th gameboy follows. 
    /// the lower byte gets sent to the lower memory address index.
    /// This also follows the timings for how write 16 bits of data should
    pub fn write_word(&mut self, address: u16, data: u16) {
        self.write(address, (data & 0xFF) as u8);
        self.write(address+1, (data >> 8) as u8);
    }

    pub fn unchecked_read(&self, address: u16) -> u8 {
        return self.mem[address as usize];
    }

    /// reads from memory
    pub fn read(&self, address: u16) -> u8 {
        let address = address as usize;

        if is_within_rom(address) {
            return self.mbc.read_rom_bank(address);
        }
        if is_within_ram(address) {
            return self.mbc.read_ram_bank(address);
        }

        if address == JOYPAD_ADDRESS as usize {
            return joypad(self.mem[JOYPAD_ADDRESS]);
        }

        // only the second bit of the stat register matter
        let blocker = self.mem[0xFF41] & 0b0000_0011;
        match (blocker, is_within_oam(address), is_within_vram(address)) {
            (2, true, _) => return 0xFF,
            (3, true, true) => return 0xFF,
            _ => return self.mem[address]
        }
    }

    /// just makes reading 16-bits of data more convenient
    pub fn read_word(&mut self, address: u16) -> u16 {
        little_endian_combine(self.read(address), self.read(address+1))
    }

    pub fn oam_search(&self, index: u8) -> [u8; 4] {
        // the start of oam plus the index spacing
        let start = 0xFE00 + (index as usize * 4);
        (self.mem[start], self.mem[start+1], self.mem[start+2], self.mem[start+3]).try_into().unwrap()
    }

    pub fn read_bg_tile(&self, address: u16, addressing: u16) -> [u16; 8] {
        // the address is the tile in the background
        // not the data for the tile
        let tile_index = self.mem[address as usize];

        let tile_address = match addressing {
            0x8000 => addressing + (tile_index as u16) * 16,
            0x8800 => 0x9000_u16.wrapping_add_signed((tile_index as i8 as i16) * 16),
            _ => panic!("invalid addressing"),
        };
        self.read_tile(tile_address)
    }

    pub fn read_tile(&self, address: u16) -> [u16; 8] {
        let mut tile_data: Vec<u16> = Vec::new();
        for i in 0..8 {
            let low = i*2;
            let high = (i*2) + 1;

            let low_data = self.mem[address as usize+low];
            let high_data = self.mem[address as usize+high];
            
            let mut line_data: u16 = 0;
            for j in 0..8 {
                let new_pixel = (low_data>>j)&1 | ((high_data>>j)&1)<<1;
                line_data |= (new_pixel as u16) << (j*2);
            }
            tile_data.push(line_data);
        }
        tile_data.try_into().unwrap()
    }
}

fn process_dma(mem: &mut Memory, address: u8) {
    let real_address = (address as usize) << 8;
    for i in 0..=0x9F {
        mem.mem[0xFE00+i] = mem.mem[real_address+i];
    }
}

fn is_within_oam(index: usize) -> bool {
    return index >= 0xFE00 && index <= 0xFE9F
}
fn is_within_vram(index: usize) -> bool {
    return index >= 0x8000 && index <= 0x9FFF
}
fn is_within_rom(index: usize) -> bool {
    return index <= 0x7FFF;
}
fn is_within_ram(index: usize) -> bool {
    return index >= 0xA000 && index <= 0xBFFF
}

use crate::util::TimerRegisters;
use crate::util::INTERRUPT_F_ADDRESS;

pub fn update_timer(memory: &mut Memory, cycles: u8) {
    use TimerRegisters::*;
    let tac = memory.read(TAC as u16);

    let timer_enable = (tac & 0b0000_0100) != 0;
    let bit_position = match tac & 0b0000_0011 {
        0 => 9,
        1 => 3,
        2 => 5,
        3 => 7,
        _ => unreachable!(),
    };

    let mut whole_div = (memory.read(DIV as u16) as u16) << 8 | memory.div as u16;
    let mut prev_edge = true;

    for _ in 0..cycles {
        // div is incremented
        whole_div = whole_div.wrapping_add(1);
        memory.mem[DIV as usize] = (whole_div>>8) as u8;

        let anded_result = ((whole_div & 1<<bit_position)!=0)&&timer_enable;
        if prev_edge && !anded_result {
            // for the next cycle
            prev_edge = anded_result;
            
            let tima = memory.read(TIMA as u16);
            let (new_tima, overflow) = tima.overflowing_add(1);

            if overflow {
                // the value it resets to when overlfowing
                let tma = memory.read(TMA as u16);
                memory.write(TIMA as u16, tma);

                //call the interrupt
                let i_flag = memory.read(INTERRUPT_F_ADDRESS);
                memory.write(INTERRUPT_F_ADDRESS, i_flag|0b0000_0100);
                continue;
            }
            // just a normal increment
            memory.write(TIMA as u16, new_tima);
        }
    }
    // update the register as well
    memory.div = memory.div.wrapping_add(cycles);
}