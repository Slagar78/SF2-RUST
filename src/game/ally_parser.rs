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

// ============================================================
//  START DEFINITIONS  (allystartdefs.asm)
// ============================================================

struct AllyStartDef {
    class: String,
    level: u8,
    items: Vec<String>,
}

fn item_enum_to_display(enum_name: &str) -> String {
    enum_name
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_ally_start_defs() -> Vec<AllyStartDef> {
    let content = fs::read_to_string("src/data/stats/allies/allystartdefs.asm")
        .unwrap_or_default();

    let mut defs: Vec<AllyStartDef> = Vec::new();
    let mut cur_class: Option<String> = None;
    let mut cur_level: u8 = 1;
    let mut cur_items: Vec<String> = Vec::new();
    let mut in_items = false;
    let mut items_left: u8 = 0;

    for line in content.lines() {
        let clean = line.split(';').next().unwrap().trim();
        if clean.is_empty() { continue; }

        if clean.starts_with("startClass ") {
            if let Some(cls) = cur_class.take() {
                defs.push(AllyStartDef {
                    class: cls,
                    level: cur_level,
                    items: std::mem::take(&mut cur_items),
                });
            }
            cur_class = Some(clean["startClass".len()..].trim().to_string());
            cur_level = 1;
            cur_items.clear();
            in_items = false;
            items_left = 0;
        } else if clean.starts_with("startLevel ") {
            cur_level = clean["startLevel".len()..].trim().parse().unwrap_or(1);
        } else if clean.starts_with("startItems") {
            in_items = true;
            items_left = 4;
        } else if in_items && items_left > 0 {
            let mut token = clean;
            while token.ends_with('&') || token.ends_with(',') {
                token = token[..token.len() - 1].trim_end();
            }

            if !token.is_empty() && !token.eq_ignore_ascii_case("NOTHING") {
                let enum_name = token.split('|').next().unwrap_or(token).trim();
                if !enum_name.is_empty() && !enum_name.eq_ignore_ascii_case("NOTHING") {
                    cur_items.push(item_enum_to_display(enum_name));
                }
            }

            items_left -= 1;
            if items_left == 0 { in_items = false; }
        }
    }

    if let Some(cls) = cur_class {
        defs.push(AllyStartDef { class: cls, level: cur_level, items: cur_items });
    }

    defs
}

// ============================================================
//  CLASS MOVEMENT  (classnames.asm + classdefs.asm)
// ============================================================

fn load_class_movements() -> HashMap<String, u16> {
    let mut class_names: Vec<String> = Vec::new();
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
        let mut current_index = 0usize;
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

// ============================================================
//  CLASS TYPES  (classnames.asm + classtypes.asm)  <-- НОВОЕ
//  Возвращает map: "PRST" -> "BASE", "PLDN" -> "PROMO", "SORC" -> "SPECIAL"
// ============================================================

pub fn load_class_types() -> HashMap<String, String> {
    // 1. Читаем имена классов по порядку
    let mut class_names: Vec<String> = Vec::new();
    if let Ok(content) = fs::read_to_string("src/data/stats/allies/classes/classnames.asm") {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap_or("").trim();
            if let Some(start) = clean.find("className \"") {
                let rest = &clean[start + 11..];
                if let Some(end) = rest.find('"') {
                    class_names.push(rest[..end].to_string());
                }
            }
        }
    }

    // 2. Читаем типы и связываем по индексу
    let mut class_types = HashMap::new();
    if let Ok(content) = fs::read_to_string("src/data/stats/allies/classes/classtypes.asm") {
        let mut index = 0;
        for line in content.lines() {
            let clean = line.split(';').next().unwrap_or("").trim();
            if clean.starts_with("classType ") {
                let ctype = clean["classType".len()..].trim().to_string();
                if let Some(name) = class_names.get(index) {
                    class_types.insert(name.clone(), ctype);
                }
                index += 1;
            }
        }
    }
    class_types
}

// ============================================================
//  NAMES / MAPSPRITES
// ============================================================

fn load_ally_names() -> Vec<String> {
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
    names
}

fn load_ally_mapsprites() -> Vec<String> {
    let mut mapsprites = Vec::new();
    if let Ok(content) = fs::read_to_string("src/data/stats/allies/allymapsprites.asm") {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mapsprite ") {
                mapsprites.push(clean["mapsprite".len()..].trim().to_string());
            }
        }
    }
    mapsprites.truncate(30);
    mapsprites
}

// ============================================================
//  STATS + SPELLS  (allystatsXX.asm)
// ============================================================

