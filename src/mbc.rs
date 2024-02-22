/// due to how MBCs work the `upper_rom_bank` and `upper_ram_bank`
/// start at 1 and not 0. however, it is more convenient to use
/// the more conventional approach of having the index starting at 0
pub struct MBC {
    rom_banks: Vec<u8>,
    upper_rom_bank: usize,
    ram_banks: Vec<u8>,
    upper_ram_bank: usize,

    ram_enabled: bool,
    mode: bool,

    total_rom_banks: usize,
}
impl MBC {
    pub fn read_rom_bank(&self, address: usize) -> u8 { 
        if address < 0x4000 {
            return self.rom_banks[address];
        }
        let offset_address = (0x4000 * self.upper_rom_bank) + (address % 0x4000);
        self.rom_banks[offset_address]
    }
    pub fn read_ram_bank(&self, address: usize) -> u8 {
        if !self.ram_enabled {
            eprintln!("read to ram was ignored");
            return 0xFF;
        }
        let offset_address = address - 0xA000;
        self.ram_banks[self.upper_ram_bank * 0x2000 + offset_address]
    }

    pub fn write_rom_bank(&mut self, address: usize, data: u8) {
        match address {
            0..=0x1FFF => self.ram_enabled = data & 0xA == 0xA,
            0x2000..=0x3FFF => {
                if data == 0 || self.total_rom_banks == 2 {
                    self.upper_rom_bank = 1;
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
                println!("new rom bank => {rom_bank}");
                self.upper_rom_bank = rom_bank as usize;
            }
            0x4000..=0x5FFF => {
                let ram_bank = data & 0b0000_0011;
                self.upper_ram_bank = ram_bank as usize;
            }
            0x6000..=0x7FFF => {
                let mode = data & 1 == 1;
                println!("mode gets changed");
                self.mode = mode;
            }
            _ => {}
        }
    }
    pub fn write_ram_bank(&mut self, address: usize, data: u8) {
        if !self.ram_enabled {
            eprintln!("write to ram was ignored");
            return;
        }
        let offset_address = address - 0xA000;
        self.ram_banks[self.upper_ram_bank * 0x2000 + offset_address] = data;
    }
}

pub fn create_mbc(rom: &Vec<u8>) -> MBC {
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

    //let ram_size_code = rom[0x149];
    /* let ram_size = match ram_size_code {
        0x00 => 1,
        _ => panic!("unsupported ram_size provided"),
    }; */

    match mbc_type_code {
        0x00 | 0x01 => {
            let upper_bound = 0x3FFF + 0x4000 * (rom_size - 1) as usize;
            let rom_bank = rom[0x0000..=upper_bound].to_vec();
            let ram_bank = vec![0; 0x2000];

            MBC {
                rom_banks: rom_bank,
                total_rom_banks: rom_size as usize,
                upper_rom_bank: 1,

                ram_banks: ram_bank,
                upper_ram_bank: 0,

                ram_enabled: true,
                mode: false,
            }
        }
        _ => panic!("unsupported MBC type"),
    }
}