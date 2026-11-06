// src/game/menu/member_status_menu.rs
use crate::game::menu::window_engine::{WindowEngine, VDPTILE_PALETTE2, VDPTILE_PALETTE3, VDPTILE_PRIORITY};
use crate::game::ally_parser::Combatant;
use crate::game::menu::portrait::{load_portrait, PortraitData};
use crate::game::menu::layouts;
use crate::engine::base_tiles::Vram;
use crate::engine::decompressor;
use sdl3::pixels::Color;

/// Member status screen ("Status" in the Diamond Menu).
///
/// Reimplements SF2's per-character status layout: a portrait with a blinking
/// eye animation, a large status window (class/name/level, HP/MP/EXP, combat
/// stats, magic and item lists), a KILLS/DEFEAT panel, and a GOLD panel.
/// Windows slide in from off-screen using the original game's coordinates
/// and animation speed.
pub struct MemberStatusState {
    pub window_slot_portrait: Option<usize>,
    pub window_slot_status: Option<usize>,
    pub window_slot_kd: Option<usize>,
    pub window_slot_gold: Option<usize>,
    pub portrait_data: Option<PortraitData>,
    /// True while windows are animating back off-screen. The caller waits on
    /// this flag before deleting windows and leaving the state.
    pub closing: bool,
    /// Ticks until the next eye-blink toggle.
    pub blink_timer: u16,
    pub eyes_closed: bool,
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
        }
    }

    /// Open the status screen: load portrait and icon tiles into VRAM, load the
    /// map sprite for the overlay, then create and animate the four windows.
    ///
    /// Every window is created off-screen and slides toward its target with
    /// `move_window(.., 4)`, matching SF2's opening animation cadence.
    pub fn open(&mut self, window_engine: &mut WindowEngine, vram: &mut Vram, global_palette: &mut Vec<Color>) {
        self.closing = false;
        self.blink_timer = 120;
        self.eyes_closed = false;

        // Portrait tiles → VRAM 0xF800; its palette is copied into global
        // palette bank 1 (indices 16..31) to match VDPTILE_PALETTE2 below.
        if let Some(p_data) = load_portrait(0) {
            let _ = vram.write_bytes(0xF800, &p_data.tiles);
            for (i, c) in p_data.palette.iter().enumerate() {
                if 16 + i < global_palette.len() {
                    global_palette[16 + i] = *c;
                }
            }
            self.portrait_data = Some(p_data);
        }

        // Small icons used in the magic/item lists. Sword and herb are item
        // icons; the spell icon sits in a separate VRAM region.
        if let Ok(sword_data) = std::fs::read("src/data/graphics/icons/item/icon002.bin") {
            let _ = vram.write_bytes(0x8800, &sword_data);
        }
        if let Ok(herb_data) = std::fs::read("src/data/graphics/icons/item/icon026.bin") {
            let _ = vram.write_bytes(0x88C0, &herb_data);
        }
        if let Ok(spell_data) = std::fs::read("src/data/graphics/icons/spell/icon000.bin") {
            let _ = vram.write_bytes(0x8980, &spell_data);
        }

        // 1. Portrait window: spawn off-screen left (0xF8F6), slide to 0x0201.
        //    The inner tilemap is then overwritten to point at the portrait
        //    tiles loaded above (0x7C0 + row*8 + col), using PALETTE2 + PRIORITY
        //    so the portrait draws above the window frame. The +1 row/col
        //    offset skips the border tiles.
        self.window_slot_portrait = window_engine.create_window_with_layout(0xF8F6, 8, 10, &layouts::LAYOUT_PORTRAIT_WINDOW);
        if let Some(slot) = self.window_slot_portrait {
            window_engine.move_window(slot, 0x0201, 4);
            let start = window_engine.entries[slot].layout_start_index;
            let width = 8;
            for r in 0..8 {
                for c in 0..6 {
                    let idx = start + ((r + 1) * width + (c + 1));
                    let tile_idx = 0x7C0 + (r * 8) + c;
                    window_engine.window_tile_layouts[idx] = (tile_idx as u16) | VDPTILE_PALETTE2 | VDPTILE_PRIORITY;
                }
            }
        }

        // 2. Status window: spawn off-screen right (0x2015), slide left to 0x0A01.
        self.window_slot_status = window_engine.create_empty_window(0x2015, 21, 26);
        if let Some(slot) = self.window_slot_status {
            window_engine.move_window(slot, 0x0A01, 4);
        }

        // 3. KILLS/DEFEAT panel: spawn off-screen left (0xF80B), slide to 0x020B.
        self.window_slot_kd = window_engine.create_empty_window(0xF80B, 8, 12);
        if let Some(slot) = self.window_slot_kd {
            window_engine.move_window(slot, 0x020B, 4);
        }

        // 4. GOLD panel: spawn off-screen left (0xF817), slide to 0x0217.
        self.window_slot_gold = window_engine.create_empty_window(0xF817, 8, 4);
        if let Some(slot) = self.window_slot_gold {
            window_engine.move_window(slot, 0x0217, 4);
        }

        // Load the downward-facing map sprite used for the 3x3 overlay portrait
        // next to the KILLS panel. Tiles are stored column-major in the source
        // sheet and must be transposed to row-major before upload.
        if let Ok(raw) = std::fs::read("src/data/graphics/mapsprites/mapsprite000-2.bin") {
            let d = decompressor::decompress_basic(&raw);
            if d.len() >= 288 {
                let mut reordered = vec![0u8; 288];
                for ty in 0..3 {
                    for tx in 0..3 {
                        let src = (tx * 3 + ty) * 32;
                        let dst = (ty * 3 + tx) * 32;
                        reordered[dst..dst + 32].copy_from_slice(&d[src..src + 32]);
                    }
                }
                let _ = vram.write_bytes(0x7000, &reordered);
            }
        }
    }

    /// Begin the exit animation: slide every window (including the portrait)
    /// back to its original off-screen corner. Windows are deleted only after
    /// all animations finish; see `close`.
    pub fn start_closing_animation(&mut self, window_engine: &mut WindowEngine) {
        if self.closing { return; }
        self.closing = true;
        
        if let Some(slot) = self.window_slot_portrait { window_engine.move_window(slot, 0xF8F6, 4); }
        if let Some(slot) = self.window_slot_status { window_engine.move_window(slot, 0x2015, 4); }
        if let Some(slot) = self.window_slot_kd { window_engine.move_window(slot, 0xF80B, 4); }
        if let Some(slot) = self.window_slot_gold { window_engine.move_window(slot, 0xF817, 4); }
    }

    /// Tear down all windows and reset the state. Must be called only after
    /// the closing animation has fully played out.
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

    /// Advance the blink timer. When it hits zero, toggle the portrait's eye
    /// tiles. The next interval is short (10 ticks) while the eyes are closed,
    /// then long (120 ticks) while open — producing a natural blink.
    pub fn tick_blink(&mut self, window_engine: &mut WindowEngine) {
        if self.blink_timer > 0 {
            self.blink_timer -= 1;
            if self.blink_timer == 0 {
                self.toggle_eyes(window_engine);
            }
        }
    }

    /// Swap the portrait's eye tiles between the open and closed variants.
    ///
    /// The portrait data carries per-eye (x, y) and (alt_x, alt_y) positions
    /// in the 6x8 inner tilemap; the `+1` offset converts them to the window's
    /// border-inclusive tilemap coordinates.
    fn toggle_eyes(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot_portrait {
            if let Some(p_data) = &self.portrait_data {
                let start = window_engine.entries[slot].layout_start_index;
                let width = 8;
                self.eyes_closed = !self.eyes_closed;
                self.blink_timer = if self.eyes_closed { 10 } else { 120 };

                for eye in p_data.eyes.iter() {
                    let idx = start + ((eye.y as usize + 1) * width + (eye.x as usize + 1));
                    let (tx, ty) = if self.eyes_closed { (eye.alt_x, eye.alt_y) } else { (eye.x, eye.y) };
                    let tile_idx = 0x7C0 + (ty as usize * 8) + tx as usize;
                    window_engine.window_tile_layouts[idx] = (tile_idx as u16) | VDPTILE_PALETTE2 | VDPTILE_PRIORITY;
                }
            }
        }
    }

    /// Place a 3x2-tile icon in the window's tilemap at tile position (x, y).
    /// Icons use PALETTE3 | PRIORITY so they draw above window content.
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

    /// Render the status screen contents.
    ///
    /// NOTE: several stat lines (HP/MP/EXP, ATT/DEF/AGI/MOV, KILLS/DEFEAT,
    /// GOLD) are hardcoded placeholders because those fields do not yet exist
    /// on `Combatant`. They render at the correct screen positions but will
    /// show static values until the stat model is wired up.
    pub fn draw(&self, window_engine: &mut WindowEngine, member: &Combatant) {
        if let Some(slot) = self.window_slot_status {
            // Clear the entire body of the window before redrawing.
            for y in 1..25 {
                window_engine.write_string(slot, 1, y, "                   ");
            }
            window_engine.write_string(slot, 2, 1, &member.class);
            window_engine.write_string(slot, 8, 1, &member.name);
            
            // Left column: level, HP, MP, EXP.
            window_engine.write_string(slot, 2, 3, &format!("LV      {}", member.level));
            window_engine.write_string(slot, 2, 5, "HP 12/12"); 
            window_engine.write_string(slot, 2, 7, "MP  8/ 8"); 
            window_engine.write_string(slot, 2, 9, "EXP    0"); 
            
            // Right column: combat stats.
            window_engine.write_string(slot, 12, 3, "ATT   9");  
            window_engine.write_string(slot, 12, 5, "DEF   4");  
            window_engine.write_string(slot, 12, 7, "AGI   4");  
            window_engine.write_string(slot, 12, 9, "MOV   6");  

            // Two-column list headers for spells (left) and items (right).
            window_engine.write_string(slot, 1, 13, "MAGIC");
            window_engine.write_string(slot, 10, 13, "ITEM"); 

            // Spell list: each entry is 3 rows tall (icon area + name + level).
            // Only the first entry draws the spell icon (hardcoded tile 0x44C);
            // the level line is a placeholder until the class spell table is wired.
            let mut spell_y = 14;
            for (i, (spell, _lvl)) in member.spells.iter().enumerate() {
                if i == 0 { self.draw_icon(window_engine, slot, 1, spell_y as u8, 0x44C); }
                window_engine.write_string(slot, 3, spell_y, spell); 
                window_engine.write_string(slot, 3, spell_y + 1, "Level 1");
                spell_y += 3;
            }

            // Item list: only the first two items get icons (sword / herb).
            // Long item names are wrapped at 7 characters across two lines.
            let mut item_y = 14;
            for (i, item) in member.items.iter().enumerate() {
                if i == 0 { self.draw_icon(window_engine, slot, 10, item_y as u8, 0x440); }
                if i == 1 { self.draw_icon(window_engine, slot, 10, item_y as u8, 0x446); }

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

        // KILLS / DEFEAT panel (placeholder values until the stat model exists).
        if let Some(slot) = self.window_slot_kd {
            window_engine.write_string(slot, 1, 6, "KILLS");
            window_engine.write_string(slot, 6, 7, "0"); 
            window_engine.write_string(slot, 1, 9, "DEFEAT");
            window_engine.write_string(slot, 6, 10, "0");
        }

        // GOLD panel (placeholder value).
        if let Some(slot) = self.window_slot_gold {
            window_engine.write_string(slot, 1, 1, "GOLD");
            window_engine.write_string(slot, 5, 2, "60");
        }
    }

    /// Draw the 3x3-tile character sprite next to the KILLS panel, as a
    /// pixel-level overlay on top of the window frame. Needed because window
    /// tilemaps cannot reproduce the multi-layer overlap of a moving map sprite.
    ///
    /// The window's stored (x, y) are signed 8-bit values; `+32` shifts the
    /// leftmost visible column past the HUD strip, and `* 8` converts tiles to
    /// pixels. The sprite offset is then applied in pixel space.
    pub fn draw_sprite_overlay(&self, buf: &mut [u8], pitch: usize, vram: &Vram, window_engine: &WindowEngine, palette: &[Color]) {
        if let Some(slot) = self.window_slot_kd {
            let entry = &window_engine.entries[slot];
            // Signed 8-bit interpretation: windows can legitimately have
            // negative coordinates while sliding in from off-screen.
            let win_x = (((entry.x_y >> 8) as u8 as i8) as i32 * 8) + 32;
            let win_y = ((entry.x_y & 0xFF) as u8 as i8) as i32 * 8;

            // 2 tiles down/right from the window origin places the sprite
            // beside the KILLS/DEFEAT text.
            let spr_x = win_x + 2 * 8;
            let spr_y = win_y + 2 * 8; 

            // VRAM tile base of the uploaded map sprite (see `open`).
            let tile_base = 0x380;

            for r in 0..3 {
                for c in 0..3 {
                    let tile_idx = tile_base + (r * 3) + c;
                    if let Some(tile) = vram.tile(tile_idx) {
                        for py in 0..8 {
                            for px in 0..8 {
                                let byte = tile[py * 4 + (px / 2)];
                                let ci = if px % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;
                                
                                // Palette index 0 is transparent.
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