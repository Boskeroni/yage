pub trait MBC {
    fn read_rom(&self, address: usize) -> u8;
    fn write_rom(&mut self, address: usize, data: u8);

    fn read_ram(&self, address: usize) -> u8;
    fn write_ram(&mut self, address: usize, data: u8);
}

pub struct MBC1 {
    rom_banks: Vec<u8>,
    high_bank_index: usize,
    zero_bank_index: usize,

    ram: Vec<u8>,
    ram_index: usize,

    ram_enabled: bool,
    mode: bool,

    total_rom_banks: usize,
}
impl MBC for MBC1 {
    fn read_rom(&self, address: usize) -> u8 { 
        if address < 0x4000 {
            return self.rom_banks[address + self.zero_bank_index * 0x4000];
        }
        let offset_address = (0x4000 * self.high_bank_index) + (address % 0x4000);
        self.rom_banks[offset_address]
    }
    fn write_rom(&mut self, address: usize, data: u8) {
        match address {
            0..=0x1FFF => self.ram_enabled = data & 0xA == 0xA,
            0x2000..=0x3FFF => {
                if data == 0 || self.total_rom_banks == 2 {
                    self.high_bank_index = 1;
                    return;
                }
                let bitmask = match self.total_rom_banks {
                    128 | 64 | 32 => 0b0001_1111,
                    16 => 0b0000_1111,
                    8 => 0b0000_0111,
                    4 => 0b0000_0011,
                    _ => unreachable!(),
                };
                let rom_bank = data & bitmask;
                self.high_bank_index = rom_bank as usize;
                if self.total_rom_banks == 64 {
                    let lower_ram_bit = self.ram_index & 1;
                    self.high_bank_index |= lower_ram_bit << 5;
                }
                if self.total_rom_banks == 128 {
                    self.high_bank_index |= self.ram_index << 5;
                }
            }
            0x4000..=0x5FFF => {
                let ram_bank = data & 0b0000_0011;
                self.ram_index = ram_bank as usize;
            }
            0x6000..=0x7FFF => {
                let mode = data & 1 == 1;
                self.mode = mode;


                if self.total_rom_banks <= 32 {
                    return;
                }
                if self.total_rom_banks == 64 {
                    let lower_ram_bit = self.ram_index & 1;
                    self.zero_bank_index &= 0;
                    self.zero_bank_index |= lower_ram_bit<<5;
                    return;
                }
                if self.total_rom_banks == 128 {
                    self.zero_bank_index &= 0;
                    self.zero_bank_index |= self.ram_index << 6;
                }
            }
            _ => {}
        }
    }
    fn read_ram(&self, address: usize) -> u8 {
        if !self.ram_enabled {
            eprintln!("read to ram was ignored");
            return 0xFF;
        }
        let offset_address = address - 0xA000;
        self.ram[self.ram_index * 0x2000 + offset_address]
    }
    fn write_ram(&mut self, address: usize, data: u8) {
        if !self.ram_enabled {
            eprintln!("write to ram was ignored");
            return;
        }
        let offset_address = address - 0xA000;
        self.ram[self.ram_index * 0x2000 + offset_address] = data;
    }
}
pub struct MBC2 {
    rom_banks: Vec<u8>,
    high_rom_index: usize,
    
    ram_enabled: bool,
    ram: Vec<u8>
}
impl MBC for MBC2 {
    fn read_rom(&self, address: usize) -> u8 {
        if address <= 0x3FFF {
            return self.rom_banks[address];
        }
        let rom_address = 0x4000 * self.high_rom_index + (address - 0x4000);
        return self.rom_banks[rom_address];
    }
    fn write_rom(&mut self, address: usize, data: u8) {
        if address & 0x100 == 0 {
            self.ram_enabled = data & 0xA == 0xA;
            return;
        }
        self.high_rom_index = data.min(1) as usize;
    }
    fn read_ram(&self, address: usize) -> u8 {
        // it might be pointless to reconfirm it is 4 bits as all the
        // writes already do it but there is no harm in doing it
        return self.ram[address&0x1FF] & 0x0F
    }
    fn write_ram(&mut self, address: usize, data: u8) {
        if !self.ram_enabled {
            eprintln!("the write is ignored");
        }
        let actual_address = address & 0x1FF;
        self.ram[actual_address] = data & 0x0F;
    }
}
struct MBC3 {
    rom: Vec<u8>,
    high_rom_index: usize,

    ram: Vec<u8>,
    ram_index: usize,
    ram_enabled: bool,
}
impl MBC for MBC3 {
    fn read_rom(&self, address: usize) -> u8 {
        if address <= 0x3FFF {
            return self.rom[address];
        }
        let actual_address = 0x4000 * self.high_rom_index + (address - 0x4000);
        return self.rom[actual_address];
    }
    fn read_ram(&self, address: usize) -> u8 {
        let actual_address = (0x2000 * self.ram_index) + (address - 0xA000);
        return self.ram[actual_address];
    }
    fn write_rom(&mut self, address: usize, data: u8) {
        match address {
            0..=0x1FFF => {
                self.ram_enabled = data & 0xA == 0xA;
            }
            0x2000..=0x3FFF => {
                self.high_rom_index = (data & 0x7F).min(1) as usize;
            }
            0x4000..=0x5FFF => {
                if data > 0x04 {
                    panic!("havent implemened RTC yet");
                }
                self.ram_index = data as usize;
            }
            0x6000..=0x7FFF => {
                eprint!("also havent implemented RTC yet");
            }
            _ => unreachable!()
        }
    }
    fn write_ram(&mut self, address: usize, data: u8) {
        if !self.ram_enabled {
            eprintln!("write to ram is ignored");
            return;
        }
        let actual_address = 0x2000 * self.ram_index + (address - 0xA000);
        self.ram[actual_address] = data;
    }
}

pub fn create_mbc(rom: &Vec<u8>) -> Box<dyn MBC> {
    let mbc_type_code = rom[0x147];
    let rom_size_code = rom[0x148];

    let rom_size;
    if rom_size_code <= 0x08 {
        rom_size = 2_u32.pow(rom_size_code as u32 + 1);
    } else {
        rom_size = match rom_size_code {
            0x52 => 72,
            0x53 => 80,
            0x54 => 96,
            _ => panic!("unsupported rom size provided"),
        };
    }
    let rom_length = 0x3FFF + 0x4000 * (rom_size - 1) as usize;
    let rom_bank = rom[0x0000..=rom_length].to_vec();

    //let ram_size_code = rom[0x149];
    /* let ram_size = match ram_size_code {
        0x00 => 1,
        _ => panic!("unsupported ram_size provided"),
    }; */

    match mbc_type_code {
        0x00 | 0x01 | 0x02 | 0x03 => {
            let ram_bank = vec![0; 0x2000];

            Box::new(MBC1 {
                rom_banks: rom_bank,
                total_rom_banks: rom_size as usize,
                high_bank_index: 1,
                zero_bank_index: 0,

                ram: ram_bank,
                ram_index: 0,

                ram_enabled: true,
                mode: false,
            })
        }
        0x05 | 0x06 => {
            let ram = vec![0; 256];
            Box::new(MBC2 {
                rom_banks: rom_bank,
                high_rom_index: 1,
                ram_enabled: true,
                ram,
            })
        }
        0x0F..=0x13 => {
           let ram = rom[rom_length+1..].to_vec();
           Box::new(MBC3 {
            rom: rom_bank,
            high_rom_index: 1,
            ram,
            ram_index: 0,
            ram_enabled: true,
           })
        }
        _ => panic!("unsupported MBC type"),
    }
}