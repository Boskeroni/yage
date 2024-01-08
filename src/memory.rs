use crate::util::little_endian_combine;

pub struct Memory {
    mem: Vec<u8>,
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

        Self { mem: memory }
    }

    pub fn write(&mut self, address: u16, data: u8) {
        let address = address as usize;
        if address < 0x8000 {
            //panic!("cannot handle swapping yet, {address}");
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
    pub fn write_word(&mut self, address: u16, data: u16) {
        self.write(address, (data & 0xFF) as u8);
        self.write(address+1, (data >> 8) as u8);
    }


    pub fn read(&self, address: u16) -> u8 {
        if address == 0xFF44 {
            return 0x90;
        }
        self.mem[address as usize]
    }
    pub fn read_word(&self, address: u16) -> u16 {
        let address = address as usize;
        little_endian_combine(self.mem[address], self.mem[address+1])
    }
}