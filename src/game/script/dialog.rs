// src/game/script/dialog.rs
use crate::game::menu::window_engine::WindowEngine;

pub struct DialogState {
    pub text: String,
    pub visible_chars: usize,
    pub timer: u8,
    pub is_active: bool,
    pub window_slot: Option<usize>,
}

impl DialogState {
    pub fn new() -> Self {
        Self { text: String::new(), visible_chars: 0, timer: 0, is_active: false, window_slot: None }
    }

    pub fn open(&mut self, text: String, window_engine: &mut WindowEngine) {
        self.text = text.replace("{N}", "\n")
                         .replace("{LEADER}", "BOWIE")
                         .replace("{W1}", "").replace("{W2}", "");
        self.visible_chars = 0;
        self.timer = 0;
        self.is_active = true;

        // Делаем высоту 8 тайлов (64 пикселя), чтобы влезло 3 строки
        self.window_slot = window_engine.create_empty_window(0x0220, 28, 8);
        
        if let Some(slot) = self.window_slot {
            // Поднимаем окно чуть выше: Y = 19 (152 пикселя), X = 2
            window_engine.move_window(slot, 0x0213, 4); 
        }
    }

    pub fn close(&mut self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot {
            window_engine.delete_window(slot);
        }
        self.window_slot = None;
        self.is_active = false;
        self.text.clear();
    }

    pub fn tick(&mut self) {
        if !self.is_active { return; }
        
        if self.visible_chars < self.text.chars().count() {
            self.timer += 1;
            if self.timer >= 2 {
                self.visible_chars += 1;
                self.timer = 0;
            }
        }
    }

    pub fn update_text(&self, window_engine: &mut WindowEngine) {
        if let Some(slot) = self.window_slot {
            // Очищаем внутреннюю область окна (6 строк по 26 символов)
            for i in 0..6 {
                window_engine.write_string(slot, 1, 1 + i, "                          ");
            }
            
            let visible_text: String = self.text.chars().take(self.visible_chars).collect();
            
            // Строки идут через 2 тайла (i * 2), теперь 3-я строка влезет
            for (i, line) in visible_text.split('\n').enumerate() {
                if i >= 3 { break; } 
                let safe_line: String = line.chars().take(26).collect();
                window_engine.write_string(slot, 1, 1 + (i as u8 * 2), &safe_line);
            }
        }
    }
}