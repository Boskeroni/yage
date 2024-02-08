pub fn little_endian_combine(a: u8, b: u8) -> u16 {
    (b as u16) << 8 | (a as u16)
}
pub fn combine(a: u8, b: u8) -> u16 {
    (a as u16) << 8 | b as u16
}
pub fn split(a: u16) -> (u8, u8) {
    (((a & 0xFF00) >> 8) as u8, (a & 0xFF) as u8)
}

// just helps with boot rom not failing the check
pub const NINTENDO_LOGO: [u8; 48] = 
    [0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
     0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
     0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E
];

pub const JOYPAD_ADDRESS: usize = 0xFF00;
pub const INTERRUPT_F_ADDRESS: u16 = 0xFF0F;
pub const INTERRUPT_E_ADDRESS: u16 = 0xFFFF;

pub mod ppu {
    pub enum PpuRegisters {
        LCDC=0xFF40,
        STAT=0xFF41,
        SCY=0xFF42,
        SCX=0xFF43,
        LY=0xFF44,
        LYC=0xFF45,
        BGP=0xFF47,
        OBP0=0xFF48,
        OBP1=0xFF49,
        WY=0xFF4A,
        WX=0xFF4B,
    }
    /// how many ppu dots each cycle takes
    /// includes the amount from the previous mode
    pub const OAM_CYCLES: usize = 80;
    pub const DRAW_CYCLES: usize = 178;
    pub const HBLANK_CYCLES: usize = 456;
    
    pub const STAT_OAM: u8 = 5;
    pub const STAT_VBLANK: u8 = 4;
    pub const STAT_HBLANK: u8 = 3;
}
pub enum TimerRegisters {
    DIV=0xFF04,
    TIMA=0xFF05,
    TMA=0xFF06,
    TAC=0xFF07,
}