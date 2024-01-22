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

    /// reads from memory. Currently doesnt handle how the PPU modifies
    /// memory and invalid read requests but I will eventually get around to fixing
    /// that
    pub fn read(&self, address: u16) -> u8 {
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