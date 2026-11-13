// src/game/script/entity_events.rs
use std::fs;
use std::collections::HashMap;

pub struct EntityEvents {
    pub event_map: HashMap<String, String>, // NPC -> Event Handler (например, Map3_EntityEvent0)
    pub script_texts: HashMap<String, u16>, // Event Handler -> Text ID
    pub script_commands: HashMap<String, String>, // Event Handler -> Script Name (cs_513D6)
}

impl EntityEvents {
    pub fn new(map_id: u8) -> Self {
        let mut event_map = HashMap::new();
        let mut script_texts = HashMap::new();
        let mut script_commands = HashMap::new();

        let path = format!("src/data/maps/entries/map{:02}/mapsetups/s2_entityevents.asm", map_id);
        if let Ok(content) = fs::read_to_string(&path) {
            let mut current_handler = String::new();

            for line in content.lines() {
                let clean = line.split(';').next().unwrap().trim();
                if clean.is_empty() { continue; }

                // 1. Привязка NPC к скрипту (msEntityEvent ALLY_SARAH, DOWN, Map3_EntityEvent0)
                if clean.starts_with("msEntityEvent ") {
                    let args: Vec<&str> = clean["msEntityEvent ".len()..].split(',').map(|s| s.trim()).collect();
                    if args.len() >= 3 {
                        let entity_name = args[0].to_string();
                        let script_label = args[2].split('-').next().unwrap().trim().to_string();
                        event_map.insert(entity_name, script_label);
                    }
                }
                // 2. Новый главный обработчик (Map3_EntityEvent0:)
                else if clean.ends_with(':') && (clean.starts_with("Map") || clean.starts_with("ms_")) {
                    let label = clean.trim_end_matches(':').to_string();
                    // Игнорируем внутренние метки типа byte_50F6A
                    if !label.starts_with("byte_") && !label.starts_with("loc_") && !label.starts_with("return_") {
                        current_handler = label;
                    }
                }
                // 3. Текст
                else if clean.starts_with("txt ") {
                    if !current_handler.is_empty() {
                        let arg = clean["txt ".len()..].trim();
                        let num_str: String = arg.chars().take_while(|c| c.is_ascii_digit()).collect();
                        let num = num_str.parse::<u16>().unwrap_or(0);
                        script_texts.entry(current_handler.clone()).or_insert(num);
                    }
                }
                // 4. Скрипт
                else if clean.starts_with("script ") {
                    if !current_handler.is_empty() {
                        let arg = clean["script ".len()..].trim();
                        script_commands.entry(current_handler.clone()).or_insert(arg.to_string());
                    }
                }
            }
        }

        Self { event_map, script_texts, script_commands }
    }

    pub fn get_script_name(&self, entity_id: &str) -> Option<String> {
        if let Some(event_handler) = self.event_map.get(entity_id) {
            return self.script_commands.get(event_handler).cloned();
        }
        None
    }

    pub fn get_text_id(&self, entity_id: &str) -> Option<u16> {
        if let Some(script_label) = self.event_map.get(entity_id) {
            if let Some(&txt_id) = self.script_texts.get(script_label) {
                return Some(txt_id);
            }
        }
        None
    }
}