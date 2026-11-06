// src/game/menu/member_menu.rs
use crate::game::menu::window_engine::{WindowEngine, VDPTILE_PALETTE2, VDPTILE_PRIORITY};
use crate::game::ally_parser::Combatant;
use crate::game::menu::portrait::{load_portrait, PortraitData};
use crate::game::menu::layouts;
use crate::engine::base_tiles::Vram;
use sdl3::pixels::Color;

/// Party-list screen ("Members" in the Diamond Menu).
///
/// Reimplements SF2's member roster layout: three windows (portrait, summary,
/// roster list) that slide in from off-screen, matching the original game's
/// window coordinates, animation speeds, and VRAM placement.
pub struct MemberMenuState {
    pub window_slot_portrait: Option<usize>,
    pub window_slot_summary: Option<usize>,
    pub window_slot_list: Option<usize>,
    pub portrait_data: Option<PortraitData>,
    /// True while windows are sliding back off-screen. The caller polls this
    /// to know when it is safe to delete windows and change state.
    pub closing: bool,
    pub selected_member: usize,
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
        }
    }

    /// Open the member menu: load the selected portrait into VRAM, then create
    /// and animate the three windows into place.
    ///
    /// Each window is created off-screen and slid toward its target using
    /// `move_window(.., speed)`. Speeds and target coordinates are taken from
    /// the original SF2 data so the animation cadence matches the source game.
    pub fn open(&mut self, window_engine: &mut WindowEngine, vram: &mut Vram, global_palette: &mut Vec<Color>) {
        self.closing = false;
        self.selected_member = 0;

        // Load portrait tiles into VRAM at 0xF800 and copy its 16-color palette
        // into global palette bank 1 (indices 16..31), which is what PALETTE2
        // indexes. Portrait tiles render with PALETTE2 | PRIORITY flags below.
        if let Some(p_data) = load_portrait(0) {
            let _ = vram.write_bytes(0xF800, &p_data.tiles);
            for (i, c) in p_data.palette.iter().enumerate() {
                if 16 + i < global_palette.len() {
                    global_palette[16 + i] = *c;
                }
            }
            self.portrait_data = Some(p_data);
        }

        // 1. Portrait window: spawn at off-screen corner (0xF8F6) and slide to
        //    0x0201. After the window exists, its inner tilemap is overwritten
        //    to point at the portrait tiles loaded above (0x7C0 + row*8 + col),
        //    using PALETTE2 + PRIORITY so they draw above the window frame.
        self.window_slot_portrait = window_engine.create_window_with_layout(0xF8F6, 8, 10, &layouts::LAYOUT_PORTRAIT_WINDOW);
        if let Some(slot) = self.window_slot_portrait {
            window_engine.move_window(slot, 0x0201, 4);
            let start = window_engine.entries[slot].layout_start_index;
            let width = 8;
            for r in 0..8 {
                for c in 0..6 {
                    // +1 on row/col skips the window's border tiles, so the
                    // portrait fills the inner 6x8 area only.
                    let idx = start + ((r + 1) * width + (c + 1));
                    let tile_idx = 0x7C0 + (r * 8) + c;
                    window_engine.window_tile_layouts[idx] = (tile_idx as u16) | VDPTILE_PALETTE2 | VDPTILE_PRIORITY;
                }
            }
        }

        // 2. Summary window: spawn off-screen right (0x20F3), slide to 0x0A01.
        self.window_slot_summary = window_engine.create_empty_window(0x20F3, 21, 13);
        if let Some(slot) = self.window_slot_summary {
            window_engine.move_window(slot, 0x0A01, 4);
        }

        // 3. Roster list: spawn off-screen bottom (0x0220), slide to 0x020E.
        self.window_slot_list = window_engine.create_empty_window(0x0220, 29, 13);
        if let Some(slot) = self.window_slot_list {
            window_engine.move_window(slot, 0x020E, 4);
        }
    }

    /// Begin the exit animation: slide every window back to the corner it
    /// originated from. The state stays `closing = true` until the caller has
    /// confirmed all window animations have finished; only then should `close`
    /// be invoked to actually delete the windows.
    pub fn start_closing_animation(&mut self, window_engine: &mut WindowEngine) {
        if self.closing { return; }
        self.closing = true;
        
        if let Some(slot) = self.window_slot_portrait { window_engine.move_window(slot, 0xF8F6, 4); }
        if let Some(slot) = self.window_slot_summary { window_engine.move_window(slot, 0x20F3, 4); }
        if let Some(slot) = self.window_slot_list { window_engine.move_window(slot, 0x0220, 4); }
    }

    /// Tear down all windows and drop the loaded portrait. Should be called
    /// only after `start_closing_animation` has fully played out.
    pub fn close(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot_portrait { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_summary { window_engine.delete_window(slot); }
        if let Some(slot) = self.window_slot_list { window_engine.delete_window(slot); }
        
        self.window_slot_portrait = None;
        self.window_slot_summary = None;
        self.window_slot_list = None;
        self.portrait_data = None;
    }

    /// Render the selected member's summary and the roster list.
    ///
    /// The window engine has no "clear" primitive, so fixed-width spaces are
    /// written over each dynamic line before drawing new content. This mirrors
    /// SF2's approach of clearing tilemap rows in place.
    pub fn draw(&self, window_engine: &mut WindowEngine, members: &[Combatant]) {
        if members.is_empty() { return; }
        // Defensive clamp: `selected_member` may point past the last member if
        // the roster was reloaded while the menu was open.
        let member = &members[self.selected_member.min(members.len() - 1)];

        // --- SUMMARY WINDOW ---
        if let Some(slot) = self.window_slot_summary {
            // Clear previous dynamic rows (name/class/level line + 4 body rows).
            window_engine.write_string(slot, 2, 1, "                  ");
            window_engine.write_string(slot, 2, 3, "                  ");
            window_engine.write_string(slot, 2, 4, "                  ");
            window_engine.write_string(slot, 2, 5, "                  ");
            window_engine.write_string(slot, 2, 6, "                  ");

            // Header line: name | class | level.
            window_engine.write_string(slot, 2, 1, &member.name);
            window_engine.write_string(slot, 9, 1, &member.class);
            window_engine.write_string(slot, 15, 1, &format!("L{}", member.level));
            
            // Two-column body: spells on the left, items on the right.
            window_engine.write_string(slot, 2, 3, "MAGIC");
            window_engine.write_string(slot, 13, 3, "ITEM");

            // Spells take two lines each (name + required level), so the
            // vertical stride is 2 per entry.
            for (i, (spell, lvl)) in member.spells.iter().enumerate() {
                let y_base = 4 + (i as u8 * 2);
                window_engine.write_string(slot, 2, y_base, spell);
                window_engine.write_string(slot, 2, y_base + 1, &format!("Level {}", lvl));
            }

            // Items: long names are wrapped across two lines (hard split at 7
            // characters, matching the original UI's character budget).
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

        // --- ROSTER LIST WINDOW ---
        if let Some(slot) = self.window_slot_list {
            // Clear the 8 body rows before redrawing. Rows are spaced two
            // apart (3,5,7,9,..), but we clear every row in the body range so
            // stale single-row entries cannot linger.
            for i in 0..8 {
                window_engine.write_string(slot, 1, 3 + i, "                           ");
            }

            // Column headers.
            window_engine.write_string(slot, 1, 1, "NAME");
            window_engine.write_string(slot, 11, 1, "CLASS");
            window_engine.write_string(slot, 21, 1, "LEV");
            window_engine.write_string(slot, 25, 1, "EXP");
            
            // The visible roster fits four members at two rows each.
            // EXP is currently a placeholder — exp is not yet tracked on
            // Combatant, so a literal "0" is written instead.
            for (i, m) in members.iter().take(4).enumerate() {
                let y = 3 + (i as u8 * 2); 
                window_engine.write_string(slot, 1, y, &m.name);
                window_engine.write_string(slot, 11, y, &m.class);
                window_engine.write_string(slot, 21, y, &m.level.to_string());
                window_engine.write_string(slot, 25, y, "0");
            }
        }
    }
}