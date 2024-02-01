use rand::{distributions::Standard, Rng};
use crate::little_endian_combine;

const NINTENDO_LOGO: [u8; 48] = 
    [0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
     0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
     0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E
];


fn random_padding(amount: usize) -> Vec<u8> {
    rand::thread_rng().sample_iter(Standard).take(amount).collect()
}

pub struct Memory {
    pub mem: Vec<u8>,
    pub div: u8,
}
impl Memory {
    pub fn new_unbooted(rom: Vec<u8>) -> Self {
        let mut memory = rom;
        if memory.len() > 0x8000 {
            panic!("not going to handle these yet")
        }
        let padding_amount = 0xFE00 - memory.len();
        let padding_vec = random_padding(padding_amount);
        memory.extend(padding_vec);
        memory.extend(vec![0; 0x200]);

        Self { mem: memory, div: 0 }
    }

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

    /// this completes a write to memory and follows the rules of writing
    /// to memory. Currently doesnt handle memory bank controllers but I will
    /// eventually implement them
    pub fn write(&mut self, address: u16, data: u8) {
        let address = address as usize;
        if address < 0x8000 {
            return;
        }

        if address == 0xFF46 {
            process_dma(self, data);
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
    pub fn unchecked_write(&mut self, address: u16, data: u8) {
        self.mem[address as usize] = data;
    }

    /// this follows the little endian encoding which th gameboy follows. 
    /// the lower byte gets sent to the lower memory address index.
    /// This also follows the timings for how write 16 bits of data should
    pub fn write_word(&mut self, address: u16, data: u16) {
        self.write(address, (data & 0xFF) as u8);
        self.write(address+1, (data >> 8) as u8);
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

    /// just makes reading 16-bits of data more convenient
    pub fn read_word(&mut self, address: u16) -> u16 {
        little_endian_combine(self.read(address), self.read(address+1))
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
    for i in 0..0x100 {
        mem.mem[0xFE00+i] = mem.mem[real_address+i];
    }
}

fn is_within_oam(index: usize) -> bool {
    return index >= 0xFE00 && index <= 0xFE9F
}
fn is_within_vram(index: usize) -> bool {
    return index >= 0x8000 && index <= 0x9FFF
}