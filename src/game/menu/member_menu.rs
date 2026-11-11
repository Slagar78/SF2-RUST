// src/game/menu/member_menu.rs
use crate::game::menu::window_engine::{WindowEngine, VDPTILE_PALETTE2, VDPTILE_PRIORITY};
use crate::game::ally_parser::Combatant;
use crate::game::menu::portrait::{load_portrait, PortraitData};
use crate::game::menu::layouts;
use crate::engine::base_tiles::{Vram, VRAM_PORTRAITS};
use sdl3::pixels::Color;

pub struct MemberMenuState {
    pub window_slot_portrait: Option<usize>,
    pub window_slot_summary: Option<usize>,
    pub window_slot_list: Option<usize>,
    pub portrait_data: Option<PortraitData>,
    pub closing: bool,
    pub selected_member: usize,
    pub pending_status: bool,
}

impl MemberMenuState {
    pub fn new() -> Self {
        Self {
            window_slot_portrait: None,
            window_slot_summary: None,
            window_slot_list: None,
            portrait_data: None,
            closing: false,
            selected_member: 0,
            pending_status: false,
        }
    }

    pub fn open(&mut self, window_engine: &mut WindowEngine, vram: &mut Vram, global_palette: &mut Vec<Color>) {
        self.closing = false;
        self.pending_status = false;
        self.selected_member = 0;

        if let Some(p_data) = load_portrait(0) {
            let _ = vram.write_bytes(VRAM_PORTRAITS, &p_data.tiles);
            for (i, c) in p_data.palette.iter().enumerate() {
                if 16 + i < global_palette.len() {
                    global_palette[16 + i] = *c;
                }
            }
            self.portrait_data = Some(p_data);
        }

        self.window_slot_portrait = window_engine.create_window_with_layout(0xF8F6, 8, 10, &layouts::LAYOUT_PORTRAIT_WINDOW);
        if let Some(slot) = self.window_slot_portrait {
            window_engine.move_window(slot, 0x0201, 4);
            let start = window_engine.entries[slot].layout_start_index;
            let width = 8;
            
            let portrait_base_tile = (VRAM_PORTRAITS / 32) as u16;

            for r in 0..8 {
                for c in 0..6 {
                    let idx = start + ((r + 1) * width + (c + 1));
                    let tile_idx = portrait_base_tile + ((r * 8) as u16) + (c as u16);
                    window_engine.window_tile_layouts[idx] = tile_idx | VDPTILE_PALETTE2 | VDPTILE_PRIORITY;
                }
            }
        }

        self.window_slot_summary = window_engine.create_empty_window(0x20F3, 21, 13);
        if let Some(slot) = self.window_slot_summary {
            window_engine.move_window(slot, 0x0A01, 4);
        }

        self.window_slot_list = window_engine.create_empty_window(0x0220, 29, 13);
        if let Some(slot) = self.window_slot_list {
            window_engine.move_window(slot, 0x020E, 4);
        }
    }

    pub fn start_closing_animation(&mut self, window_engine: &mut WindowEngine) {
        if self.closing { return; }
        self.closing = true;
        
        if let Some(slot) = self.window_slot_portrait { window_engine.move_window(slot, 0xF8F6, 4); }
        if let Some(slot) = self.window_slot_summary { window_engine.move_window(slot, 0x20F3, 4); }
        if let Some(slot) = self.window_slot_list { window_engine.move_window(slot, 0x0220, 4); }
    }

