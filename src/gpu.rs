use std::usize;
use crate::memory::Memory;
use crate::util::ppu::*;
use crate::util::INTERRUPT_F_ADDRESS;

// this number just represents that no pixel should be here
pub const BLANK_PIXEL: u8 = 4;

#[inline]
fn stat_interrupt(mem: &mut Memory, interrupt_index: u8, mode: u8) {
    let mut stat = mem.unchecked_read(PpuRegisters::STAT as u16);
    stat &= 0b1111_1100;
    stat |= mode;
    mem.write(PpuRegisters::STAT as u16, stat);

    // just means i dont want anything else happening
    if interrupt_index == 0 || mode == 3 {
        return;
    }
    if stat & (1<<interrupt_index) != 0 {
        let interrupt_flag = mem.unchecked_read(INTERRUPT_F_ADDRESS as u16);
        mem.write(INTERRUPT_F_ADDRESS as u16, interrupt_flag|0b0000_0010);
    }
    if mode == 1 {
        let i_flag = mem.unchecked_read(INTERRUPT_F_ADDRESS as u16);
        mem.write(INTERRUPT_F_ADDRESS as u16, i_flag|1);
    }
}

fn to_palette(index: u8, palette: u8) -> u8 {
    if index == 4 { return index }
    let lcdc_color = (palette >> index*2) & 0b0000_0011;
    lcdc_color
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
    window_line: usize
}
impl Ppu {
    pub fn default() -> Self {
        Self {
            ticks: 0,
            state: PpuState::Oam,
            window_line: 0,
        }
    }
    pub fn line_reset(&mut self) {
        self.state = PpuState::Oam;
        self.ticks = 0;
    }
}

/// updates the ppu, called in between instructions
pub fn update_ppu(ppu: &mut Ppu, mem: &mut Memory, ticks: u8) -> Option<Vec<u8>> {
    use PpuState::*;

    let lcdc = mem.unchecked_read(PpuRegisters::LCDC as u16);
    if lcdc & 0b1000_0000 == 0 {
        return None;
    }

    // lyc == ly check should be done right at the start
    if ppu.ticks == 0 {
        let mut stat = mem.unchecked_read(PpuRegisters::STAT as u16);
        let condition_met = mem.unchecked_read(PpuRegisters::LY as u16) == mem.unchecked_read(PpuRegisters::LYC as u16);

        stat &= 0b1111_1011;
        stat |= (condition_met as u8) << 2;

        if stat & 0b0100_0000 != 0 && condition_met {
            let interrupt_flag = mem.unchecked_read(INTERRUPT_F_ADDRESS);
            mem.write(INTERRUPT_F_ADDRESS, interrupt_flag|0b0000_0010);
        }
        mem.write(PpuRegisters::STAT as u16, stat);
    }

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
            let ly = mem.unchecked_read(PpuRegisters::LY as u16);
            mem.write(PpuRegisters::LY as u16, ly+1);
            ppu.line_reset();
            
            if ly == 143 {
                stat_interrupt(mem, STAT_VBLANK, 1);
                ppu.state = PpuState::VBlank;
                return None;
            } 
            stat_interrupt(mem, STAT_OAM, 2);
        }, // waits
        VBlank => {    
            if ppu.ticks < HBLANK_CYCLES*10 {
                let new_ly = 144+(ppu.ticks/HBLANK_CYCLES) as u8;
                mem.write(PpuRegisters::LY as u16, new_ly);
                return None;
            }
            stat_interrupt(mem, STAT_OAM, 2);
            *ppu = Ppu::default();
            mem.write(PpuRegisters::LY as u16, 0)
        }, // waits
    }
    return None;
}

fn draw(ppu: &mut Ppu, mem: &mut Memory) -> Vec<u8> {
    // this bit in the middle will do all the drawing
    let lcdc = mem.unchecked_read(PpuRegisters::LCDC as u16);
    let ly = mem.unchecked_read(PpuRegisters::LY as u16);

    // the number of pixels pushed to the lcd
    let mut screen_pixels: Vec<u8> = Vec::new();

    let background_pixels = draw_background(mem, lcdc, ly);
    let window_pixels = draw_window(ppu, mem, lcdc, ly);

    // also the palle
    let bg_palette = mem.unchecked_read(PpuRegisters::BGP as u16);

    let sprite_pixels = draw_sprites(mem, lcdc, ly);
    let sprite_palette_0 = mem.unchecked_read(PpuRegisters::OBP0 as u16);
    let sprite_palette_1 = mem.unchecked_read(PpuRegisters::OBP1 as u16);


    // each pixel is decided sequentially
    for i in 0..160 {
        let win_pixel = window_pixels[i];
        let sprite_pixel = sprite_pixels[i];
        let bg_pixel = background_pixels[i];

        let sprite_pallete_i = sprite_pixel & 0b0001_0000 == 0;
        let sprite_pallete = match sprite_pallete_i {
            true => sprite_palette_0,
            false => sprite_palette_1,
        };

        // sprite pixel is the priority
        if sprite_pixel & 0b1000_0000 == 0 {
            let sprite_color = sprite_pixel & 0b0000_0011;

            // empty
            if sprite_color == 0 || sprite_color == BLANK_PIXEL {
                if win_pixel != 0 && win_pixel != BLANK_PIXEL {
                    screen_pixels.push(to_palette(win_pixel, bg_palette));
                    continue;
                }
                screen_pixels.push(to_palette(bg_pixel, bg_palette));
                continue;
            }
            screen_pixels.push(to_palette(sprite_color, sprite_pallete));
            continue;
        }
        if sprite_pixel == BLANK_PIXEL {
            screen_pixels.push(match win_pixel {
                BLANK_PIXEL => to_palette(bg_pixel, bg_palette),
                _ => to_palette(win_pixel, bg_palette),
            });
            continue;
        }
        // there is a sprite pixel, but 1-3 of bg & window draw over it
        screen_pixels.push(match (win_pixel, bg_pixel) {
            (0|BLANK_PIXEL, 0|BLANK_PIXEL) => to_palette(sprite_pixel&3, sprite_pallete),
            (_, 0|BLANK_PIXEL) => to_palette(win_pixel, bg_palette),
            (0|BLANK_PIXEL, _) => to_palette(bg_pixel, bg_palette),
            _ => to_palette(win_pixel, bg_palette),
        });
    }
    return screen_pixels;
}

