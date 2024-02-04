pub fn little_endian_combine(a: u8, b: u8) -> u16 {
    (b as u16) << 8 | (a as u16)
}
pub fn combine(a: u8, b: u8) -> u16 {
    (a as u16) << 8 | b as u16
}
pub fn split(a: u16) -> (u8, u8) {
    (((a & 0xFF00) >> 8) as u8, (a & 0xFF) as u8)
}

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