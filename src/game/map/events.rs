// src/game/map/events.rs
use crate::game::map::loader::{MapData, MapLayoutBlock, RoofEvent};
use crate::game::player::{PlayerState, Facing};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MapEventResult {
    None,
    Warp(u8, u8, u8, u8),
}

pub struct MapEventsState {
    pub active_roofs: HashMap<(u8, u8, u8, u8, u8, u8), Vec<MapLayoutBlock>>,
}

impl MapEventsState {
    pub fn new() -> Self {
        Self { active_roofs: HashMap::new() }
    }

    pub fn reset(&mut self) {
        self.active_roofs.clear();
    }

    pub fn update_roofs_on_warp(&mut self, map: &mut MapData, x: i32, y: i32) {
        let mut scnd_x = 0;
        let mut scnd_y = 0;
        for area in &map.areas {
            if x >= area.start_x as i32 && x <= area.end_x as i32
            && y >= area.start_y as i32 && y <= area.end_y as i32 {
                scnd_x = area.scnd_x as i32;
                scnd_y = area.scnd_y as i32;
                break;
            }
        }

        let check_x = x + scnd_x;
        let check_y = y + scnd_y;

        for ev in &map.roof_events {
            let inside = check_x >= ev.dest_x as i32
                && check_x < ev.dest_x as i32 + ev.width as i32
                && check_y >= ev.dest_y as i32
                && check_y < ev.dest_y as i32 + ev.height as i32;
            
            let key = (ev.dest_x, ev.dest_y, ev.src_x, ev.src_y, ev.width, ev.height);

            if inside {
                if !self.active_roofs.contains_key(&key) {
                    let mut saved = Vec::new();
                    for dy in 0..ev.height as usize {
                        for dx in 0..ev.width as usize {
                            let dst_idx = (ev.dest_y as usize + dy) * 64 + (ev.dest_x as usize + dx);
                            if dst_idx < map.layout.len() {
                                saved.push(map.layout[dst_idx]);
                                if ev.src_x == 255 || ev.src_y == 255 {
                                    map.layout[dst_idx] = MapLayoutBlock { block_index: 0, flags: 0 };
                                } else {
                                    let src_idx = (ev.src_y as usize + dy) * 64 + (ev.src_x as usize + dx);
                                    if src_idx < map.layout.len() {
                                        map.layout[dst_idx] = map.layout[src_idx];
                                    }
                                }
                            }
                        }
                    }
                    self.active_roofs.insert(key, saved);
                }
            } else {
                if let Some(saved) = self.active_roofs.remove(&key) {
                    let mut idx = 0;
                    for dy in 0..ev.height as usize {
                        for dx in 0..ev.width as usize {
                            let dst_idx = (ev.dest_y as usize + dy) * 64 + (ev.dest_x as usize + dx);
                            if dst_idx < map.layout.len() && idx < saved.len() {
                                map.layout[dst_idx] = saved[idx];
                            }
                            idx += 1;
                        }
                    }
                }
            }
        }
    }
}

pub fn check_tile_events(_player: &mut PlayerState, map: &mut MapData, events: &mut MapEventsState, nx: i32, ny: i32) -> MapEventResult {
    for warp in &map.warps {
        if (warp.src_x == 255 || warp.src_x == nx as u8) && (warp.src_y == 255 || warp.src_y == ny as u8) {
            return MapEventResult::Warp(warp.dest_map, warp.dest_x, warp.dest_y, warp.facing);
        }
    }

    for ev in &map.step_events {
        if ev.trig_x == nx as u8 && ev.trig_y == ny as u8 {
            for y in 0..ev.height as usize {
                for x in 0..ev.width as usize {
                    let src_idx = (ev.src_y as usize + y) * 64 + (ev.src_x as usize + x);
                    let dst_idx = (ev.dest_y as usize + y) * 64 + (ev.dest_x as usize + x);
                    if src_idx < map.layout.len() && dst_idx < map.layout.len() {
                        map.layout[dst_idx] = map.layout[src_idx];
                    }
                }
            }
        }
    }

    let mut unique_roofs: HashMap<(u8, u8, u8, u8, u8, u8), Vec<&RoofEvent>> = HashMap::new();
    for ev in &map.roof_events {
        let key = (ev.dest_x, ev.dest_y, ev.src_x, ev.src_y, ev.width, ev.height);
        unique_roofs.entry(key).or_default().push(ev);
    }

    for (key, evs) in unique_roofs {
        let is_opening = evs.iter().any(|ev| nx == ev.trig_x as i32 && ny == ev.trig_y as i32);
        let is_closing = evs.iter().any(|ev| nx == ev.trig_x as i32 && ny == ev.trig_y as i32 + 1);

        if is_opening {
            if !events.active_roofs.contains_key(&key) {
                let ev = evs[0];
                let mut saved = Vec::new();
                for y in 0..ev.height as usize {
                    for x in 0..ev.width as usize {
                        let dst_idx = (ev.dest_y as usize + y) * 64 + (ev.dest_x as usize + x);
                        if dst_idx < map.layout.len() {
                            saved.push(map.layout[dst_idx]);
                            if ev.src_x == 255 || ev.src_y == 255 {
                                map.layout[dst_idx] = MapLayoutBlock { block_index: 0, flags: 0 };
                            } else {
                                let src_idx = (ev.src_y as usize + y) * 64 + (ev.src_x as usize + x);
                                if src_idx < map.layout.len() {
                                    map.layout[dst_idx] = map.layout[src_idx];
                                }
                            }
                        }
                    }
                }
                events.active_roofs.insert(key, saved);
            }
        } else if is_closing {
            if let Some(saved) = events.active_roofs.remove(&key) {
                let ev = evs[0];
                let mut idx = 0;
                for y in 0..ev.height as usize {
                    for x in 0..ev.width as usize {
                        let dst_idx = (ev.dest_y as usize + y) * 64 + (ev.dest_x as usize + x);
                        if dst_idx < map.layout.len() && idx < saved.len() {
                            map.layout[dst_idx] = saved[idx];
                        }
                        idx += 1;
                    }
                }
            }
        }
    }

    MapEventResult::None
}

