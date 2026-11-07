// src/game/map/finder.rs
use std::fs;
use std::collections::HashMap;
use crate::game::map::loader::{MapArea, StepEvent, RoofEvent, Warp}; // <--- Добавлен Warp

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
    pub warps: Vec<Warp>, // <--- ДОБАВЛЕНО
}

pub fn find_map_paths(map_id: u8, map_dict: &HashMap<String, u8>) -> MapPaths {
    let base_dir = "src/data/maps/";
    let map_prefix = format!("map{:02}", map_id);
    let map_label = format!("Map{:02}", map_id);

    // 1. Ищем имя карты в map_names.txt
    let names_path = format!("{}map_names.txt", base_dir);
    let names_content = fs::read_to_string(&names_path).expect("Нет map_names.txt");
    let mut map_name = "UNKNOWN".to_string();

    for line in names_content.lines() {
        let clean = line.split(';').next().unwrap().trim();
        if clean.starts_with(&format!("{:02} ", map_id)) {
            map_name = clean.to_string();
            break;
        }
    }

    // 2. Читаем entries.asm и ищем пути для этой карты
    let entries_path = format!("{}entries.asm", base_dir);
    let entries_content = fs::read_to_string(&entries_path).expect("Нет entries.asm");

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
        warps: Vec::new(), // <--- ДОБАВЛЕНО
    };

    let mut in_map_block = false;

    for line in entries_content.lines() {
        let clean = line.split(';').next().unwrap().trim();

        if clean.starts_with(&format!("{}:", map_label)) {
            in_map_block = true;
            continue;
        }

        if in_map_block && clean.starts_with("Map") && clean.contains(":") && !clean.starts_with(&map_label) {
            break;
        }

        if in_map_block {
            if clean.starts_with(&format!("{}s0_Blocks:", map_label)) { paths.blocks_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s1_Layout:", map_label)) { paths.layout_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s2_Areas:", map_label)) { paths.areas_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s4_StepEvents:", map_label)) { paths.step_events_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s5_RoofEvents:", map_label)) { paths.roof_events_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s6_WarpEvents:", map_label)) { paths.warp_events_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s7_ChestItems:", map_label)) { paths.chest_items_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s8_OtherItems:", map_label)) { paths.other_items_path = extract_path(clean).unwrap_or_default(); }
            if clean.starts_with(&format!("{}s9_Animations:", map_label)) { paths.animations_path = extract_path(clean).unwrap_or_default(); }
        }
    }

    // 3. Читаем 00-tilesets.asm
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
                if id == 255 {
                    paths.tileset_paths.push(String::new());
                } else {
                    let label = format!("MapTileset{:03}", id);
                    let path = find_in_registry("src/data/graphics/maps/maptilesets/entries.asm", &label).unwrap_or_default();
                    paths.tileset_paths.push(path);
                }
            }
        }
    }

    while paths.tileset_paths.len() < 5 {
        paths.tileset_paths.push(String::new());
    }

    // 4. Читаем ЗОНЫ
    let areas_asm = format!("src/data/maps/entries/{}/2-areas.asm", map_prefix);
    if let Ok(content) = fs::read_to_string(&areas_asm) {
        let mut start = (0, 0);
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mainLayerStart") {
                let n: Vec<i32> = clean.replace("mainLayerStart", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { start = (n[0] as u8, n[1] as u8); }
            } else if clean.starts_with("mainLayerEnd") {
                let n: Vec<i32> = clean.replace("mainLayerEnd", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 { 
                    paths.areas.push(MapArea { start_x: start.0, start_y: start.1, end_x: n[0] as u8, end_y: n[1] as u8 });
                }
            }
        }
    }

    // 5. Читаем ДВЕРИ
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
                if n.len() >= 2 {
                    paths.step_events.push(StepEvent { trig_x: trig.0, trig_y: trig.1, src_x: src.0, src_y: src.1, width: size.0, height: size.1, dest_x: n[0] as u8, dest_y: n[1] as u8 });
                }
            }
        }
    }

    // 6. Читаем КРЫШИ
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
                if n.len() >= 2 {
                    paths.roof_events.push(RoofEvent { trig_x: trig.0, trig_y: trig.1, src_x: src.0, src_y: src.1, width: size.0, height: size.1, dest_x: n[0] as u8, dest_y: n[1] as u8 });
                }
            }
        }
    }

    // 7. Читаем ВАРПЫ из 6-warp-events.asm
    let warps_asm = format!("src/data/maps/entries/{}/6-warp-events.asm", map_prefix);
    if let Ok(content) = fs::read_to_string(&warps_asm) {
        let mut current_warp: Option<Warp> = None;
        for line in content.lines() {
            let clean = line.split(';').next().unwrap().trim();
            if clean.starts_with("mWarp") {
                if let Some(w) = current_warp.take() { paths.warps.push(w); }
                let n: Vec<i32> = clean.replace("mWarp", "").split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 {
                    current_warp = Some(Warp { src_x: n[0] as u8, src_y: n[1] as u8, dest_map: 0, dest_x: 0, dest_y: 0, facing: 0 });
                }
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