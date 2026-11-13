// src/game/script/portrait_properties.rs
use std::fs;
use std::collections::HashMap;

pub struct PortraitProperties {
    pub sprite_to_portrait: HashMap<String, String>,
    pub portrait_ids: HashMap<String, u16>,
}

impl PortraitProperties {
    pub fn new() -> Self {
        let mut sprite_to_portrait = HashMap::new();
        let mut portrait_ids = HashMap::new();

        // 1. Парсим ID портретов
        if let Ok(content) = fs::read_to_string("src/data/graphics/portraits/portraitnames.txt.txt") {
            for line in content.lines() {
                let clean = line.split(';').next().unwrap().trim();
                if let Some(start) = clean.find("PORTRAIT_") {
                    let rest = &clean[start + 9..];
                    if let Some(end) = rest.find(':') {
                        let name = rest[..end].to_string();
                        if let Some(equ_idx) = clean.find("equ") {
                            let val_str = clean[equ_idx + 3..].trim();
                            if let Ok(id) = val_str.parse::<i32>() {
                                portrait_ids.insert(name, if id < 0 { 255 } else { id as u16 });
                            }
                        }
                    }
                }
            }
        }

        // 2. Парсим свойства спрайтов
        if let Ok(content) = fs::read_to_string("src/data/spritedialogproperties.asm") {
            let mut current_sprite = String::new();
            for line in content.lines() {
                let clean = line.split(';').next().unwrap().trim();
                if clean.starts_with("mapsprite ") {
                    current_sprite = clean.replace("mapsprite ", "").trim().to_string();
                } else if clean.starts_with("portrait ") {
                    let mut portrait = clean.replace("portrait ", "").trim().to_string();
                    portrait = portrait.replace("PORTRAIT_", "");
                    sprite_to_portrait.insert(current_sprite.clone(), portrait);
                }
            }
        }

        Self { sprite_to_portrait, portrait_ids }
    }

    pub fn get_portrait_id(&self, sprite_name: &str) -> Option<u16> {
        let clean = sprite_name.replace("ALLY_", "").replace("MAPSPRITE_", "");
        
        let p_name = self.sprite_to_portrait.get(&clean)
            .or_else(|| self.sprite_to_portrait.get(&format!("{}_BASE", clean)))
            .or_else(|| self.sprite_to_portrait.get(&format!("{}_PROMO", clean)));

        if let Some(p_name) = p_name {
            if let Some(&id) = self.portrait_ids.get(p_name) {
                if id != 255 { return Some(id); }
            }
        }
        None
    }
}