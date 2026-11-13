// src/game/map/finder.rs
use std::fs;
use std::collections::HashMap;
use crate::game::map::loader::{MapArea, StepEvent, RoofEvent, Warp};

pub struct MapPaths {
    pub map_name: String,
    pub blocks_path: String,
    pub layout_path: String,
    pub areas_path: String,
    pub step_events_path: String,
    pub roof_events_path: String,
    pub warp_events_path: String,
    pub chest_items_path: String,
    pub other_items_path: String,
    pub animations_path: String,
    pub tileset_paths: Vec<String>,
    pub palette_path: String,
    pub areas: Vec<MapArea>,
    pub step_events: Vec<StepEvent>,
    pub roof_events: Vec<RoofEvent>,
    pub warps: Vec<Warp>,
    pub anim_data: Option<(u8, usize, usize)>,
    pub chest_items: Vec<(u8, u8, u16, String)>, // x, y, flag, item_name
    pub other_items: Vec<(u8, u8, u16, String)>, // x, y, flag, item_name
    pub scripts_paths: Vec<String>,
    pub entity_events_paths: Vec<String>,
}

pub fn find_map_paths(map_id: u8, map_dict: &HashMap<String, u8>) -> MapPaths {
    let base_dir = "src/data/maps/";
    let map_prefix = format!("map{:02}", map_id);
    let map_label = format!("Map{:02}", map_id);

    // 1. Имя карты
    let names_content = fs::read_to_string(format!("{}map_names.txt", base_dir)).expect("Нет map_names.txt");
    let mut map_name = "UNKNOWN".to_string();
    for line in names_content.lines() {
        let clean = line.split(';').next().unwrap().trim();
        if clean.starts_with(&format!("{:02} ", map_id)) {
            map_name = clean.to_string();
            break;
        }
    }

    let mut paths = MapPaths {
        map_name,
        blocks_path: String::new(),
        layout_path: String::new(),
        areas_path: String::new(),
        step_events_path: String::new(),
        roof_events_path: String::new(),
        warp_events_path: String::new(),
        chest_items_path: String::new(),
        other_items_path: String::new(),
        animations_path: String::new(),
        tileset_paths: Vec::new(),
        palette_path: String::new(),
        areas: Vec::new(),
        step_events: Vec::new(),
        roof_events: Vec::new(),
        warps: Vec::new(),
        anim_data: None,
        chest_items: Vec::new(),
        other_items: Vec::new(),
        scripts_paths: Vec::new(),
        entity_events_paths: Vec::new(),
    };

    // 2. Ищем пути в entries.asm (ДИНАМИЧЕСКИ)
    let entries_content = fs::read_to_string(format!("{}entries.asm", base_dir)).expect("Нет entries.asm");
    
    let mut in_target_map = false;
    let mut dc_l_index = 0;

    for line in entries_content.lines() {
        let clean = line.split(';').next().unwrap().trim();

        // Находим начало нужной карты (например, "Map24:")
        if clean.starts_with(&format!("{}:", map_label)) {
            in_target_map = true;
            dc_l_index = 0;
            continue;
        }

        if in_target_map {
            // Если наткнулись на начало следующей карты (например "Map25:") — выходим
            if clean.starts_with("Map") && clean.ends_with(":") && !clean.contains("s") {
                break;
            }

            // Читаем строки dc.l
            if clean.starts_with("dc.l ") {
                let replaced = clean.replace("dc.l", "");
                let ptr_name = replaced.trim();
                
                // Если это не $FFFFFFFF, ищем этот указатель в файле
                if ptr_name != "$FFFFFFFF" && !ptr_name.is_empty() {
                    let target_label = format!("{}:", ptr_name);
                    
                    // Ищем реальный путь по этому указателю (например, Map23s0_Blocks:)
                    for sub_line in entries_content.lines() {
                        let sub_clean = sub_line.split(';').next().unwrap().trim();
                        if sub_clean.starts_with(&target_label) {
                            let path = extract_path(sub_clean).unwrap_or_default();
                            match dc_l_index {
                                0 => paths.blocks_path = path,
                                1 => paths.layout_path = path,
                                2 => paths.areas_path = path,
                                3 => {}, // FlagEvents
                                4 => paths.step_events_path = path,
                                5 => paths.roof_events_path = path,
                                6 => paths.warp_events_path = path,
                                7 => paths.chest_items_path = path,
                                8 => paths.other_items_path = path,
                                9 => paths.animations_path = path,
                                _ => {}
                            }
                            break;
                        }
                    }
                }
                dc_l_index += 1;
            }
        }
    }

    // 3. Тайлсеты и палитра
    let tilesets_asm = format!("src/data/maps/entries/{}/00-tilesets.asm", map_prefix);
    if let Ok(content) = fs::read_to_string(&tilesets_asm) {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mapPalette") {
                let id: u8 = clean.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
                let label = format!("MapPalette{:02}", id);
                paths.palette_path = find_in_registry("src/data/graphics/maps/mappalettes/entries.asm", &label).unwrap_or_default();
            } else if clean.starts_with("mapTileset") {
                let id: u8 = clean.split_whitespace().nth(1).unwrap_or("255").parse().unwrap_or(255);
                if id != 255 {
                    let label = format!("MapTileset{:03}", id);
                    let path = find_in_registry("src/data/graphics/maps/maptilesets/entries.asm", &label).unwrap_or_default();
                    paths.tileset_paths.push(path);
                } else {
                    paths.tileset_paths.push(String::new());
                }
            }
        }
    }
    while paths.tileset_paths.len() < 5 { paths.tileset_paths.push(String::new()); }

    // 4. Зоны
    let areas_asm = format!("src/data/maps/entries/{}/2-areas.asm", map_prefix);
    if let Ok(content) = fs::read_to_string(&areas_asm) {
        let mut start = (0, 0);
        let mut end = (0, 0);
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mainLayerStart") {
                let n: Vec<i32> = clean.replace("mainLayerStart", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { start = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("mainLayerEnd") {
                let n: Vec<i32> = clean.replace("mainLayerEnd", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { end = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("scndLayerFgndStart") {
                let n: Vec<i32> = clean.replace("scndLayerFgndStart", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { 
                    let scnd = (n[0] as u8, n[1] as u8);
                    // Сохраняем зону ТОЛЬКО когда прочитали все три параметра
                    paths.areas.push(MapArea { 
                        start_x: start.0, start_y: start.1, 
                        end_x: end.0, end_y: end.1,
                        scnd_x: scnd.0, scnd_y: scnd.1
                    }); 
                }
            }
        }
    }

    // 5. Двери
    let steps_asm = format!("src/data/maps/entries/{}/4-step-events.asm", map_prefix);
    if let Ok(content) = fs::read_to_string(&steps_asm) {
        let mut trig = (0, 0); let mut src = (0, 0); let mut size = (0, 0);
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("sbc ") {
                let n: Vec<i32> = clean.replace("sbc", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { trig = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("sbcSource") {
                let n: Vec<i32> = clean.replace("sbcSource", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { src = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("sbcSize") {
                let n: Vec<i32> = clean.replace("sbcSize", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { size = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("sbcDest") {
                let n: Vec<i32> = clean.replace("sbcDest", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { paths.step_events.push(StepEvent { trig_x: trig.0, trig_y: trig.1, src_x: src.0, src_y: src.1, width: size.0, height: size.1, dest_x: n[0] as u8, dest_y: n[1] as u8 }); }
            }
        }
    }

    // 6. Крыши
    let roofs_asm = format!("src/data/maps/entries/{}/5-roof-events.asm", map_prefix);
    if let Ok(content) = fs::read_to_string(&roofs_asm) {
        let mut trig = (0, 0); let mut src = (0, 0); let mut size = (0, 0);
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("slbc ") {
                let n: Vec<i32> = clean.replace("slbc", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { trig = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("slbcSource") {
                let n: Vec<i32> = clean.replace("slbcSource", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { src = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("slbcSize") {
                let n: Vec<i32> = clean.replace("slbcSize", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { size = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("slbcDest") {
                let n: Vec<i32> = clean.replace("slbcDest", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { paths.roof_events.push(RoofEvent { trig_x: trig.0, trig_y: trig.1, src_x: src.0, src_y: src.1, width: size.0, height: size.1, dest_x: n[0] as u8, dest_y: n[1] as u8 }); }
            }
        }
    }

    // 7. Варпы
    let warps_asm = format!("src/data/maps/entries/{}/6-warp-events.asm", map_prefix);
    if let Ok(content) = fs::read_to_string(&warps_asm) {
        let mut current_warp: Option<Warp> = None;
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mWarp") {
                if let Some(w) = current_warp.take() { paths.warps.push(w); }
                let n: Vec<i32> = clean.replace("mWarp", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { current_warp = Some(Warp { src_x: n[0] as u8, src_y: n[1] as u8, dest_map: 0, dest_x: 0, dest_y: 0, facing: 0 }); }
            } else if clean.starts_with("warpMap") {
                if let Some(ref mut w) = current_warp {
                    let name = clean.replace("warpMap", "").trim().to_string();
                    if name == "MAP_CURRENT" { w.dest_map = 255; }
                    else if let Some(&id) = map_dict.get(&name) { w.dest_map = id; }
                }
            } else if clean.starts_with("warpDest") {
                if let Some(ref mut w) = current_warp {
                    let n: Vec<i32> = clean.replace("warpDest", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                    if n.len() >= 2 { w.dest_x = n[0] as u8; w.dest_y = n[1] as u8; }
                }
            } else if clean.starts_with("warpFacing") {
                if let Some(ref mut w) = current_warp {
                    let f = clean.replace("warpFacing", "").trim().to_string();
                    w.facing = match f.as_str() { "UP" => 0, "DOWN" => 1, "LEFT" => 2, "RIGHT" => 3, _ => 0 };
                }
            }
        }
        if let Some(w) = current_warp { paths.warps.push(w); }
    }

    // 8. Анимации
    if !paths.animations_path.is_empty() {
        if let Ok(content) = fs::read_to_string(format!("src/{}", paths.animations_path)) {
            let mut src_id = 0;
            let mut dest_addr = 0;
            let mut length = 0;
            for line in content.lines() {
                let clean = line.split(';').next().unwrap().trim();
                if clean.starts_with("mapAnimation") {
                    let n: Vec<&str> = clean.split_whitespace().collect();
                    if n.len() >= 2 { src_id = n[1].trim_end_matches(',').parse().unwrap_or(0); }
                } else if clean.starts_with("mapAnimEntry") {
                    let n: Vec<&str> = clean.split_whitespace().collect();
                    if n.len() >= 4 {
                        length = n[2].trim_end_matches(',').parse().unwrap_or(0);
                        let dest_str = n[3].trim_end_matches(',');
                        if dest_str.starts_with('$') { dest_addr = usize::from_str_radix(&dest_str[1..], 16).unwrap_or(0); }
                    }
                    break; 
                }
            }
            if src_id > 0 && length > 0 {
                let dest_offset = dest_addr.saturating_sub(0x300);
                paths.anim_data = Some((src_id, dest_offset, length));
            }
        }
    }

    // 9. Сундуки (Chest Items)
    if !paths.chest_items_path.is_empty() {
        if let Ok(content) = fs::read_to_string(format!("src/{}", paths.chest_items_path)) {
            for line in content.lines() {
                let clean = line.split(';').next().unwrap().trim();
                  if clean.starts_with("mapItem ") {
                    let stripped = clean.replace("mapItem ", "");
                    let args: Vec<&str> = stripped.split(',').map(|s| s.trim()).collect();
                    if args.len() >= 4 {
                        let x = args[0].parse().unwrap_or(0);
                        let y = args[1].parse().unwrap_or(0);
                        let flag = args[2].parse().unwrap_or(0);
                        let item_name = args[3].to_string();
                        paths.chest_items.push((x, y, flag, item_name));
                    }
                }
            }
        }
    }

    // 10. Предметы на земле (Other Items)
    if !paths.other_items_path.is_empty() {
        if let Ok(content) = fs::read_to_string(format!("src/{}", paths.other_items_path)) {
            for line in content.lines() {
                let clean = line.split(';').next().unwrap().trim();
                if clean.starts_with("mapItem ") {
                    let stripped = clean.replace("mapItem ", "");
                    let args: Vec<&str> = stripped.split(',').map(|s| s.trim()).collect();
                    if args.len() >= 4 {
                        let x = args[0].parse().unwrap_or(0);
                        let y = args[1].parse().unwrap_or(0);
                        let flag = args[2].parse().unwrap_or(0);
                        let item_name = args[3].to_string();
                        paths.other_items.push((x, y, flag, item_name));
                    }
                }
            }
        }
    }

    // 11. Парсим mapsetupsstorage.asm (точные пути ко всем скриптам и событиям)
    if let Ok(content) = fs::read_to_string("src/data/maps/mapsetupsstorage.asm") {
        for line in content.lines() {
            let clean = line.trim();
            // Проверяем и с обратными, и с прямыми слешами
            let search_str_1 = format!("entries\\map{:02}\\mapsetups\\", map_id);
            let search_str_2 = format!("entries/map{:02}/mapsetups/", map_id);
            
            if clean.contains(&search_str_1) || clean.contains(&search_str_2) {
                if let Some(start) = clean.find('"') {
                    if let Some(end) = clean[start+1..].find('"') {
                        let filepath = clean[start+1..start+1+end].replace('\\', "/");
                        
                        if filepath.contains("scripts") && filepath.ends_with(".asm") {
                            paths.scripts_paths.push(filepath); 
                        } 
                        // БЕРЕМ ТОЛЬКО s2_entityevents.asm (без _609, _506 и т.д.)
                        else if filepath.ends_with("s2_entityevents.asm") {
                            paths.entity_events_paths.push(filepath); 
                        }
                    }
                }
            }
        }
    }

    paths
}

fn find_in_registry(registry_path: &str, target_label: &str) -> Option<String> {
    if let Ok(content) = fs::read_to_string(registry_path) {
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with(&format!("{}:", target_label)) {
                return extract_path(clean);
            }
        }
    }
    None
}

fn extract_path(line: &str) -> Option<String> {
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start+1..].find('"') {
            return Some(line[start+1..start+1+end].replace('\\', "/"));
        }
    }
    None
}