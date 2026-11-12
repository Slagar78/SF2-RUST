// src/game/item_parser.rs
use std::fs;

#[allow(dead_code)]
pub struct Item {
    pub id: usize,
    pub name: String,
    pub item_type: String,
    pub price: u16,
    pub equip_effects: Vec<(String, i32)>,
}

pub fn load_items() -> Vec<Item> {
    let mut items = Vec::new();
    let mut names = Vec::new();

    // 1. Парсим имена предметов
    if let Ok(content) = fs::read_to_string("src/data/stats/items/itemnames.asm") {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("itemName ") {
                let raw = clean.trim_start_matches("itemName").trim();
                let mut name = String::new();
                
                // Разбиваем на токены по запятым и пробелам. 
                // Например: '"Short", 13, "Axe"' -> ['"Short"', '13', '"Axe"']
                let tokens: Vec<&str> = raw.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).collect();
                
                for token in tokens {
                    if token == "11" {
                        name.push('\n'); // 11 = просто перенос
                    } else if token == "13" {
                        name.push_str("\n "); // 13 = перенос + пробел
                    } else if token.starts_with('"') && token.ends_with('"') {
                        let word = &token[1..token.len()-1];
                        // Если перед словом нет переноса и это не начало строки, ставим пробел
                        if !name.is_empty() && !name.ends_with('\n') && !name.ends_with(' ') {
                            name.push(' ');
                        }
                        name.push_str(word);
                    }
                }
                names.push(name);
            }
        }
    }

    // 2. Парсим статы предметов
    let mut current_id = 0;
    let mut current_type = "NONE".to_string();
    let mut current_price = 0;
    let mut current_effects = Vec::new();
    let mut in_effects = false;

    if let Ok(content) = fs::read_to_string("src/data/stats/items/itemdefs.asm") {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.is_empty() { continue; }

            if clean.starts_with("equipFlags") {
                if current_id > 0 {
                    let name = names.get(current_id - 1).cloned().unwrap_or_else(|| format!("Item {}", current_id - 1));
                    items.push(Item {
                        id: current_id - 1,
                        name,
                        item_type: current_type.clone(),
                        price: current_price,
                        equip_effects: current_effects.clone(),
                    });
                }
                current_id += 1;
                current_type = "NONE".to_string();
                current_price = 0;
                current_effects.clear();
                in_effects = false;
            } else if clean.starts_with("itemType") {
                current_type = clean.replace("itemType", "").trim().to_string();
            } else if clean.starts_with("price") {
                current_price = clean.replace("price", "").trim().parse().unwrap_or(0);
            } else if clean.starts_with("equipEffects") {
                in_effects = true;
                let effect_str = clean.replace("equipEffects", "");
                parse_effect(&effect_str, &mut current_effects);
            } else if in_effects && clean.contains(',') {
                if !clean.starts_with("NONE") {
                    parse_effect(clean, &mut current_effects);
                }
            }
        }
        
        if current_id > 0 {
            let name = names.get(current_id - 1).cloned().unwrap_or_else(|| format!("Item {}", current_id - 1));
            items.push(Item {
                id: current_id - 1,
                name,
                item_type: current_type.clone(),
                price: current_price,
                equip_effects: current_effects.clone(),
            });
        }
    }

    items
}

fn parse_effect(effect_str: &str, effects: &mut Vec<(String, i32)>) {
    let clean = effect_str.replace("&", "").replace(",", " ");
    let parts: Vec<&str> = clean.split_whitespace().collect();
    if parts.len() >= 2 {
        let name = parts[0].to_string();
        let val = parts[1].parse::<i32>().unwrap_or(0);
        if name != "NONE" && name != "UNDEFINED1" && name != "UNDEFINED2" {
            effects.push((name, val));
        }
    }
}