#![allow(unused, dead_code)]
use std::usize;

use macroquad::miniquad::window;

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

    if mode == 1 {
        let i_flag = mem.read(PpuRegister::IF as u16);
        mem.write(PpuRegister::IF as u16, i_flag|1);
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
}
impl Default for Ppu {
    fn default() -> Self {
        Self {
            ticks: 0,
            state: PpuState::Oam,
        }
    }
}
impl Ppu {
    fn reset(&mut self) {
        *self = Ppu::default();
    }
}


/// updates the ppu, called in between instructions
pub fn update_ppu(ppu: &mut Ppu, mem: &mut Memory, ticks: u8) -> Option<Vec<u8>> {
    use PpuState::*;
    ppu.ticks += ticks as usize;

    match ppu.state {
        Oam => {
            if ppu.ticks < OAM_CYCLES {
                return None;
            }
            ppu.state = PpuState::Drawing;
            stat_interrupt(mem, 0, 3);
        },
        Drawing => {
            if ppu.ticks < DRAW_CYCLES {
                return None;
            }
            stat_interrupt(mem, STAT_HBLANK, 0);
            ppu.state = PpuState::HBlank;
            return Some(draw(ppu, mem));
        },
        HBlank => {
            if ppu.ticks < HBLANK_CYCLES {
                return None;
            }
            let ly = mem.read(PpuRegister::LY as u16);
            mem.write(PpuRegister::LY as u16, ly+1);
            ppu.ticks = 0;
    
            if ly == 143 {
                stat_interrupt(mem, STAT_VBLANK, 1);
                ppu.state = PpuState::VBlank;
                return None;
            } 
            stat_interrupt(mem, STAT_OAM, 2);
            ppu.state = PpuState::Oam
        }, // waits
        VBlank => {    
            if ppu.ticks < HBLANK_CYCLES*10 {
                let new_ly = 144+(ppu.ticks/HBLANK_CYCLES) as u8;
                mem.write(PpuRegister::LY as u16, new_ly);
                return None;
            }
            stat_interrupt(mem, STAT_OAM, 2);
            *ppu = Ppu::default();
            mem.write(PpuRegister::LY as u16, 0)
        }, // waits
    }
    return None;
}
fn draw(ppu: &mut Ppu, mem: &mut Memory) -> Vec<u8> {
    // this bit in the middle will do all the drawing
    let lcdc = mem.read(PpuRegister::LCDC as u16);
    let ly = mem.read(PpuRegister::LY as u16);
    // the screen is just off
    if lcdc & 0b1000_0000 == 0 {
        ppu.state = PpuState::HBlank;
        return vec![BLANK_PIXEL; 160];
    }
    // the number of pixels pushed to the lcd
    let mut screen_pixels: Vec<u8> = Vec::new();

    let background_pixels = draw_background(mem, lcdc, ly);
    let window_pixels = draw_window(mem, lcdc, ly);

    // also the palle
    let bg_palette = mem.read(PpuRegister::BGP as u16);

    let sprite_pixels = draw_sprites(mem, lcdc, ly);
    let sprite_palette_0 = mem.read(PpuRegister::OBP0 as u16);
    let sprite_palette_1 = mem.read(PpuRegister::OBP1 as u16);


    // each pixel is decided sequentially
    for i in 0..160 {
        let win_pixel = window_pixels[i];
        let sprite_pixel = sprite_pixels[i];
        let bg_pixel = background_pixels[i];

        if sprite_pixel&0b1000_0000 == 0 {
            if sprite_pixel != 0 && sprite_pixel != BLANK_PIXEL {
                let palette_id = sprite_pixel&0b0001_0000==0;
                let palette = if palette_id { sprite_palette_0 } else { sprite_palette_1} ;
                screen_pixels.push(to_palette(sprite_pixel&3, palette));
                continue;
            }
            if win_pixel != BLANK_PIXEL {
                screen_pixels.push(to_palette(win_pixel, bg_palette));
                continue;
            }
            screen_pixels.push(to_palette(bg_pixel, bg_palette));
            continue;
        }

        // the sprite is not the priority
        if window_pixels[i] != BLANK_PIXEL {
            screen_pixels.push(to_palette(win_pixel, bg_palette));
            continue;
        }
        if bg_pixel != 0 {
            screen_pixels.push(to_palette(bg_pixel, bg_palette));
            continue;
        }
        let palette_id = sprite_pixel&0b0001_0000==0;
        let palette = if palette_id { sprite_palette_0 } else { sprite_palette_1} ;
        screen_pixels.push(to_palette(sprite_pixel&3, palette));
    }
    return screen_pixels;
}

