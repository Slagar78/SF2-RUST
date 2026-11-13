// src/game/script/dialog.rs
use crate::game::menu::window_engine::{WindowEngine, VDPTILE_PALETTE2, VDPTILE_PRIORITY};
use crate::game::menu::layouts;
use crate::game::menu::portrait::{load_portrait, PortraitData};
use crate::engine::base_tiles::{Vram, VRAM_PORTRAITS};
use sdl3::pixels::Color;

pub struct DialogState {
    pub text: String,
    pub visible_chars: usize,
    pub timer: u8,
    pub is_active: bool,
    pub window_slot: Option<usize>,
    
    // Данные для портрета
    pub portrait_slot: Option<usize>,
    pub portrait_data: Option<PortraitData>,
    pub blink_timer: u16,
    pub eyes_closed: bool,
    pub mouth_timer: u16,
    pub mouth_open: bool,
}

impl DialogState {
    pub fn new() -> Self {
        Self { 
            text: String::new(), 
            visible_chars: 0, 
            timer: 0, 
            is_active: false, 
            window_slot: None,
            portrait_slot: None,
            portrait_data: None,
            blink_timer: 120,
            eyes_closed: false,
            mouth_timer: 0,
            mouth_open: false,
        }
    }

    pub fn open(
        &mut self, 
        text: String, 
        window_engine: &mut WindowEngine,
        vram: &mut Vram,
        palette: &mut Vec<Color>,
        portrait_id: Option<u16>
    ) {
        // Очистка текста от тегов
        let mut clean_text = text;
        clean_text = clean_text.replace("{N}", "\n")
                               .replace("{W1}", "").replace("{W2}", "")
                               .replace("{D1}", "").replace("{D2}", "").replace("{D3}", "")
                               .replace("{CLEAR}", "");

        clean_text = clean_text.replace("{LEADER}", "BOWIE");
        clean_text = clean_text.replace("{NAME;1}", "SARAH");
        clean_text = clean_text.replace("{NAME;2}", "CHESTER");
        clean_text = clean_text.replace("{NAME;3}", "JAHA");
        clean_text = clean_text.replace("{NAME;4}", "KAZIN");

        let mut final_text = String::new();
        let mut skip = false;
        for c in clean_text.chars() {
            if c == '{' { skip = true; }
            if !skip { final_text.push(c); }
            if c == '}' { skip = false; }
        }

        self.text = final_text;
        self.visible_chars = 0;
        self.timer = 0;
        self.is_active = true;
        self.blink_timer = 120;
        self.eyes_closed = false;
        self.mouth_timer = 0;
        self.mouth_open = false;

        // 1. Создаем основное окно диалога (высота 7 тайлов)
        self.window_slot = window_engine.create_empty_window(0x0220, 28, 7);
        if let Some(slot) = self.window_slot {
            window_engine.move_window(slot, 0x0213, 4); 
        }

        // 2. Создаем окно портрета
        self.portrait_slot = None;
        self.portrait_data = None;
        if let Some(pid) = portrait_id {
            if let Some(p_data) = load_portrait(pid as usize) {
                let _ = vram.write_bytes(VRAM_PORTRAITS, &p_data.tiles);
                for (i, c) in p_data.palette.iter().enumerate() {
                    if 16 + i < palette.len() {
                        palette[16 + i] = *c;
                    }
                }
                self.portrait_data = Some(p_data);
                
                self.portrait_slot = window_engine.create_window_with_layout(0xF8F6, 8, 10, &layouts::LAYOUT_PORTRAIT_WINDOW);
                if let Some(slot) = self.portrait_slot {
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
            }
        }
    }

    pub fn close(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot { window_engine.delete_window(slot); }
        if let Some(slot) = self.portrait_slot { window_engine.delete_window(slot); }
        self.window_slot = None;
        self.portrait_slot = None;
        self.portrait_data = None;
        self.is_active = false;
        self.text.clear();
    }

    pub fn tick(&mut self, window_engine: &mut WindowEngine) {
        if !self.is_active { return; }
        
        // Анимация текста
        if self.visible_chars < self.text.chars().count() {
            self.timer += 1;
            if self.timer >= 2 {
                self.visible_chars += 1;
                self.timer = 0;
                
                // Анимация рта (открывается/закрывается, пока печатается текст)
                self.mouth_timer += 1;
                if self.mouth_timer >= 4 { // <-- БЫЛО 2, СТАЛО 4
                    self.mouth_open = !self.mouth_open;
                    self.mouth_timer = 0;
                    self.update_portrait_tiles(window_engine);
                }
            }
        } else {
            // Если текст допечатан, рот закрыт
            if self.mouth_open {
                self.mouth_open = false;
                self.update_portrait_tiles(window_engine);
            }
        }

        // Анимация моргания
        if self.portrait_slot.is_some() && self.portrait_data.is_some() {
            if self.blink_timer > 0 {
                self.blink_timer -= 1;
                if self.blink_timer == 0 {
                    self.eyes_closed = !self.eyes_closed;
                    self.blink_timer = if self.eyes_closed { 10 } else { 120 };
                    self.update_portrait_tiles(window_engine);
                }
            }
        }
    }

    fn update_portrait_tiles(&self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.portrait_slot {
            if let Some(p_data) = &self.portrait_data {
                let start = window_engine.entries[slot].layout_start_index;
                let width = 8;
                let portrait_base_tile = (VRAM_PORTRAITS / 32) as u16;

                // 1. Обновляем глаза
                for eye in p_data.eyes.iter() {
                    let idx = start + ((eye.y as usize + 1) * width + (eye.x as usize + 1));
                    let (tx, ty) = if self.eyes_closed { (eye.alt_x, eye.alt_y) } else { (eye.x, eye.y) };
                    let tile_idx = portrait_base_tile + (((ty as usize * 8) + tx as usize) as u16);
                    window_engine.window_tile_layouts[idx] = tile_idx | VDPTILE_PALETTE2 | VDPTILE_PRIORITY;
                }

                // 2. Обновляем рот
                for mouth in p_data.mouth.iter() {
                    let idx = start + ((mouth.y as usize + 1) * width + (mouth.x as usize + 1));
                    let (tx, ty) = if self.mouth_open { (mouth.alt_x, mouth.alt_y) } else { (mouth.x, mouth.y) };
                    let tile_idx = portrait_base_tile + (((ty as usize * 8) + tx as usize) as u16);
                    window_engine.window_tile_layouts[idx] = tile_idx | VDPTILE_PALETTE2 | VDPTILE_PRIORITY;
                }
            }
        }
    }

    pub fn update_text(&self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot {
            for i in 0..5 {
                window_engine.write_string(slot, 1, 1 + i, "                          ");
            }           
            let visible_text: String = self.text.chars().take(self.visible_chars).collect();            
            for (i, line) in visible_text.split('\n').enumerate() {
                if i >= 3 { break; } 
                let safe_line: String = line.chars().take(26).collect();
                window_engine.write_string(slot, 1, 1 + (i as u8 * 2), &safe_line);
            }
        }
    }
}