    pub fn close(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot_portrait { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_summary { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_list { window_engine.delete_window(slot); }
        
        self.window_slot_portrait = None;
        self.window_slot_summary = None;
        self.window_slot_list = None;
        self.portrait_data = None;
    }

    pub fn draw(&self, window_engine: &mut WindowEngine, members: &[Combatant]) {
        if members.is_empty() { return; }
        let member = &members[self.selected_member.min(members.len() - 1)];

        if let Some(slot) = self.window_slot_summary {
            window_engine.write_string(slot, 2, 1, "                  ");
            window_engine.write_string(slot, 2, 3, "                  ");
            window_engine.write_string(slot, 2, 4, "                  ");
            window_engine.write_string(slot, 2, 5, "                  ");
            window_engine.write_string(slot, 2, 6, "                  ");

            window_engine.write_string(slot, 2, 1, &member.name);
            window_engine.write_string(slot, 9, 1, &member.class);
            window_engine.write_string(slot, 15, 1, &format!("L{}", member.level));
            
            window_engine.write_string(slot, 2, 3, "MAGIC");
            window_engine.write_string(slot, 13, 3, "ITEM");

            for (i, (spell, lvl)) in member.spells.iter().enumerate() {
                let y_base = 4 + (i as u8 * 2);
                window_engine.write_string(slot, 2, y_base, spell);
                window_engine.write_string(slot, 2, y_base + 1, &format!("Level {}", lvl));
            }

            let mut item_y = 4;
            for item in member.items.iter() {
                if item.len() > 10 {
                    let (part1, part2) = item.split_at(7); 
                    window_engine.write_string(slot, 13, item_y, part1.trim());
                    window_engine.write_string(slot, 13, item_y + 1, part2.trim());
                    item_y += 2;
                } else {
                    window_engine.write_string(slot, 13, item_y, item);
                    item_y += 1;
                }
            }
        }

        if let Some(slot) = self.window_slot_list {
            for i in 0..8 {
                window_engine.write_string(slot, 1, 3 + i, "                           ");
            }

            window_engine.write_string(slot, 2, 1, "NAME");
            window_engine.write_string(slot, 11, 1, "CLASS");
            window_engine.write_string(slot, 21, 1, "LEV");
            window_engine.write_string(slot, 25, 1, "EXP");
            
            for (i, m) in members.iter().take(4).enumerate() {
                let y = 3 + (i as u8 * 2); 
                
                // Добавили стрелку выделения
                let prefix = if i == self.selected_member { ">" } else { " " };
                window_engine.write_string(slot, 1, y, prefix);
                
                window_engine.write_string(slot, 2, y, &m.name);
                window_engine.write_string(slot, 11, y, &m.class);
                window_engine.write_string(slot, 21, y, &m.level.to_string());
                window_engine.write_string(slot, 25, y, "0");
            }
        }
    }

    // Добавили функцию для мигающей красной рамки
    pub fn draw_highlight(&self, buf: &mut [u8], pitch: usize, window_engine: &WindowEngine, frame_counter: u32) {
        if let Some(slot) = self.window_slot_list {
            let blink_cycle = frame_counter % 30;
            if blink_cycle < 7 {
                return;
            }

            let entry = &window_engine.entries[slot];
            let win_x = (((entry.x_y >> 8) as u8 as i8) as i32 * 8) + 32;
            let win_y = ((entry.x_y & 0xFF) as u8 as i8) as i32 * 8;

            let target_x = win_x + (1 * 8); 
            let target_y = win_y + ((3 + self.selected_member as i32 * 2) * 8) - 4;

            let w = 72; 
            let h = 16;
            let color = Color::RGB(255, 0, 0);

            for x in 0..w {
                let px = target_x + x;
                let py_top = target_y;
                let py_bot = target_y + h - 1;

                if px >= 32 && px < 288 && py_top >= 0 && py_top < 224 {
                    let off = py_top as usize * pitch + px as usize * 4;
                    buf[off] = color.r; buf[off+1] = color.g; buf[off+2] = color.b; buf[off+3] = 255;
                }
                if px >= 32 && px < 288 && py_bot >= 0 && py_bot < 224 {
                    let off = py_bot as usize * pitch + px as usize * 4;
                    buf[off] = color.r; buf[off+1] = color.g; buf[off+2] = color.b; buf[off+3] = 255;
                }
            }

            for y in 0..h {
                let py = target_y + y;
                let px_left = target_x;
                let px_right = target_x + w - 1;

                if px_left >= 32 && px_left < 288 && py >= 0 && py < 224 {
                    let off = py as usize * pitch + px_left as usize * 4;
                    buf[off] = color.r; buf[off+1] = color.g; buf[off+2] = color.b; buf[off+3] = 255;
                }
                if px_right >= 32 && px_right < 288 && py >= 0 && py < 224 {
                    let off = py as usize * pitch + px_right as usize * 4;
                    buf[off] = color.r; buf[off+1] = color.g; buf[off+2] = color.b; buf[off+3] = 255;
                }
            }
        }
    }
}