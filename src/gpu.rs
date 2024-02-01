#![allow(unused)]
use std::usize;

use crate::memory::Memory;

/// how many ppu dots each cycle takes
/// includes the amount from the previous mode
const OAM_CYCLES: usize = 80;
const DRAW_CYCLES: usize = 178;
const HBLANK_CYCLES: usize = 456;

// this number just represents that no pixel should be here
const BLANK_PIXEL: u8 = 4;

const STAT_OAM: u8 = 5;
const STAT_VBLANK: u8 = 4;
const STAT_HBLANK: u8 = 3;


#[inline]
fn stat_interrupt(mem: &mut Memory, interrupt_index: u8, mode: u8) {
    let mut stat = mem.read(PpuRegister::STAT as u16);
    stat &= 0b1111_1100;
    stat |= mode;

    if mode == 3 {
        return;
    }

    if stat & (1<<interrupt_index) != 0 {
        let interrupt_flag = mem.read(PpuRegister::IF as u16);
        mem.write(PpuRegister::IF as u16, interrupt_flag|0b0000_0010);
    }
}

fn to_palette(index: u8, palette: u8) -> u8 {
    if index == 4 { return index }
    let lcdc_color = (palette >> index*2) & 0b0000_0011;
    lcdc_color
}

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
    ppu.ticks += ticks as usize;

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
    stat_interrupt(mem, 0, 3);
}
fn draw_tick(ppu: &mut Ppu, mem: &mut Memory) -> Option<Vec<u8>> {
    if ppu.ticks < DRAW_CYCLES {
        return None;
    }

    // this bit in the middle will do all the drawing
    let lcdc = mem.read(PpuRegister::LCDC as u16);

    // the screen is just off
    if lcdc & 0b1000_0000 == 0 {
        ppu.state = PpuState::HBlank;
        return Some(vec![BLANK_PIXEL; 160]);
    }
    // the number of pixels pushed to the lcd
    let mut screen_pixels: Vec<u8> = Vec::new();

    let background_pixels = draw_background(mem, lcdc);
    let window_pixels = draw_window(mem, lcdc);

    // also the palle
    let bg_palette = mem.read(PpuRegister::BGP as u16);

    let sprite_pixels = draw_sprites(mem, lcdc);
    let sprite_palette = mem.read(PpuRegister::OBP0 as u16);


    // each pixel is decided sequentially
    for i in 0..160 {
        if sprite_pixels[i] != 0 && sprite_pixels[i] != BLANK_PIXEL {
            screen_pixels.push(to_palette(sprite_pixels[i], sprite_palette));
            continue;
        }
        screen_pixels.push(to_palette(background_pixels[i], bg_palette));

    }

    stat_interrupt(mem, STAT_HBLANK, 0);
    ppu.state = PpuState::HBlank;
    return Some(screen_pixels);
}

fn hblank_tick(ppu: &mut Ppu, mem: &mut Memory) {
    if ppu.ticks < HBLANK_CYCLES {
        return;
    }

    let ly = mem.read(PpuRegister::LY as u16);

    // also reset the ticks for next line
    ppu.ticks = 0;

    if ly == 143 {
        mem.write(PpuRegister::LY as u16, 144);
        stat_interrupt(mem, STAT_VBLANK, 1);
        ppu.state = PpuState::VBlank;

        let i_flag = mem.read(PpuRegister::IF as u16);
        mem.write(PpuRegister::IF as u16, i_flag|1);

        return;
    } 
    mem.write(PpuRegister::LY as u16, ly+1);

    stat_interrupt(mem, STAT_OAM, 2);
    ppu.state = PpuState::Oam;

}

fn vblank_tick(ppu: &mut Ppu, mem: &mut Memory) {
    // ten full lines wil have to be drawn
    if ppu.ticks < HBLANK_CYCLES*10 {
        mem.write(PpuRegister::LY as u16, 144+(ppu.ticks/HBLANK_CYCLES) as u8);
        return;
    }

    stat_interrupt(mem, STAT_OAM, 2);
    ppu.state = PpuState::Oam;
    ppu.ticks = 0;
    mem.write(PpuRegister::LY as u16, 0);
}

