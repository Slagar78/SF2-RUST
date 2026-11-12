// src/game/menu/window_engine.rs
#![allow(dead_code)]

use sdl3::pixels::Color;
use crate::engine::base_tiles::Vram;

pub const VDPTILE_SPACE: u16 = 0x20;
pub const VDPTILE_CORNER: u16 = 0x60;
pub const VDPTILE_H_BORDER: u16 = 0x61;
pub const VDPTILE_V_BORDER: u16 = 0x70;

pub const VDPTILE_MIRROR: u16 = 0x800;
pub const VDPTILE_FLIP: u16 = 0x1000;
pub const VDPTILE_PALETTE2: u16 = 0x2000;
pub const VDPTILE_PALETTE3: u16 = 0x4000;
pub const VDPTILE_PRIORITY: u16 = 0x8000;

pub const MAX_WINDOWS: usize = 16;
pub const SLOT_SIZE: usize = 1024;

#[derive(Clone, Copy, Default)]
pub struct WindowEntry {
    pub is_active: bool,
    pub width_height: u16,
    pub x_y: u16,
    pub layout_start_index: usize,
    pub orig_x_y: u16,
    pub target_x_y: u16,
    pub anim_length: u8,
    pub anim_counter: u8,
}

impl WindowEntry {
    pub fn is_animating(&self) -> bool {
        self.anim_counter < self.anim_length
    }
}

pub struct WindowEngine {
    pub entries: [WindowEntry; MAX_WINDOWS],
    pub window_tile_layouts: Vec<u16>,
}

impl WindowEngine {
    pub fn new() -> Self {
        Self {
            entries: [WindowEntry::default(); MAX_WINDOWS],
            window_tile_layouts: vec![0; MAX_WINDOWS * SLOT_SIZE],
        }
    }

    pub fn get_window_entry(&self, slot: usize) -> &WindowEntry {
        &self.entries[slot]
    }

    pub fn create_window_with_layout(&mut self, d1: u16, width: u8, height: u8, layout: &[u16]) -> Option<usize> {
        let slot = self.alloc_slot()?;
        let start_index = slot * SLOT_SIZE;

        for (i, &tile) in layout.iter().enumerate() {
            if i < SLOT_SIZE {
                self.window_tile_layouts[start_index + i] = tile;
            }
        }

        self.entries[slot] = WindowEntry {
            is_active: true,
            width_height: ((width as u16) << 8) | (height as u16),
            x_y: d1,
            layout_start_index: start_index,
            orig_x_y: d1,
            target_x_y: d1,
            anim_length: 0,
            anim_counter: 0,
        };
        Some(slot)
    }

    pub fn create_empty_window(&mut self, d1: u16, width: u8, height: u8) -> Option<usize> {
        let slot = self.alloc_slot()?;
        let start_index = slot * SLOT_SIZE;

        for i in 0..(width as usize * height as usize) {
            if i < SLOT_SIZE {
                self.window_tile_layouts[start_index + i] = VDPTILE_SPACE | VDPTILE_PALETTE3 | VDPTILE_PRIORITY;
            }
        }

        self.entries[slot] = WindowEntry {
            is_active: true,
            width_height: ((width as u16) << 8) | (height as u16),
            x_y: d1,
            layout_start_index: start_index,
            orig_x_y: d1,
            target_x_y: d1,
            anim_length: 0,
            anim_counter: 0,
        };
        self.draw_window_frame(slot);
        Some(slot)
    }

    fn alloc_slot(&mut self) -> Option<usize> {
        for i in 0..MAX_WINDOWS {
            if !self.entries[i].is_active { return Some(i); }
        }
        None
    }

    pub fn move_window(&mut self, slot: usize, dest_x_y: u16, length: u8) {
        let entry = &mut self.entries[slot];
        entry.orig_x_y = entry.x_y;
        entry.target_x_y = dest_x_y;
        entry.anim_length = length.saturating_mul(2);
        entry.anim_counter = 0;
    }

