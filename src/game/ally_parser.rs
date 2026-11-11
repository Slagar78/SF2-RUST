// src/game/ally_parser.rs
use std::fs;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct Combatant {
    pub name: String,
    pub class: String,
    pub level: u8,
    pub mapsprite: String,
    pub items: Vec<String>,
    pub spells: Vec<(String, u8)>,
    
    pub hp: u16, pub max_hp: u16,
    pub mp: u16, pub max_mp: u16,
    pub attack: u16, pub defense: u16, pub agility: u16,
    pub movement: u16,
}

// Вспомогательная функция для чтения движений классов из classdefs.asm
fn load_class_movements() -> HashMap<String, u16> {
    let mut class_names = Vec::new();
    if let Ok(content) = fs::read_to_string("src/data/stats/allies/classes/classnames.asm") {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if let Some(start) = clean.find("className \"") {
                let rest = &clean[start + 11..];
                if let Some(end) = rest.find('"') {
                    class_names.push(rest[..end].to_string());
                }
            }
        }
    }

    let mut movements = HashMap::new();
    if let Ok(content) = fs::read_to_string("src/data/stats/allies/classes/classdefs.asm") {
        let mut current_index = 0;
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mov ") {
                let parts: Vec<&str> = clean.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(mov_val) = parts[1].parse::<u16>() {
                        if let Some(class_name) = class_names.get(current_index) {
                            movements.insert(class_name.clone(), mov_val);
                        }
                    }
                }
                current_index += 1;
            }
        }
    }
    movements
}

pub fn load_force_members() -> Vec<Combatant> {
    let mut members = Vec::new();

    // 1. Читаем имена (первые 30)
    let mut names = Vec::new();
    if let Ok(content) = fs::read_to_string("src/data/stats/allies/allynames.asm") {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if let Some(start) = clean.find("allyName \"") {
                let rest = &clean[start + 10..];
                if let Some(end) = rest.find('"') {
                    names.push(rest[..end].to_string());
                }
            }
        }
    }
    names.truncate(30);

    // 2. Читаем мапспрайты (первые 30)
    let mut mapsprites = Vec::new();
    if let Ok(content) = fs::read_to_string("src/data/stats/allies/allymapsprites.asm") {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mapsprite ") {
                let sprite = clean.replace("mapsprite ", "").trim().to_string();
                mapsprites.push(sprite);
            }
        }
    }
    mapsprites.truncate(30);

    // Загружаем движения классов (MOV)
    let class_movements = load_class_movements();

    // 3. Читаем статы из папки stats/allystatsXX.asm
    for id in 0..30 {
        let stats_path = format!("src/data/stats/allies/stats/allystats{:02}.asm", id);
        let content = fs::read_to_string(&stats_path).unwrap_or_default();
        
        let mut class = "SDMN".to_string();
        let mut hp = 1; let mut max_hp = 1;
        let mut mp = 0; let mut max_mp = 0;
        let mut att = 1; let mut def = 1; let mut agi = 1;
        let mut spells = Vec::new();
        
        let mut in_first_class = false;
        let mut in_spell_list = false;

        for line in content.lines() {
            // 1. Убираем комментарии
            let mut clean = line.split(';').next().unwrap().trim();
            
            // 2. Отрезаем метку в начале строки (например, "AllyStats01: ")
            if let Some(colon_idx) = clean.find(':') {
                clean = clean[colon_idx + 1..].trim();
            }

            if clean.is_empty() { continue; }

            if clean.starts_with("forClass") {
                if in_first_class { break; } 
                in_first_class = true;
                class = clean.replace("forClass", "").trim().to_string();
            } else if in_first_class {
                // ИСПРАВЛЕНИЕ ОШИБКИ E0716: сохраняем результат replace() в переменную
                if clean.starts_with("hpGrowth") {
                    let clean_growth = clean.replace("hpGrowth", "");
                    let parts: Vec<&str> = clean_growth.split(',').collect();
                    if parts.len() >= 2 {
                        hp = parts[0].trim().parse().unwrap_or(1);
                        max_hp = parts[1].trim().parse().unwrap_or(1);
                    }
                } else if clean.starts_with("mpGrowth") {
                    let clean_growth = clean.replace("mpGrowth", "");
                    let parts: Vec<&str> = clean_growth.split(',').collect();
                    if parts.len() >= 2 {
                        mp = parts[0].trim().parse().unwrap_or(0);
                        max_mp = parts[1].trim().parse().unwrap_or(0);
                    }
                } else if clean.starts_with("attGrowth") {
                    let clean_growth = clean.replace("attGrowth", "");
                    let parts: Vec<&str> = clean_growth.split(',').collect();
                    if !parts.is_empty() {
                        att = parts[0].trim().parse().unwrap_or(1);
                    }
                } else if clean.starts_with("defGrowth") {
                    let clean_growth = clean.replace("defGrowth", "");
                    let parts: Vec<&str> = clean_growth.split(',').collect();
                    if !parts.is_empty() {
                        def = parts[0].trim().parse().unwrap_or(1);
                    }
                } else if clean.starts_with("agiGrowth") {
                    let clean_growth = clean.replace("agiGrowth", "");
                    let parts: Vec<&str> = clean_growth.split(',').collect();
                    if !parts.is_empty() {
                        agi = parts[0].trim().parse().unwrap_or(1);
                    }
                } else if clean.starts_with("spellList") || in_spell_list {
                    in_spell_list = true;
                    let spell_clean = clean.replace("spellList", "").replace("&", "").replace(",", " ");
                    let parts: Vec<&str> = spell_clean.split_whitespace().collect();
                    for chunk in parts.chunks(2) {
                        if chunk.len() == 2 {
                            let lvl = chunk[0].parse::<u8>().unwrap_or(1);
                            let spell_name = chunk[1].split('|').next().unwrap_or(chunk[1]).to_string();
                            if 1 >= lvl { 
                                spells.push((spell_name, lvl));
                            }
                        }
                    }
                }
            }
        }

        let name = names.get(id).cloned().unwrap_or_else(|| format!("UNKNOWN {}", id));
        let mapsprite = mapsprites.get(id).cloned().unwrap_or_else(|| "BOWIE_PROMO".to_string());
        
        // Берем движение из распарсенных классов, если нет - ставим 6
        let movement = *class_movements.get(&class).unwrap_or(&6);

        members.push(Combatant {
            name,
            class,
            level: 1,
            mapsprite,
            items: vec!["Wooden Sword".to_string(), "Medical Herb".to_string()],
            spells,
            hp, max_hp,
            mp, max_mp,
            attack: att, defense: def, agility: agi,
            movement,
        });
    }

    members
}