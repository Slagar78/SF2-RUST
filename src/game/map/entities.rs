// src/game/map/entities.rs
use std::fs;
use std::collections::{HashMap, HashSet};
use crate::game::map::loader::MapLayoutBlock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EntityMovement {
    Fixed,
    Walking { zone_x: u8, zone_y: u8, zone_size: u8 },
}

#[derive(Clone, Debug)]
pub struct MapEntity {
    pub x: u8,
    pub y: u8,
    pub facing: u8,
    pub original_facing: u8,
    pub sprite_id: u8,
    pub event_id: String,
    pub sprite_name: String,
    pub movement: EntityMovement,
    pub is_hidden: bool,
    pub is_talking: bool,
    pub script_played: bool,
    pub is_scripting: bool,
    pub pixel_x: i32,
    pub pixel_y: i32,
    pub target_x: i32,
    pub target_y: i32,
    pub travel_x: i32,
    pub travel_y: i32,
    pub anim_counter: u8,
    pub frame: u8,
    pub idle_timer: u16,
}

pub fn load_sprite_names() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    let path = "src/data/graphics/mapsprites/mapspritenames.txt";
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.is_empty() { continue; }
            let parts: Vec<&str> = clean.split("equ").collect();
            if parts.len() == 2 {
                let name = parts[0].trim().trim_end_matches(':').trim();
                if let Ok(id) = parts[1].trim().parse::<u8>() {
                    map.insert(name.to_string(), id);
                }
            }
        }
    }
    map
}

pub fn load_entities(map_id: u8) -> Vec<MapEntity> {
    let mut entities = Vec::new();
    let path = format!("src/data/maps/entries/map{:02}/mapsetups/s1_entities.asm", map_id);
    let content = match fs::read_to_string(&path) { Ok(c) => c, Err(_) => return entities };
    let sprite_map = load_sprite_names();

    let mut npc_id: u16 = 128;

    for line in content.lines() {
        let clean = line.split(';').next().unwrap().trim();
        if clean.is_empty() { continue; }

        let (macro_name, is_walking) = if clean.starts_with("msFixedEntity ") {
            ("msFixedEntity", false)
        } else if clean.starts_with("msWalkingEntity ") {
            ("msWalkingEntity", true)
        } else {
            continue;
        };

        let args = parse_args(clean, macro_name);
        if args.len() >= 4 {
            let x = args[0].parse::<u8>().unwrap_or(0);
            let y = args[1].parse::<u8>().unwrap_or(0);
            let px = x as i32 * 384;
            let py = y as i32 * 384;

            let movement = if is_walking && args.len() >= 7 {
                EntityMovement::Walking {
                    zone_x: args[4].parse::<u8>().unwrap_or(0),
                    zone_y: args[5].parse::<u8>().unwrap_or(0),
                    zone_size: args[6].parse::<u8>().unwrap_or(0),
                }
            } else {
                EntityMovement::Fixed
            };

            let sprite_name = args[3].clone();
            
            let event_id = if sprite_name.starts_with("ALLY_") {
                sprite_name.clone()
            } else {
                let id = npc_id.to_string();
                npc_id += 1;
                id
            };

            let facing = parse_facing(&args[2]);

            entities.push(MapEntity {
                x, y,
                facing,
                original_facing: facing,
                sprite_id: resolve_sprite(&sprite_name, &sprite_map),
                event_id,
                sprite_name: sprite_name.clone(),
                movement,
                is_hidden: x == 63 && y == 63,
                is_talking: false,
                script_played: false,
                is_scripting: false,
                pixel_x: px, pixel_y: py,
                target_x: px, target_y: py,
                travel_x: 0, travel_y: 0,
                anim_counter: (x as u8) % 20,
                frame: 0,
                idle_timer: (x as u16 * 13 + y as u16 * 7) % 60,
            });
        }
    }
    entities
}

fn parse_args(line: &str, macro_name: &str) -> Vec<String> {
    let raw = line.trim_start_matches(macro_name).trim();
    raw.split(',').map(|s| s.trim().to_string()).collect()
}

fn parse_facing(s: &str) -> u8 {
    match s { "UP" => 0, "DOWN" => 1, "LEFT" => 2, "RIGHT" => 3, _ => 1 }
}

fn resolve_sprite(name: &str, sprite_map: &HashMap<String, u8>) -> u8 {
    if name.starts_with("ALLY_") {
        let suffix = name.trim_start_matches("ALLY_");
        let map_name = format!("MAPSPRITE_{}_BASE", suffix);
        if let Some(&id) = sprite_map.get(&map_name) { return id; }
        match name {
            "ALLY_BOWIE" => 0, "ALLY_SARAH" => 2, "ALLY_CHESTER" => 5, 
            "ALLY_JAHA" => 8, "ALLY_KAZIN" => 11, _ => 0
        }
    } else {
        *sprite_map.get(name).unwrap_or(&195)
    }
}

