use crate::little_endian_combine;

enum TimerRegisters {
    DIV=0xFF04,
    TIMA=0xFF05,
    TMA=0xFF06,
    TAC=0xFF07,
}

pub struct Memory {
    mem: Vec<u8>,
    div: u16,
}
impl Memory {
    pub fn new(rom: Vec<u8>) -> Self {
        let mut memory = rom;
        if memory.len() > 0x8000 {
            panic!("not going to handle these yet")
        }
        let padding_amount = 65536 - memory.len();
        let padding_vec = vec![0; padding_amount];
        memory.extend(padding_vec);

        memory[0xFF00] = 0xCF;
        memory[0xFF02] = 0x7E;
        memory[0xFF04] = 0x18;
        memory[0xFF07] = 0xF8;
        memory[0xFF0F] = 0xE1;
        memory[0xFF40] = 0x91;
        memory[0xFF41] = 0x81;
        memory[0xFF44] = 0x91;
        memory[0xFF46] = 0xFF;

        Self { mem: memory, div: 0 }
    }

    /// this completes a write to the memory while also updating
    /// the timer accordingly to how it should work.
    pub fn write_timed(&mut self, address: u16, data: u8) {
        self.write(address, data);
        update_timer(self, 4);
    }

    /// this completes a write to memory and follows the rules of writing
    /// to memory. Currently doesnt handle memory bank controllers but I will
    /// eventually implement them
    pub fn write(&mut self, address: u16, data: u8) {
        let address = address as usize;
        if address < 0x8000 {
            panic!("cannot handle swapping yet, {address}");
        }

        // only the second bit of the stat register matter
        let blocker = self.mem[0xFF41] & 0b0000_0011;
        if blocker == 2 && is_within_oam(address) {
            return;
        } else if blocker == 3 && (is_within_oam(address) || is_within_vram(address)) {
            return;
        }

        // the internal DIV 
        if address == 0xFF05 {
            self.div = 0;
            self.mem[TimerRegisters::DIV as usize] = 0x00;
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
        self.write_timed(address, (data & 0xFF) as u8);
        self.write_timed(address+1, (data >> 8) as u8);
    }

    /// reads from memory
    pub fn read(&self, address: u16) -> u8 {
        let address = address as usize;

        // only the second bit of the stat register matter
        let blocker = self.mem[0xFF41] & 0b0000_0011;
        if blocker == 2 && is_within_oam(address) {
            return 0xFF;
        } else if blocker == 3 && (is_within_oam(address) || is_within_vram(address)) {
            return 0xFF;
        }


        self.mem[address as usize]
    }

    /// What the CPU uses to read memory as it takes 4 cycles in order to get
    /// memory and the timer needs to be updated accordingly.
    pub fn read_timed(&mut self, address: u16) -> u8 {
        update_timer(self, 4);
        self.read(address)
    }

    /// just makes reading 16-bits of data more convenient
    pub fn read_word(&mut self, address: u16) -> u16 {
        little_endian_combine(self.read_timed(address), self.read_timed(address+1))
    }

    pub fn oam_search(&self, index: u8) -> [u8; 4] {
        // the start of oam plus the index spacing
        let start = 0xFE00 + index as usize * 4;
        (self.mem[start], self.mem[start+1], self.mem[start+2], self.mem[start+3]).try_into().unwrap()
    }

    pub fn read_bg_tile(&self, address: u16, addressing: u16) -> [u16; 8] {
        // the address is the tile in the background
        // not the data for the tile
        let tile_index = self.mem[address as usize];
        let tile_address;

        if addressing == 0x8000 {
            tile_address = addressing + (tile_index as u16) * 16;
        } else if addressing == 0x8800 {
            tile_address = addressing.checked_add_signed(tile_index as i8 as i16).unwrap();
        } else {
            panic!("invalid addressing for tiles");
        }

        let mut tile_data: Vec<u16> = Vec::new();
        for i in 0..8 {
            let low = i*2;
            let high = (i*2) + 1;

            let low_data = self.mem[tile_address as usize+low];
            let high_data = self.mem[tile_address as usize+high];
            
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

pub fn update_timer(memory: &mut Memory, cycles: u8) {
    use TimerRegisters::*;

    if cycles == 0 {
        return;
    }

    let tac = memory.read(TAC as u16) & 0b0000_0011;
    let bit_position = match tac {
        0 => 9,
        1 => 3,
        2 => 5,
        3 => 7,
        _ => panic!("genuinely impossible"),
    };

    for _ in 0..cycles {
        let starting_edge = memory.div & (1 << bit_position) != 0;
    
        memory.div = memory.div.wrapping_add(1);
        memory.write(DIV as u16, (memory.div >> 8) as u8);
        
        let ending_edge = memory.div & (1 << bit_position) != 0;
    
        // tima doesnt need to be incremented
        if ending_edge || !starting_edge {
            continue;
        }
    
        let tima = memory.read(TIMA as u16);
        let (sum, overflow) = tima.overflowing_add(1);
    
        // the tima doesnt need resetting / interrupting
        if !overflow {
            memory.write(TIMA as u16, sum);
            continue;
        }
    
        // reset the TIMA
        let tma = memory.read(TMA as u16);
        memory.write(TIMA as u16, tma);
    
        // call the interrupt
        let interrupt_flag = memory.read(0xFF0F);
        memory.write(0xFF0F, interrupt_flag | 0b0000_0100);
    }
}

fn is_within_oam(index: usize) -> bool {
    return index >= 0xFE00 && index <= 0xFE9F
}
fn is_within_vram(index: usize) -> bool {
    return index >= 0x8000 && index <= 0x9FFF
}