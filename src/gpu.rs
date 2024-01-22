use crate::memory::Memory;

enum PpuRegister {
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

pub enum PpuState {
    Oam,
    HBlank,
    VBlank,
    Drawing,
}

pub struct Ppu {
    ticks: usize,
    state: PpuState,
    current_line: Vec<u8>,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            ticks: 0,
            state: PpuState::Oam,
            current_line: Vec::new(),
        }
    }
}

pub fn update_ppu(ppu: &mut Ppu, ticks: u8) {
    
    

}