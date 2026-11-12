// src/game/menu/member_status_menu.rs
use crate::game::menu::window_engine::{WindowEngine, VDPTILE_PALETTE2, VDPTILE_PALETTE3, VDPTILE_PRIORITY};
use crate::game::ally_parser::Combatant;
use crate::game::menu::portrait::{load_portrait, PortraitData};
use crate::game::menu::layouts;
use crate::engine::base_tiles::{Vram, VRAM_UI_ICONS, VRAM_PORTRAITS, VRAM_MAP_SPRITES};
use crate::engine::decompressor;
use sdl3::pixels::Color;

pub struct MemberStatusState {
    pub window_slot_portrait: Option<usize>,
    pub window_slot_status: Option<usize>,
    pub window_slot_kd: Option<usize>,
    pub window_slot_gold: Option<usize>,
    pub portrait_data: Option<PortraitData>,
    pub closing: bool,
    pub blink_timer: u16,
    pub eyes_closed: bool,
    pub anim_timer: u16,
    pub anim_frame: u16,
    pub member_index: usize, // НОВОЕ
}

impl MemberStatusState {
    pub fn new() -> Self {
        Self {
            window_slot_portrait: None,
            window_slot_status: None,
            window_slot_kd: None,
            window_slot_gold: None,
            portrait_data: None,
            closing: false,
            blink_timer: 120,
            eyes_closed: false,
            anim_timer: 20,
            anim_frame: 0,
            member_index: 0,
        }
    }