    pub fn tick(&mut self) {
        for entry in self.entries.iter_mut() {
            if !entry.is_active || !entry.is_animating() { continue; }
            
            entry.anim_counter += 1;

            let orig_x = ((entry.orig_x_y >> 8) as u8 as i8) as i32;
            let orig_y = ((entry.orig_x_y & 0xFF) as u8 as i8) as i32;
            let dest_x = ((entry.target_x_y >> 8) as u8 as i8) as i32;
            let dest_y = ((entry.target_x_y & 0xFF) as u8 as i8) as i32;
            
            let len = entry.anim_length as i32;
            let cnt = entry.anim_counter as i32;

            let cur_x = orig_x + ((dest_x - orig_x) * cnt / len);
            let cur_y = orig_y + ((dest_y - orig_y) * cnt / len);

            entry.x_y = ((cur_x as u8 as u16) << 8) | (cur_y as u8 as u16);
        }
    }

    fn draw_window_frame(&mut self, slot: usize) {
        let entry = self.entries[slot];
        let width = ((entry.width_height >> 8) & 0xFF) as usize;
        let height = (entry.width_height & 0xFF) as usize;
        let start = entry.layout_start_index;

        for r in 0..height {
            for c in 0..width {
                let idx = start + (r * width + c);
                let is_top = r == 0;
                let is_bottom = r == height - 1;
                let is_left = c == 0;
                let is_right = c == width - 1;

                let tile = if is_top && is_left { VDPTILE_CORNER | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else if is_top && is_right { VDPTILE_CORNER | VDPTILE_MIRROR | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else if is_bottom && is_left { VDPTILE_CORNER | VDPTILE_FLIP | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else if is_bottom && is_right { VDPTILE_CORNER | VDPTILE_MIRROR | VDPTILE_FLIP | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else if is_top { VDPTILE_H_BORDER | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else if is_bottom { VDPTILE_H_BORDER | VDPTILE_FLIP | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else if is_left { VDPTILE_V_BORDER | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else if is_right { VDPTILE_V_BORDER | VDPTILE_MIRROR | VDPTILE_PALETTE3 | VDPTILE_PRIORITY }
                    else { continue };

                if idx < self.window_tile_layouts.len() {
                    self.window_tile_layouts[idx] = tile;
                }
            }
        }
    }

    pub fn delete_window(&mut self, slot: usize) {
        if slot < MAX_WINDOWS {
            self.entries[slot].is_active = false;
        }
    }

    pub fn write_string(&mut self, slot: usize, x: u8, y: u8, text: &str) {
        let entry = &self.entries[slot];
        if !entry.is_active { return; }

        let width = ((entry.width_height >> 8) & 0xFF) as usize;
        let start_offset = entry.layout_start_index + (y as usize * width + x as usize);

        for (i, ch) in text.chars().enumerate() {
            // ЗАЩИТА: если текст доходит до правой рамки (width - 1), обрубаем его
            if x as usize + i >= width - 1 { break; } 
            if ch == '\0' { break; }
            
            let mut tile = (ch as u16) | VDPTILE_PALETTE3 | VDPTILE_PRIORITY;
            if (ch as u8) >= 0x60 { tile += 32; }
            
            let idx = start_offset + i;
            if idx < self.window_tile_layouts.len() {
                self.window_tile_layouts[idx] = tile;
            }
        }
    }

    pub fn draw_windows(&self, buf: &mut [u8], pitch: usize, vram: &Vram, palette: &[Color], anim_timer: u16, menu_choice: usize, x_offset: i32) {
        for entry in self.entries.iter() {
            if !entry.is_active { continue; }

            let width_tiles = ((entry.width_height >> 8) & 0xFF) as i32;
            let height_tiles = (entry.width_height & 0xFF) as i32;
            
            let x_pos = (((entry.x_y >> 8) as u8 as i8) as i32 * 8) + x_offset;
            let y_pos = ((entry.x_y & 0xFF) as u8 as i8) as i32 * 8;

            let mut index = entry.layout_start_index;

            for r in 0..height_tiles {
                for c in 0..width_tiles {
                    let word = self.window_tile_layouts[index];
                    index += 1;

                    let mut tile_idx = (word & 0x07FF) as usize;
                    if tile_idx == 0 { continue; }

                    let h_flip = (word & VDPTILE_MIRROR) != 0;
                    let v_flip = (word & VDPTILE_FLIP) != 0;
                    
                    let pal_bits = (word >> 13) & 0x03;
                    let pal_offset = match pal_bits {
                        0 => 0,
                        1 => 16,
                        2 => 32,
                        3 => 48,
                        _ => 0,
                    };

                    let is_selected = match menu_choice {
                        0 => tile_idx >= 0x0300 && tile_idx < 0x0309,
                        1 => tile_idx >= 0x0312 && tile_idx < 0x031B,
                        2 => tile_idx >= 0x0324 && tile_idx < 0x032D,
                        3 => tile_idx >= 0x0336 && tile_idx < 0x033F,
                        _ => false,
                    };

                    if is_selected && anim_timer >= 15 { tile_idx += 9; }

                    if let Some(tile) = vram.tile(tile_idx) {
                        let tx = x_pos + c * 8;
                        let mut ty = y_pos + r * 8;

                        // Сдвиг для стрелок
                        if tile_idx >= 0x0312 && tile_idx <= 0x0335 {
                            ty += 4;
                        }

                        // Если это иконка магии, сначала рисуем под ней синий фон (VDPTILE_SPACE = 0x20)
                        let is_magic_icon = tile_idx >= 0x140 && tile_idx <= 0x150;
                        if is_magic_icon {
                            if let Some(bg_tile) = vram.tile(0x20) {
                                for py in 0..8 {
                                    for px in 0..8 {
                                        let out_x = tx + px as i32;
                                        let out_y = ty + py as i32;
                                        if out_x >= 32 && out_x < 288 && out_y >= 0 && out_y < 224 {
                                            let bg_byte = bg_tile[(py * 4 + (px / 2)) as usize];
                                            let bg_ci = if px % 2 == 0 { (bg_byte >> 4) & 0x0F } else { bg_byte & 0x0F } as usize;
                                            if bg_ci != 0 {
                                                let color_idx = pal_offset + bg_ci;
                                                let color = if color_idx < palette.len() { palette[color_idx] } else { Color::RGB(0, 0, 0) };
                                                let off = out_y as usize * pitch + out_x as usize * 4;
                                                buf[off] = color.r; buf[off+1] = color.g; buf[off+2] = color.b; buf[off+3] = 255;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Теперь рисуем саму иконку/тайл
                        for py in 0..8 {
                            for px in 0..8 {
                                let src_col = if h_flip { 7 - px } else { px };
                                let src_row = if v_flip { 7 - py } else { py };
                                let byte = tile[(src_row * 4 + (src_col / 2)) as usize];
                                let ci = if src_col % 2 == 0 { ((byte >> 4) & 0x0F) as usize } else { (byte & 0x0F) as usize };
                                
                                if ci == 0 { continue; } 
                                
                                let color_idx = pal_offset + ci;
                                let color = if color_idx < palette.len() { palette[color_idx] } else { Color::RGB(255, 0, 255) };
                                
                                let out_x = tx + px as i32;
                                let out_y = ty + py as i32;
                                
                                if out_x >= 32 && out_x < 288 && out_y >= 0 && out_y < 224 {
                                    let off = out_y as usize * pitch + out_x as usize * 4;
                                    buf[off] = color.r; buf[off+1] = color.g; buf[off+2] = color.b; buf[off+3] = 255;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}