pub fn tick_entities(
    entities: &mut Vec<MapEntity>, 
    layout: &[MapLayoutBlock], 
    player_x: i32,
    player_y: i32,
    frame_counter: u32
) {
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();
    occupied.insert((player_x, player_y));
    for e in entities.iter() {
        // Добавляем в "занятые" только тех, кто НЕ в скрипте и НЕ говорит
        if !e.is_hidden && !e.is_talking && !e.is_scripting {
            occupied.insert((e.x as i32, e.y as i32));
        }
    }

    for entity in entities.iter_mut() {
        if entity.is_hidden { continue; }

        // Если NPC в диалоге - крутим анимацию и пропускаем
        if entity.is_talking {
            entity.anim_counter = entity.anim_counter.wrapping_add(1);
            if entity.anim_counter >= 20 {
                entity.anim_counter = 0;
                entity.frame = if entity.frame == 0 { 1 } else { 0 };
            }
            continue;
        }

        let is_moving = entity.travel_x != 0 || entity.travel_y != 0;

        // 1. Плавное движение пикселей
        if is_moving {
            let speed = 16; 
            if entity.travel_x > 0 {
                entity.pixel_x += speed;
                if entity.pixel_x >= entity.target_x { 
                    entity.pixel_x = entity.target_x; 
                    entity.travel_x = 0; 
                    entity.x = (entity.pixel_x / 384) as u8;
                }
            } else if entity.travel_x < 0 {
                entity.pixel_x -= speed;
                if entity.pixel_x <= entity.target_x { 
                    entity.pixel_x = entity.target_x; 
                    entity.travel_x = 0; 
                    entity.x = (entity.pixel_x / 384) as u8;
                }
            }
            if entity.travel_y > 0 {
                entity.pixel_y += speed;
                if entity.pixel_y >= entity.target_y { 
                    entity.pixel_y = entity.target_y; 
                    entity.travel_y = 0; 
                    entity.y = (entity.pixel_y / 384) as u8;
                }
            } else if entity.travel_y < 0 {
                entity.pixel_y -= speed;
                if entity.pixel_y <= entity.target_y { 
                    entity.pixel_y = entity.target_y; 
                    entity.travel_y = 0; 
                    entity.y = (entity.pixel_y / 384) as u8;
                }
            }
        }

        // 2. Анимация
        let anim_speed = 1; 
        entity.anim_counter = entity.anim_counter.wrapping_add(anim_speed);
        if entity.anim_counter >= 20 {
            entity.anim_counter = 0;
            entity.frame = if entity.frame == 0 { 1 } else { 0 };
        }

        // 3. ИИ (НЕ РАБОТАЕТ, ЕСЛИ NPC В СКРИПТЕ)
        if !is_moving && !entity.is_scripting {
            if entity.idle_timer > 0 {
                entity.idle_timer -= 1;
            } else {
                if let EntityMovement::Walking { zone_x, zone_y, zone_size } = entity.movement {
                    // 1. Собираем все валидные направления
                    let mut valid_dirs = Vec::new();
                    for dir in 0..4 {
                        let mut nx = entity.x as i32;
                        let mut ny = entity.y as i32;
                        
                        match dir {
                            0 => { if entity.y > zone_y.saturating_sub(zone_size) { ny -= 1; } else { continue; } }
                            1 => { if entity.y < zone_y + zone_size { ny += 1; } else { continue; } }
                            2 => { if entity.x > zone_x.saturating_sub(zone_size) { nx -= 1; } else { continue; } }
                            _ => { if entity.x < zone_x + zone_size { nx += 1; } else { continue; } }
                        }
                        
                        let idx = (ny * 64 + nx) as usize;
                        let is_wall = idx < layout.len() && (layout[idx].flags & 0xC000) == 0xC000;
                        let is_occupied = occupied.contains(&(nx, ny)) && (nx != entity.x as i32 || ny != entity.y as i32);
                        
                        if !is_wall && !is_occupied {
                            valid_dirs.push((nx, ny, dir));
                        }
                    }
                    
                    // 2. Выбираем случайно из доступных
                    if !valid_dirs.is_empty() {
                        let rand_idx = (frame_counter.wrapping_add((entity.x as u32) * 17).wrapping_add((entity.y as u32) * 31)) as usize % valid_dirs.len();
                        let (nx, ny, dir) = valid_dirs[rand_idx];
                        
                        entity.facing = dir as u8;
                        entity.x = nx as u8;
                        entity.y = ny as u8;
                        entity.target_x = nx * 384;
                        entity.target_y = ny * 384;
                        entity.travel_x = entity.target_x - entity.pixel_x;
                        entity.travel_y = entity.target_y - entity.pixel_y;
                        
                        entity.idle_timer = 60; 
                    } else {
                        // Если все стороны заблокированы, просто стоим
                        entity.idle_timer = 30;
                        entity.facing = ((frame_counter / 30) % 4) as u8;
                    }
                }
            }
        }
    }
}