// ВОТ ПОЛНАЯ ФУНКЦИЯ INTERACT
pub fn interact(
    player: &mut PlayerState, 
    map: &mut MapData, 
    flags: &mut crate::game::flags::GameFlags,
    game_script: &crate::game::script::text_parser::GameScript,
) -> Option<(String, Option<String>, String, String)> {
    let (dx, dy) = match player.facing {
        Facing::Up    => (0, -1),
        Facing::Down  => (0,  1),
        Facing::Left  => (-1, 0),
        Facing::Right => ( 1, 0),
    };
    let target_x = player.tile_x + dx;
    let target_y = player.tile_y + dy;

    if target_x < 0 || target_x >= 64 || target_y < 0 || target_y >= 64 { return None; }
    let target_idx = (target_y * 64 + target_x) as usize;

    // 1. Проверяем NPC
    for entity in &mut map.entities {
        if entity.x as i32 == target_x && entity.y as i32 == target_y && !entity.is_hidden {
            
            entity.is_talking = true;

            let dx = player.tile_x as i32 - entity.x as i32;
            let dy = player.tile_y as i32 - entity.y as i32;
            if dx.abs() > dy.abs() {
                entity.facing = if dx > 0 { 3 } else { 2 };
            } else {
                entity.facing = if dy > 0 { 1 } else { 0 };
            }

            // Ищем ID текста
            let mut txt_id = 0x0200; 
            if let Some(id) = map.entity_events.get_text_id(&entity.event_id) {
                txt_id = id;
            } else if let Some(id) = map.entity_events.get_text_id(&entity.sprite_id.to_string()) {
                txt_id = id;
            }

            // Ищем скрипт
            // Если скрипт уже играл - не запускаем его снова
            let script_name = if entity.script_played {
                None
            } else {
                map.entity_events.get_script_name(&entity.event_id)
                    .or_else(|| map.entity_events.get_script_name(&entity.sprite_id.to_string()))
            };

            let mut text = game_script.get(txt_id);
            
            // Очистка текста от тегов
            text = text.replace("{N}", "\n")
                       .replace("{W1}", "").replace("{W2}", "")
                       .replace("{D1}", "").replace("{D2}", "").replace("{D3}", "")
                       .replace("{CLEAR}", "");

            text = text.replace("{LEADER}", "BOWIE");
            text = text.replace("{NAME;1}", "SARAH");
            text = text.replace("{NAME;2}", "CHESTER");
            text = text.replace("{NAME;3}", "JAHA");
            text = text.replace("{NAME;4}", "KAZIN");
            text = text.replace("{NAME;5}", "SLADE");
            // ... добавь остальных по мере надобности

            // Убираем оставшиеся теги
            let mut final_text = String::new();
            let mut skip = false;
            for c in text.chars() {
                if c == '{' { skip = true; }
                if !skip { final_text.push(c); }
                if c == '}' { skip = false; }
            }
            
            return Some((final_text, script_name, entity.event_id.clone(), entity.sprite_name.clone()));
        }
    }

    // 2. Проверяем предметы
    let mut found_item = None;
    for (i, item) in map.items.iter().enumerate() {
        if item.x as i32 == target_x && item.y as i32 == target_y {
            found_item = Some(i);
            break;
        }
    }

    if let Some(index) = found_item {
        let item = map.items[index];
        if !flags.get(item.flag) {
            flags.set(item.flag); 
            map.items.remove(index); 
            println!("🎁 ВЫ НАШЛИ ПРЕДМЕТ! ID: {}", item.item_id);
            map.layout[target_idx].flags = 0x0000; 
        }
    }

    None
}