fn draw_sprites(mem: &Memory, lcdc: u8, ly: u8) -> Vec<u8> {
    let ly = ly + 16;
    if lcdc & 0b0000_0010 == 0 {
        return vec![BLANK_PIXEL; 160];
    }
    let obj_size = if lcdc & 0b0000_0100 == 0 { 8 } else { 16 };

    // oam scan
    let mut oam_buffer = Vec::new();
    for i in 0..40 {
        let oam_sprite = mem.oam_search(i);

        if oam_sprite[0] >= 160 || oam_sprite[0] == 0 {
            continue;
        }
        else if ly < oam_sprite[0] { continue; } // ly+16 must be greater than or equal to sprite y-position
        else if ly >= oam_sprite[0] + obj_size { continue; } // ly+16 must be less than sprite y-position + sprite height

        oam_buffer.push(oam_sprite);
        if oam_buffer.len() == 10 {
            break; // only the first ten items are wanted
        }
    }
    oam_buffer.sort_by(|a, b| a[1].cmp(&b[1]));

    let mut sprite_pixels = vec![BLANK_PIXEL; 300]; // 160 (length of lcd + 8x2 pad)
    for sprite in oam_buffer {
        let tile_index;
        if obj_size == 16 {
            let base_tile_index = sprite[2] & 0b1111_1110;
            let used_tile = ly > sprite[0] + 8;
            let reversed = sprite[3] & 0b0100_0000 != 0;
            tile_index = base_tile_index | if reversed {!used_tile as u8} else {used_tile as u8};
        } else {
            tile_index = sprite[2];
        }

        let mut tile_data = mem.read_tile(0x8000 + (tile_index as u16*16));
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

fn draw_window(ppu: &mut Ppu, mem: &Memory, lcdc: u8, ly: u8) -> Vec<u8> {
    // the window just isnt drawing
    if lcdc & 0b0010_0001 != 0b0010_0001 {
        return vec![BLANK_PIXEL; 160];
    }

    let wy = mem.unchecked_read(PpuRegisters::WY as u16);
    let wx = mem.unchecked_read(PpuRegisters::WX as u16);

    // the window will display just not on this line yet
    // or too far to the right to be displayed
    if wy > ly || wx > 167 {
        return vec![BLANK_PIXEL; 160];
    }

    let map_address = if lcdc & 0b0100_0000 != 0 { 0x9C00 } else { 0x9800 };
    let addressing = if lcdc & 0b0001_0000 != 0 { 0x8000 } else { 0x8800 };

    let mut window_pixels: Vec<u8> = vec![BLANK_PIXEL; wx as usize];

    // 0 = top layer, 1 = second to top, 2=...
    let layer_shown = ppu.window_line;
    ppu.window_line = ppu.window_line + 1;

    let starting_address = (map_address + (layer_shown/8)*32) as u16;
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

    // the address and addressing of the map and subsequent tiles
    let map_address = if lcdc & 0b0000_1000 != 0 { 0x9C00 } else { 0x9800 };
    let addressing: u16 = if lcdc & 0b0001_0000 != 0 { 0x8000 } else { 0x8800 };

    // this is the line of the background which we are drawing
    let background_line = ly.wrapping_add(mem.unchecked_read(PpuRegisters::SCY as u16));

    // which row of tile indexes in the background are used
    let bg_tile_row = background_line as u16 / 8;
    let tile_row = background_line as usize % 8;

    let scx = mem.unchecked_read(PpuRegisters::SCX as u16) as u16;

    // drawing all of the tiles
    let mut tile_number = 0;
    'background: loop {
        // get the next tile
        let tile_index = map_address + (bg_tile_row * 32) + ((scx/8 + tile_number) % 32);
        let tile = mem.read_bg_tile(tile_index, addressing);
        let tile_row_data = tile[tile_row];
        let pixels = get_individual_pixels(tile_row_data);
        for pixel in pixels {
            background_pixels.push(pixel);
            if background_pixels.len() == 168 {
                break 'background
            }
        }
        tile_number += 1;
    }
    return background_pixels[((scx%8) as usize)..].to_vec();
}

fn get_individual_pixels(tile_row: u16) -> Vec<u8> {
    let mut pixels = Vec::new();
    for i in (0..8).rev() {
        pixels.push((tile_row >> (i*2) & 3) as u8);
    }
    return pixels
}