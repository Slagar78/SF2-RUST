// src/game/menu/magic_menu.rs
use crate::game::menu::window_engine::{
    WindowEngine, VDPTILE_PALETTE2, VDPTILE_PALETTE3, VDPTILE_PRIORITY
};
use crate::game::ally_parser::Combatant;
use crate::game::menu::portrait::PortraitData;
use crate::game::menu::layouts;
use crate::engine::base_tiles::{Vram, VRAM_PORTRAITS, VRAM_UI_ICONS};
use sdl3::pixels::Color;

pub const VISIBLE_MEMBERS: usize = 5;

pub struct MagicMenuState {
    pub window_slot_portrait: Option<usize>,
    pub window_slot_summary: Option<usize>,
    pub window_slot_list: Option<usize>,
    pub portrait_data: Option<PortraitData>,
    pub closing: bool,
    pub selected_member: usize,
    pub scroll_offset: usize,
}

impl MagicMenuState {
    pub fn new() -> Self {
        Self {
            window_slot_portrait: None,
            window_slot_summary: None,
            window_slot_list: None,
            portrait_data: None,
            closing: false,
            selected_member: 0,
            scroll_offset: 0,
        }
    }

    pub fn open(&mut self, window_engine: &mut WindowEngine, vram: &mut Vram, global_palette: &mut Vec<Color>, p_data: Option<PortraitData>) {
        self.closing = false;
        self.selected_member = 0;
        self.scroll_offset = 0;

        if let Some(p_data) = p_data {
            let _ = vram.write_bytes(VRAM_PORTRAITS, &p_data.tiles);
            for (i, c) in p_data.palette.iter().enumerate() {
                if 16 + i < global_palette.len() {
                    global_palette[16 + i] = *c;
                }
            }
            self.portrait_data = Some(p_data);
        }

        // ХАК ДЛЯ ОРАНЖЕВОГО ЦВЕТА: перекрашиваем Банк 3 (индекс 49) в оранжевый
        if global_palette.len() > 49 {
            global_palette[49] = Color::RGB(255, 116, 0); 
        }

        if let Ok(data) = std::fs::read("src/data/graphics/icons/spell/icon010.bin") {
            let offset = VRAM_UI_ICONS + 0x800;
            let _ = vram.write_bytes(offset, &data);
        }
        if let Ok(data) = std::fs::read("src/data/graphics/icons/spell/icon015.bin") {
            let offset = VRAM_UI_ICONS + 0x800 + 192;
            let _ = vram.write_bytes(offset, &data);
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
        if let Some(slot) = self.window_slot_summary  { window_engine.move_window(slot, 0x20F3, 4); }
        if let Some(slot) = self.window_slot_list     { window_engine.move_window(slot, 0x0220, 4); }
    }

    pub fn close(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot_portrait { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_summary  { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_list     { window_engine.delete_window(slot); }
        self.window_slot_portrait = None;
        self.window_slot_summary = None;
        self.window_slot_list = None;
        self.portrait_data = None;
    }

    pub fn clamp_scroll(&mut self, total: usize) {
        if total == 0 {
            self.scroll_offset = 0;
            return;
        }
        if self.selected_member >= total {
            self.selected_member = total - 1;
        }
        if self.selected_member < self.scroll_offset {
            self.scroll_offset = self.selected_member;
        }
        if self.selected_member >= self.scroll_offset + VISIBLE_MEMBERS {
            self.scroll_offset = self.selected_member + 1 - VISIBLE_MEMBERS;
        }
        let max_start = total.saturating_sub(VISIBLE_MEMBERS);
        if self.scroll_offset > max_start {
            self.scroll_offset = max_start;
        }
    }

    fn draw_icon(&self, window_engine: &mut WindowEngine, slot: usize, x: u8, y: u8, base_tile: u16) {
        let entry = &window_engine.entries[slot];
        let width = ((entry.width_height >> 8) & 0xFF) as usize;
        for r in 0..3 {
            for c in 0..2 {
                let idx = entry.layout_start_index + ((y as usize + r) * width + (x as usize + c));
                let tile = base_tile + (r * 2 + c) as u16;
                window_engine.window_tile_layouts[idx] = tile | VDPTILE_PALETTE3 | VDPTILE_PRIORITY;
            }
        }
    }

    pub fn draw(&self, window_engine: &mut WindowEngine, members: &[Combatant]) {
        if members.is_empty() { return; }
        let member = &members[self.selected_member.min(members.len() - 1)];

        // --- SUMMARY WINDOW (MAGIC) ---
        if let Some(slot) = self.window_slot_summary {
            // Чистим всё окно нормально (19 пробелов начиная с x=1)
            for i in 0..11 {
                window_engine.write_string(slot, 1, 1 + i, "                   "); 
            }

            window_engine.write_string(slot, 2, 1, &member.name);
            window_engine.write_string(slot, 9, 1, &member.class);
            window_engine.write_string(slot, 16, 1, &format!("L{}", member.level));

            window_engine.write_string(slot, 2, 3, "- MAGIC -");

            let egress_tile = ((VRAM_UI_ICONS + 0x800) / 32) as u16;
            let empty_tile  = ((VRAM_UI_ICONS + 0x800 + 192) / 32) as u16;

            if !member.spells.is_empty() {
                let (spell, lvl) = &member.spells[0];
                let spell_str: String = spell.chars().take(9).collect();
                window_engine.write_string(slot, 12, 3, &spell_str);
                window_engine.write_string(slot, 12, 4, &format!("Level {}", lvl));
                
                self.draw_icon(window_engine, slot, 5, 5, egress_tile);
            } else {
                self.draw_icon(window_engine, slot, 5, 5, empty_tile);
                
                // Пишем "Nothing" на месте заклинания (x=12, y=3)
                window_engine.write_string(slot, 12, 3, "Nothing");
                
                // Принудительно меняем палитровый банк на 3-й (0x6000) для этих букв
                let start_offset = window_engine.entries[slot].layout_start_index;
                let width = ((window_engine.entries[slot].width_height >> 8) & 0xFF) as usize;
                let text_idx_start = start_offset + (3 * width + 12);
                
                for i in 0..7 { // Длина слова "Nothing" = 7
                    let idx = text_idx_start + i;
                    if idx < window_engine.window_tile_layouts.len() {
                        // Срезаем старые банки и ставим 3-й (VDPTILE_PALETTE2 | VDPTILE_PALETTE3)
                        window_engine.window_tile_layouts[idx] = 
                            (window_engine.window_tile_layouts[idx] & !0x6000) | 0x6000;
                    }
                }
            }

            self.draw_icon(window_engine, slot, 2, 7, empty_tile);
            self.draw_icon(window_engine, slot, 8, 7, empty_tile);
            self.draw_icon(window_engine, slot, 5, 9, empty_tile);
        }

        // --- ROSTER LIST WINDOW ---
        if let Some(slot) = self.window_slot_list {
            // ИСПРАВЛЕНИЕ РАМКИ: останавливаем цикл на строке 11 (5-й персонаж)
            for i in 0..((VISIBLE_MEMBERS as u8 * 2) - 1) {
                window_engine.write_string(slot, 1, 3 + i, "                          ");
            }

            window_engine.write_string(slot, 2, 1, "NAME");
            window_engine.write_string(slot, 11, 1, "CLASS");
            window_engine.write_string(slot, 19, 1, "LEV");
            window_engine.write_string(slot, 24, 1, "EXP");

            for (i, m) in members.iter().skip(self.scroll_offset).take(VISIBLE_MEMBERS).enumerate() {
                let y = 3 + (i as u8 * 2);
                let member_idx = self.scroll_offset + i;

                let prefix = if member_idx == self.selected_member { ">" } else { " " };
                window_engine.write_string(slot, 1, y, prefix);

                let name_str = format!("{:<8}", m.name);
                window_engine.write_string(slot, 2, y, &name_str);
                window_engine.write_string(slot, 11, y, &m.class);
                window_engine.write_string(slot, 20, y, &m.level.to_string());
                window_engine.write_string(slot, 25, y, "0");
            }
        }
    }

    pub fn draw_highlight(&self, buf: &mut [u8], pitch: usize, window_engine: &WindowEngine, frame_counter: u32) {
        if let Some(slot) = self.window_slot_list {
            let blink_cycle = frame_counter % 30;
            if blink_cycle < 7 { return; }

            let entry = &window_engine.entries[slot];
            let win_x = (((entry.x_y >> 8) as u8 as i8) as i32 * 8) + 32;
            let win_y = ((entry.x_y & 0xFF) as u8 as i8) as i32 * 8;

            let visual_row = self.selected_member.saturating_sub(self.scroll_offset) as i32;
            let target_x = win_x + 8;
            let target_y = win_y + ((3 + visual_row * 2) * 8) - 4;

            let w = 72;
            let h = 16;
            let color = Color::RGB(255, 0, 0);

            for x in 1..w - 1 {
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

            for y in 1..h - 1 {
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