fn draw_sprites(mem: &Memory, lcdc: u8, ly: u8) -> Vec<u8> {
    let ly = ly + 16;
    if lcdc & 0b0000_0010 == 0 {
        return vec![BLANK_PIXEL; 160];
    }

    // oam scan
    let mut oam_buffer = Vec::new();
    for i in 0..40 {
        let oam_sprite = mem.oam_search(i);

        if oam_sprite[0] >= 160 || oam_sprite[0] == 0 {
            continue;
        }
        else if ly < oam_sprite[0] { continue; } // ly+16 must be greater than or equal to sprite y-position
        else if ly >= oam_sprite[0] + 8 { continue; } // ly+16 must be less than sprite y-position + sprite height

        oam_buffer.push(oam_sprite);
        if oam_buffer.len() == 10 { 
            break; // only the first ten items are wanted
        }
    }
    oam_buffer.sort_by(|a, b| a[1].cmp(&b[1]));

    let mut sprite_pixels = vec![BLANK_PIXEL; 256]; // 160 (length of lcd + 8x2 pad)
    for sprite in oam_buffer {
        let mut tile_data = mem.read_tile(0x8000 + (sprite[2] as u16*16));
        if sprite[3] & 0b0100_0000 != 0 {
            tile_data.reverse();
        }
        let row = (ly - sprite[0]) as usize % 8;

        let row_data = tile_data[row];
        let mut row_pixels = get_individual_pixels(row_data);
        if sprite[3] & 0b0010_0000 != 0 {
            row_pixels.reverse();
        }

        for i in 0..8 {
            let i = i as usize;
            match sprite_pixels[i+sprite[1] as usize] {
                0 | BLANK_PIXEL => {
                    let pixel_data = row_pixels[i] | sprite[3] & 0b1001_0000;
                    sprite_pixels[i+sprite[1] as usize] = pixel_data;
                },
                _ => continue,
            }
        }
    }
    sprite_pixels[8..168].to_vec()
}

fn draw_window(mem: &Memory, lcdc: u8, ly: u8) -> Vec<u8> {
    // the window just isnt drawing
    if lcdc & 0b0010_0001 != 0b0010_0001 {
        return vec![BLANK_PIXEL; 160];
    }

    let wy = mem.read(PpuRegister::WY as u16);

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

        let pixels = get_individual_pixels(tile_row_data);
        for pixel in pixels {
            window_pixels.push(pixel);
            if window_pixels.len() == 167 {
                break 'window;
            }
        }
        tile_number += 1;
    }
    return window_pixels[7..].to_vec();
}

fn draw_background(mem: &Memory, lcdc: u8, ly: u8) -> Vec<u8> {
    if lcdc & 0b0000_0001 == 0 {
        return vec![BLANK_PIXEL; 160];
    }

    let mut background_pixels = Vec::new();

    let map_address = if lcdc & 0b0000_1000 != 0 { 0x9C00 } else { 0x9800 };
    let addressing: u16 = if lcdc & 0b0001_0000 != 0 { 0x8000 } else { 0x8800 };

    let background_line = ly.wrapping_add(mem.read(PpuRegister::SCY as u16));
    // which row of tile indexes in the background are used
    let bg_tile_row = background_line as u16 / 8;
    // which row of each tile is being used
    let tile_row = (background_line % 8) as usize;

    let scx = mem.read(PpuRegister::SCX as u16) as u16;

    // this tile may not be fully displayed and so i will deal with it seperately
    {
        let tile_index = map_address + (bg_tile_row * 32) + (scx / 8);

        let tile = mem.read_bg_tile(tile_index, addressing);
        let tile_row = tile[tile_row]; 
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
        let tile_row_data = tile[background_line as usize % 8];
        let pixels = get_individual_pixels(tile_row_data);
        for pixel in pixels {
            background_pixels.push(pixel);
            if background_pixels.len() == 160 {
                break 'background
            }
        }
    }
    return background_pixels;
}

fn get_individual_pixels(tile_row: u16) -> Vec<u8> {
    let mut pixels = Vec::new();
    for i in (0..8).rev() {
        pixels.push((tile_row >> (i*2) & 0b0000_0011) as u8);
    }
    return pixels
}