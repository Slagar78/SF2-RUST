// src/game/script/entity_events.rs
use std::fs;
use std::collections::HashMap;

pub struct EntityEvents {
    pub event_map: HashMap<String, String>, // NPC -> Event Handler (например, Map3_EntityEvent0)
    pub script_texts: HashMap<String, u16>, // Event Handler -> Text ID
    pub script_commands: HashMap<String, String>, // Event Handler -> Script Name (cs_513D6)
}

impl EntityEvents {
    pub fn new(event_paths: &[String]) -> Self {
        let mut event_map = HashMap::new();
        let mut script_texts = HashMap::new();
        let mut script_commands = HashMap::new();

        // Читаем ВСЕ пути, которые нашёл finder.rs (обычно 1 файл, но иногда их несколько)
        for path in event_paths {
          println!("[EntityEvents] Загрузка файла событий: {}", path);
            let full_path = format!("src/{}", path);
            if let Ok(content) = fs::read_to_string(&full_path) {
                let mut current_handler = String::new();

                for line in content.lines() {
                    let clean = line.split(';').next().unwrap().trim();
                    if clean.is_empty() { continue; }

                    if clean.starts_with("msEntityEvent ") {
                        let args: Vec<&str> = clean["msEntityEvent ".len()..].split(',').map(|s| s.trim()).collect();
                        if args.len() >= 3 {
                            let entity_name = args[0].to_string();
                            let script_label = args[2].split('-').next().unwrap().trim().to_string();
                            event_map.insert(entity_name, script_label);
                        }
                    }
                    else if clean.ends_with(':') && (clean.starts_with("Map") || clean.starts_with("ms_")) {
                        let label = clean.trim_end_matches(':').to_string();
                        if !label.starts_with("byte_") && !label.starts_with("loc_") && !label.starts_with("return_") {
                            current_handler = label;
                        }
                    }
                    else if clean.starts_with("txt ") {
                        if !current_handler.is_empty() {
                            let arg = clean["txt ".len()..].trim();
                            let num_str: String = arg.chars().take_while(|c| c.is_ascii_digit()).collect();
                            let num = num_str.parse::<u16>().unwrap_or(0);
                            script_texts.entry(current_handler.clone()).or_insert(num);
                        }
                    }
                    else if clean.starts_with("script ") {
                        if !current_handler.is_empty() {
                            let arg = clean["script ".len()..].trim();
                            script_commands.entry(current_handler.clone()).or_insert(arg.to_string());
                        }
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