struct StatsBlock {
    class: String,
    hp: u16, max_hp: u16,
    mp: u16, max_mp: u16,
    att: u16, def: u16, agi: u16,
    spells: Vec<(String, u8)>,
}

fn parse_ally_stats(id: usize) -> StatsBlock {
    let path = format!("src/data/stats/allies/stats/allystats{:02}.asm", id);
    let content = fs::read_to_string(&path).unwrap_or_default();

    let mut class = "SDMN".to_string();
    let mut hp = 1u16; let mut max_hp = 1u16;
    let mut mp = 0u16; let mut max_mp = 0u16;
    let mut att = 1u16; let mut def = 1u16; let mut agi = 1u16;
    let mut spells: Vec<(String, u8)> = Vec::new();

    let mut in_first_class = false;
    let mut in_spell_list = false;

    for line in content.lines() {
        let mut clean = line.split(';').next().unwrap().trim();

        if let Some(colon_idx) = clean.find(':') {
            clean = clean[colon_idx + 1..].trim();
        }
        if clean.is_empty() { continue; }

        if clean.starts_with("forClass") {
            if in_first_class { break; }
            in_first_class = true;
            class = clean["forClass".len()..].trim().to_string();
        } else if in_first_class {
            if clean.starts_with("hpGrowth") {
                let s = clean["hpGrowth".len()..].trim();
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() >= 2 {
                    hp = parts[0].trim().parse().unwrap_or(1);
                    max_hp = parts[1].trim().parse().unwrap_or(1);
                }
            } else if clean.starts_with("mpGrowth") {
                let s = clean["mpGrowth".len()..].trim();
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() >= 2 {
                    mp = parts[0].trim().parse().unwrap_or(0);
                    max_mp = parts[1].trim().parse().unwrap_or(0);
                }
            } else if clean.starts_with("attGrowth") {
                let s = clean["attGrowth".len()..].trim();
                let parts: Vec<&str> = s.split(',').collect();
                if !parts.is_empty() {
                    att = parts[0].trim().parse().unwrap_or(1);
                }
            } else if clean.starts_with("defGrowth") {
                let s = clean["defGrowth".len()..].trim();
                let parts: Vec<&str> = s.split(',').collect();
                if !parts.is_empty() {
                    def = parts[0].trim().parse().unwrap_or(1);
                }
            } else if clean.starts_with("agiGrowth") {
                let s = clean["agiGrowth".len()..].trim();
                let parts: Vec<&str> = s.split(',').collect();
                if !parts.is_empty() {
                    agi = parts[0].trim().parse().unwrap_or(1);
                }
            } else if clean.starts_with("spellList") || in_spell_list {
                in_spell_list = true;
                let s = clean
                    .replace("spellList", "")
                    .replace('&', "")
                    .replace(',', " ");
                let parts: Vec<&str> = s.split_whitespace().collect();
                for chunk in parts.chunks(2) {
                    if chunk.len() == 2 {
                        let lvl = chunk[0].parse::<u8>().unwrap_or(1);
                        let spell_name = chunk[1]
                            .split('|').next().unwrap_or(chunk[1])
                            .to_string();
                        if 1 >= lvl {
                            spells.push((spell_name, lvl));
                        }
                    }
                }
            }
        }
    }

    StatsBlock { class, hp, max_hp, mp, max_mp, att, def, agi, spells }
}

// ============================================================
//  PUBLIC API
// ============================================================

pub fn load_force_members() -> Vec<Combatant> {
    let names          = load_ally_names();
    let mapsprites     = load_ally_mapsprites();
    let start_defs     = load_ally_start_defs();
    let class_movements = load_class_movements();

    let mut members = Vec::with_capacity(30);

    for id in 0..30 {
        let stats = parse_ally_stats(id);

        let name = names.get(id)
            .cloned()
            .unwrap_or_else(|| format!("UNKNOWN {}", id));
        let mapsprite = mapsprites.get(id)
            .cloned()
            .unwrap_or_else(|| "BOWIE_PROMO".to_string());

        let (class, level, items) = match start_defs.get(id) {
            Some(s) => (s.class.clone(), s.level, s.items.clone()),
            None    => (stats.class.clone(), 1u8, Vec::new()),
        };

        let movement = *class_movements.get(&class).unwrap_or(&6);

        members.push(Combatant {
            name,
            class,
            level,
            mapsprite,
            items,
            spells: stats.spells,
            hp: stats.hp, max_hp: stats.max_hp,
            mp: stats.mp, max_mp: stats.max_mp,
            attack: stats.att,
            defense: stats.def,
            agility: stats.agi,
            movement,
        });
    }

    members
}