fn draw_sprites(mem: &Memory, lcdc: u8) -> Vec<u8> {
    let ly = mem.read(PpuRegister::LY as u16);

    if lcdc & 0b0000_0010 == 0 {
        return vec![BLANK_PIXEL; 160];
    }

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

    let mut sprite_pixels = vec![BLANK_PIXEL; 160];

    let ly = mem.read(PpuRegister::LY as u16);

    for sprite in oam_buffer {
        // just an early check to see if we should bother rendering it
        if sprite[1] == 0 || sprite[1] >= 160 {
            continue;
        }

        let tile_index = sprite[2];
        let mut tile_data = mem.read_tile(tile_index as u16*32 + 0x8000);

        // the sprite is flipped vertically
        if sprite[3] & 0b0100_0000 != 0 {
            tile_data.reverse();
        }

        let tile_row = ly - (sprite[0] - 16);
        let mut row_data = tile_data[tile_row as usize];


        // the sprite is flipped horizontally
        if sprite[3] & 0b0010_0000 != 0 {
            // u16::reverse_bits() cannot be used as it changes the colors
            let base_data = row_data;
            row_data = 0;
            for i in 0..8 {
                row_data |= (base_data & 0b0000_0011<<(i*2))>>(i*2);
            }
        }

        // adding the data to the buffer
        if sprite[1] < 8 {
            // only render the last parts of it
            let pixels_shown = (8 - sprite[1]) as usize;
            for i in pixels_shown..8 {
                let i = i as usize;
                if sprite_pixels[i] != 0 {
                    continue;
                }
                
                let new_pixel = row_data>>(i*2)&0b11;
                sprite_pixels[i-pixels_shown] = new_pixel as u8;
            }
            continue;
        }
        if sprite[1] > 160 {
            // only render the first parts of it
            let pixels_shown = 168 - sprite[1];
            for i in 0..pixels_shown {
                let i = i as usize;
                // already a pixel there
                if sprite_pixels[i+152-sprite[1] as usize] != 0 {
                    continue;
                }
                let new_pixel = row_data>>(i*2)&0b11;
                sprite_pixels[i+152-sprite[1] as usize] = new_pixel as u8;
            }
            continue;
        }

        // the full sprite should be displayed
        for i in 0..8 {
            if sprite_pixels[i + sprite[1] as usize] != 0 {
                continue;
            }
            let new_pixel = row_data>>(i*2)&0b11;
            sprite_pixels[i+sprite[1] as usize] = new_pixel as u8;
        }
    }
    sprite_pixels
}

fn draw_window(mem: &Memory, lcdc: u8) -> Vec<u8> {
    // the window just isnt drawing
    if lcdc & 0b0010_0001 != 0b0010_0001 {
        return vec![BLANK_PIXEL; 160];
    }

    let wy = mem.read(PpuRegister::WY as u16);
    let ly = mem.read(PpuRegister::LY as u16);

    // the window will display just not on this line yet
    if wy > ly {
        return vec![BLANK_PIXEL; 160];
    }

    let wx = mem.read(PpuRegister::WX as u16);
    let map_address = if lcdc & 0b0100_0000 != 0 { 0x9C00 } else { 0x9800 };
    let addressing = if lcdc & 0b0001_0000 != 0 { 0x8000 } else { 0x8800 };

    let mut window_pixels: Vec<u8> = vec![BLANK_PIXEL; wx as usize];

    // 0 = top layer, 1 = second to top, 2=...
    let layer_shown = (wy - ly) as usize;
    let tile_row = layer_shown as u16 / 8;
    let starting_address = map_address + (tile_row/8)*32;

    let mut tile_number = 0;

    'window: loop {
        // dont need to bother with wrapping as windows do not wrap
        let tile_address = starting_address + tile_number;
        let tile_data = mem.read_bg_tile(tile_address, addressing);
        let tile_row_data = tile_data[layer_shown % 8];

        for i in (0..8).rev() {
            window_pixels.push((tile_row >> (i*2) & 0b0000_0011) as u8);
            if window_pixels.len() == 167 {
                break 'window;
            }
        }

        tile_number += 1;
    }
    return window_pixels[7..].to_vec();
}

fn draw_background(mem: &Memory, lcdc: u8) -> Vec<u8> {
    if lcdc & 0b0000_0001 == 0 {
        return vec![BLANK_PIXEL; 160];
    }

    let mut background_pixels = Vec::new();

    let map_address = if lcdc & 0b0000_1000 != 0 { 0x9C00 } else { 0x9800 };
    let addressing: u16 = if lcdc & 0b0001_0000 != 0 { 0x8000 } else { 0x8800 };

    let ly = mem.read(PpuRegister::LY as u16);

    let background_line = ly.wrapping_add(mem.read(PpuRegister::SCY as u16));
    // which row of tile indexes in the background are used
    let bg_tile_row = background_line as u16 / 8;
    // which row of each tile is being used
    let tile_row = background_line % 8;

    let scx = mem.read(PpuRegister::SCX as u16) as u16;


    // this tile may not be fully displayed and so i will deal with it seperately
    {
        let tile_index = map_address + (bg_tile_row * 32) + (scx / 8);

        let tile = mem.read_bg_tile(tile_index, addressing);
        let tile_row = tile[background_line as usize % 8]; 
        let pixels_shown = 8-(scx%8);
    
        for i in (0..pixels_shown).rev() {
            background_pixels.push((tile_row >> (i*2) & 0b0000_0011) as u8);
        }
    }

    // the rest of the tiles
    let mut tile_number = 0;
    'background: loop {
        // get the next tile
        tile_number += 1;
        let tile_index = map_address + (bg_tile_row * 32) + (scx/8 + tile_number) % 32;
        let tile = mem.read_bg_tile(tile_index, addressing);
        let tile_row = tile[background_line as usize % 8];
        
        for i in (0..8).rev() {
            background_pixels.push((tile_row >> (i*2) & 0b0000_0011) as u8);
            if background_pixels.len() == 160 {
                break 'background;
            }
        }
    }
    return background_pixels;
}