    pub fn open(
        &mut self, 
        window_engine: &mut WindowEngine, 
        vram: &mut Vram, 
        global_palette: &mut Vec<Color>,
        member_index: usize,
        map_sprite_id: u8,
    ) {
        self.member_index = member_index;
        self.closing = false;
        self.blink_timer = 120;
        self.eyes_closed = false;

        // 1. Портрет по индексу
        if let Some(p_data) = load_portrait(member_index) {
            let _ = vram.write_bytes(VRAM_PORTRAITS, &p_data.tiles);
            for (i, c) in p_data.palette.iter().enumerate() {
                if 16 + i < global_palette.len() {
                    global_palette[16 + i] = *c;
                }
            }
            self.portrait_data = Some(p_data);
        }

        // 2. Фикс палитры для иконок
        global_palette[48] = sdl3::pixels::Color::RGB(0, 0, 0);
        global_palette[49] = sdl3::pixels::Color::RGB(255, 255, 255);
        global_palette[50] = sdl3::pixels::Color::RGB(139, 69, 19);
        global_palette[51] = sdl3::pixels::Color::RGB(0, 255, 0);
        global_palette[52] = sdl3::pixels::Color::RGB(255, 255, 0);
        global_palette[53] = sdl3::pixels::Color::RGB(128, 128, 128);

        // 3. Иконки
        if let Ok(data) = std::fs::read("src/data/graphics/icons/item/icon071.bin") {
            let offset = VRAM_UI_ICONS + (71 * 192);
            let _ = vram.write_bytes(offset, &data);
        }
        if let Ok(data) = std::fs::read("src/data/graphics/icons/item/icon000.bin") {
            let offset = VRAM_UI_ICONS + (0 * 192);
            let _ = vram.write_bytes(offset, &data);
        }
        if let Ok(spell_data) = std::fs::read("src/data/graphics/icons/spell/icon000.bin") {
            let _ = vram.write_bytes(VRAM_UI_ICONS + 0x800, &spell_data);
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

        self.window_slot_status = window_engine.create_empty_window(0x2015, 21, 26);
        if let Some(slot) = self.window_slot_status {
            window_engine.move_window(slot, 0x0A01, 4);
        }

        self.window_slot_kd = window_engine.create_empty_window(0xF80B, 8, 12);
        if let Some(slot) = self.window_slot_kd {
            window_engine.move_window(slot, 0x020B, 4);
        }

        self.window_slot_gold = window_engine.create_empty_window(0xF817, 8, 4);
        if let Some(slot) = self.window_slot_gold {
            window_engine.move_window(slot, 0x0217, 4);
        }

        // 4. Маленький спрайт для окна KILLS — по правильному ID
        let path = format!("src/data/graphics/mapsprites/mapsprite{:03}-2.bin", map_sprite_id);
        if let Ok(raw) = std::fs::read(&path) {
            let d = decompressor::decompress_basic(&raw);
            if d.len() >= 576 {
                let mut reordered = vec![0u8; 576];
                for f in 0..2 {
                    let frame_src = f * 288;
                    let frame_dst = f * 288;
                    for ty in 0..3 {
                        for tx in 0..3 {
                            let src = frame_src + (tx * 3 + ty) * 32;
                            let dst = frame_dst + (ty * 3 + tx) * 32;
                            reordered[dst..dst + 32].copy_from_slice(&d[src..src + 32]);
                        }
                    }
                }
                let _ = vram.write_bytes(VRAM_MAP_SPRITES, &reordered);
            }
        }
    }

    pub fn start_closing_animation(&mut self, window_engine: &mut WindowEngine) {
        if self.closing { return; }
        self.closing = true;
        
        if let Some(slot) = self.window_slot_portrait { window_engine.move_window(slot, 0xF8F6, 4); }
        if let Some(slot) = self.window_slot_status { window_engine.move_window(slot, 0x2015, 4); }
        if let Some(slot) = self.window_slot_kd { window_engine.move_window(slot, 0xF80B, 4); }
        if let Some(slot) = self.window_slot_gold { window_engine.move_window(slot, 0xF817, 4); }
    }

    pub fn close(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot_portrait { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_status { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_kd { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_gold { window_engine.delete_window(slot); }
        self.window_slot_portrait = None;
        self.window_slot_status = None;
        self.window_slot_kd = None;
        self.window_slot_gold = None;
        self.portrait_data = None;
        self.closing = false;
    }

    pub fn tick_blink(&mut self, window_engine: &mut WindowEngine) {
        if self.blink_timer > 0 {
            self.blink_timer -= 1;
            if self.blink_timer == 0 {
                self.toggle_eyes(window_engine);
            }
        }

        if self.anim_timer > 0 {
            self.anim_timer -= 1;
        } else {
            self.anim_timer = 20;
            self.anim_frame = if self.anim_frame == 0 { 1 } else { 0 };
        }
    }

    fn toggle_eyes(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot_portrait {
            if let Some(p_data) = &self.portrait_data {
                let start = window_engine.entries[slot].layout_start_index;
                let width = 8;
                self.eyes_closed = !self.eyes_closed;
                self.blink_timer = if self.eyes_closed { 10 } else { 120 };

                let portrait_base_tile = (VRAM_PORTRAITS / 32) as u16;

                for eye in p_data.eyes.iter() {
                    let idx = start + ((eye.y as usize + 1) * width + (eye.x as usize + 1));
                    let (tx, ty) = if self.eyes_closed { (eye.alt_x, eye.alt_y) } else { (eye.x, eye.y) };
                    let tile_idx = portrait_base_tile + (((ty as usize * 8) + tx as usize) as u16);
                    window_engine.window_tile_layouts[idx] = tile_idx | VDPTILE_PALETTE2 | VDPTILE_PRIORITY;
                }
            }
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

    pub fn draw(&self, window_engine: &mut WindowEngine, member: &Combatant) {
        if let Some(slot) = self.window_slot_status {
            for y in 1..25 {
                window_engine.write_string(slot, 1, y, "                   ");
            }
            window_engine.write_string(slot, 2, 1, &member.class);
            window_engine.write_string(slot, 8, 1, &member.name);
            
            window_engine.write_string(slot, 2, 3, &format!("LV      {}", member.level));
            window_engine.write_string(slot, 2, 5, &format!("HP {:>2}/{:>2}", member.hp, member.max_hp)); 
            window_engine.write_string(slot, 2, 7, &format!("MP {:>2}/{:>2}", member.mp, member.max_mp)); 
            window_engine.write_string(slot, 2, 9, "EXP    0"); 
            
            window_engine.write_string(slot, 12, 3, &format!("ATT   {}", member.attack));  
            window_engine.write_string(slot, 12, 5, &format!("DEF   {}", member.defense));  
            window_engine.write_string(slot, 12, 7, &format!("AGI   {}", member.agility));  
            window_engine.write_string(slot, 12, 9, &format!("MOV   {}", member.movement));  

            window_engine.write_string(slot, 1, 13, "MAGIC");
            window_engine.write_string(slot, 10, 13, "ITEM"); 

            let icon_base_tile = (VRAM_UI_ICONS / 32) as u16;

            let mut spell_y = 14;
            for (i, (spell, _lvl)) in member.spells.iter().enumerate() {
                if i == 0 { 
                    self.draw_icon(window_engine, slot, 1, spell_y as u8, icon_base_tile + 0x40); 
                }
                window_engine.write_string(slot, 3, spell_y, spell); 
                window_engine.write_string(slot, 3, spell_y + 1, "Level 1");
                spell_y += 3;
            }

            let mut item_y = 14;
            for item in member.items.iter() {
                let item_id = match item.as_str() {
                    "Wooden Sword" => 71,
                    "Medical Herb" => 0,
                    _ => 255,
                };

                if item_id != 255 {
                    let tile_base = icon_base_tile + (item_id * 6) as u16;
                    self.draw_icon(window_engine, slot, 10, item_y as u8, tile_base);
                }

                if item.len() > 10 {
                    let (part1, part2) = item.split_at(7); 
                    window_engine.write_string(slot, 12, item_y, part1.trim());
                    window_engine.write_string(slot, 12, item_y + 1, part2.trim());
                } else {
                    window_engine.write_string(slot, 12, item_y, item);
                }
                item_y += 3; 
            }
       }

        if let Some(slot) = self.window_slot_kd {
            window_engine.write_string(slot, 1, 6, "KILLS");
            window_engine.write_string(slot, 6, 7, "0"); 
            window_engine.write_string(slot, 1, 9, "DEFEAT");
            window_engine.write_string(slot, 6, 10, "0");
        }

        if let Some(slot) = self.window_slot_gold {
            window_engine.write_string(slot, 1, 1, "GOLD");
            window_engine.write_string(slot, 5, 2, "60");
        }
    }

    pub fn draw_sprite_overlay(&self, buf: &mut [u8], pitch: usize, vram: &Vram, window_engine: &WindowEngine, palette: &[Color]) {
        if let Some(slot) = self.window_slot_kd {
            let entry = &window_engine.entries[slot];
            let win_x = (((entry.x_y >> 8) as u8 as i8) as i32 * 8) + 32;
            let win_y = ((entry.x_y & 0xFF) as u8 as i8) as i32 * 8;

            let spr_x = win_x + 2 * 8 + 4;
            let spr_y = win_y + 2 * 8; 

            let tile_base = (VRAM_MAP_SPRITES / 32) as u16;
            let frame_offset = self.anim_frame * 9;

            for r in 0..3 {
                for c in 0..3 {
                    let tile_idx = tile_base + (r * 3) + c + frame_offset;
                    if let Some(tile) = vram.tile(tile_idx as usize) {
                        for py in 0..8 {
                            for px in 0..8 {
                                let byte = tile[py * 4 + (px / 2)];
                                let ci = if px % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;
                                
                                // ВОТ ОНА, СПАСИТЕЛЬНАЯ СТРОКА:
                                if ci == 0 { continue; }
                                
                                let c_color = palette[ci];
                                let out_x = spr_x + (c as i32) * 8 + px as i32;
                                let out_y = spr_y + (r as i32) * 8 + py as i32;
                                
                                if out_x >= 0 && out_x < 320 && out_y >= 0 && out_y < 224 {
                                    let off = out_y as usize * pitch + out_x as usize * 4;
                                    buf[off] = c_color.r;
                                    buf[off+1] = c_color.g;
                                    buf[off+2] = c_color.b;
                                    buf[off+3] = 255;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}