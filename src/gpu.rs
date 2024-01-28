#![allow(unused)]
use crate::memory::Memory;

/// how many ppu dots each cycle takes
/// includes the amount from the previous mode
const OAM_CYCLES: usize = 80;
const DRAW_CYCLES: usize = 178;
const HBLANK_CYCLES: usize = 456;

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
    // this is more of an interrupt register but is still
    // used regularly by the ppu and so should be here too
    IF=0xFF0F
}
#[derive(Debug)]
enum PpuState {
    Oam,
    HBlank,
    VBlank,
    Drawing,
}

pub struct Ppu {
    ticks: usize,
    state: PpuState,
    oam_buffer: Vec<[u8; 4]>,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            ticks: 0,
            state: PpuState::Oam,
            oam_buffer: Vec::new(),
        }
    }
}

/// updates the ppu, called in between instructions
pub fn update_ppu(ppu: &mut Ppu, mem: &mut Memory, ticks: u8) -> Option<Vec<u8>> {
    use PpuState::*;
    ppu.ticks += ticks as usize * 4;

    match ppu.state {
        Oam => {oam_tick(ppu, mem)},
        Drawing => {return draw_tick(ppu, mem)},
        HBlank => {hblank_tick(ppu, mem)}, // wait
        VBlank => {vblank_tick(ppu, mem)}, // wait
    }
    return None;
}

// the entire oam scan can be done at the end of its cycle
// since the cpu cant modify the data here anyways
// also doesnt modify the memory
fn oam_tick(ppu: &mut Ppu, mem: &mut Memory) {
    if ppu.ticks < OAM_CYCLES {
        return;
    }
    ppu.state = PpuState::Drawing;
}
fn draw_tick(ppu: &mut Ppu, mem: &mut Memory) -> Option<Vec<u8>> {
    if ppu.ticks < DRAW_CYCLES {
        return None;
    }

    // this bit in the middle will do all the drawing
    let lcdc = mem.read(PpuRegister::LCDC as u16);

    // the screen is just off
    if lcdc & 0b1000_0000 == 0 {
        return Some(vec![4; 160]);
    }

    // the number of pixels pushed to the lcd
    let mut screen_pixels: Vec<u8> = Vec::new();

    let background_pixels = draw_background(mem, lcdc);
    let bg_pallete = mem.read(PpuRegister::BGP as u16);

    // each pixel is decided sequentially
    for i in 0..160 {
        screen_pixels.push(to_pallette(background_pixels[i], bg_pallete))
    }

    stat_interrupt(mem, 3);
    ppu.state = PpuState::HBlank;
    return Some(screen_pixels);
}

fn hblank_tick(ppu: &mut Ppu, mem: &mut Memory) {
    if ppu.ticks < HBLANK_CYCLES {
        return;
    }

    let ly = mem.read(PpuRegister::LY as u16);
    mem.write(PpuRegister::LY as u16, 0);

    // also reset the ticks for next line
    ppu.ticks = 0;


    if ly == 144 {
        stat_interrupt(mem, 4);
        ppu.state = PpuState::VBlank;
        return;
    }

    stat_interrupt(mem, 5);
    ppu.state = PpuState::Oam;

}

fn vblank_tick(ppu: &mut Ppu, mem: &mut Memory) {
    if ppu.ticks < HBLANK_CYCLES*10 {
        return;
    }

    stat_interrupt(mem, 5);
    ppu.state = PpuState::Oam;
    ppu.ticks = 0;
}

#[inline]
fn stat_interrupt(mem: &mut Memory, mode: u8) {
    let stat = mem.read(PpuRegister::STAT as u16);
    if stat & (1<<mode) != 0 {
        let interrupt_flag = mem.read(PpuRegister::IF as u16);
        mem.write(PpuRegister::IF as u16, interrupt_flag|0b0000_0010);
    }
}

fn to_pallette(index: u8, palette: u8) -> u8 {
    if index == 4 { return index }
    (palette >> index*2) & 0b0000_0011
}

fn draw_sprites(mem: &Memory, lcdc: u8) -> Vec<u8> {
    let ly = mem.read(PpuRegister::LY as u16);

    // oam scan
    let mut oam_buffer = Vec::new();
    for i in 0..40 {
        let oam_sprite = mem.oam_search(i);
        
        if oam_sprite[1] == 0 { continue; } // x-position must be greater than 0
        if ly + 16 < oam_sprite[0] { continue; } // ly+16 must be greater than or equal to sprite y-position
        if ly + 16 >= oam_sprite[0] + 8 { continue; } // ly+16 must be less than sprite y-position + sprite height

        oam_buffer.push(oam_sprite);

        if oam_buffer.len() == 10 { 
            break; // only the first ten items are wanted
        }
    }

    let mut pixels: Vec<u8> = Vec::new();
    for sprite in oam_buffer {
        
    }

    todo!();
}

fn draw_window(mem: &Memory, lcdc: u8) -> Vec<u8> {
    if lcdc & 0b0010_0001 == 0 {
        return vec![4; 160];
    }
    let wy = mem.read(PpuRegister::WY as u16);
    let ly = mem.read(PpuRegister::LY as u16);

    // the window will display just not on this line yet
    if wy > ly {
        return vec![4; 160];
    }

    let mut window_pixels = Vec::new();

    return window_pixels;
}

fn draw_background(mem: &Memory, lcdc: u8) -> Vec<u8> {
    if lcdc & 0b0000_0001 == 0 {
        return vec![4; 160];
    }

    let mut background_pixels = Vec::new();

    let bg_map_address = if lcdc & 0b0000_1000 != 0 { 0x9C00 } else { 0x9800 };
    let tile_addressing: u16 = if lcdc & 0b0001_0000 != 0 { 0x8000 } else { 0x8800 };

    let ly = mem.read(PpuRegister::LY as u16);

    let background_line = ly.wrapping_add(mem.read(PpuRegister::SCY as u16));
    // which row of tile indexes in the background are used
    let bg_tile_row = background_line as u16 / 8;
    // which row of each tile is being used
    let tile_row = background_line % 8;

    // this tile may not be fully displayed and so i will deal with it seperately
    let scx = mem.read(PpuRegister::SCX as u16) as u16;

    let tile_index = bg_map_address + (bg_tile_row * 32) + (scx / 8);

    let tile = mem.read_bg_tile(tile_index, tile_addressing);
    let tile_row = tile[background_line as usize % 8]; 
    let pixels_shown = scx%8;

    for i in (0..pixels_shown).rev() {
        background_pixels.push((tile_row >> (i*2) & 0b0000_0011) as u8);
    }
    let mut tile_number = 0;
    while background_pixels.len() != 160 {
        // get the next tile
        tile_number += 1;
        let tile_index = bg_map_address + (bg_tile_row * 32) + (scx/8 + tile_number) % 32;
        let tile = mem.read_bg_tile(tile_index, tile_addressing);
        let tile_row = tile[background_line as usize % 8];
        
        for i in (0..8).rev() {
            background_pixels.push((tile_row >> (i*2) & 0b0000_0011) as u8);
            if background_pixels.len() == 160 {
                break;
            }
        }
    }
    return background_pixels;
}