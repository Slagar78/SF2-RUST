// src/game/script/text_parser.rs
use std::fs;
use std::collections::HashMap;

pub struct GameScript {
    pub strings: HashMap<u16, String>,
}

impl GameScript {
    pub fn new() -> Self {
        let mut strings = HashMap::new();
        
        // Читаем gamescript.txt
        if let Ok(content) = fs::read_to_string("src/data/scripting/text/gamescript.txt") {
            for line in content.lines() {
                let clean = line.trim();
                if clean.is_empty() { continue; }

                // Разбиваем строку по знаку '=' (только один раз)
                if let Some((id_str, text)) = clean.split_once('=') {
                    // ID в файле записаны в HEX (например, 0000, 0200, 000A)
                    if let Ok(id) = u16::from_str_radix(id_str.trim(), 16) {
                        strings.insert(id, text.trim().to_string());
                    }
                }
            }
        } else {
            eprintln!("Не удалось загрузить gamescript.txt");
        }

        Self { strings }
    }

    pub fn get(&self, id: u16) -> String {
        self.strings.get(&id).cloned().unwrap_or_else(|| format!("[Текст {} не найден]", id))
    }
}