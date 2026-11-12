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
            // Ищем "itemName " где угодно в строке, чтобы поймать "table_ItemNames:itemName"
            if let Some(idx) = clean.find("itemName ") {
                let raw = clean[idx + 9..].trim(); // 9 - длина "itemName "
                let mut name = String::new();
                
                // Разбиваем по кавычкам. 
                // Пример: '"Medical", 13, "Herb"' -> ["", "Medical", ", 13, ", "Herb", ""]
                let parts: Vec<&str> = raw.split('"').collect();
                
                for i in 0..parts.len() {
                    if i % 2 == 1 {
                        // Нечетные индексы — это текст внутри кавычек (название предмета)
                        let word = parts[i];
                        if !name.is_empty() && !name.ends_with('\n') && !name.ends_with(' ') {
                            name.push(' ');
                        }
                        name.push_str(word);
                    } else {
                        // Четные индексы — это то, что между кавычками (там ищем 11 и 13)
                        let code = parts[i].trim();
                        if code.contains("11") {
                            name.push('\n');
                        }
                        if code.contains("13") {
                            name.push_str("\n ");
                        }
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