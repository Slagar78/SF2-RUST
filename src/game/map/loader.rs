// src/game/map/loader.rs
use sdl3::pixels::Color;
use std::collections::HashMap;
use crate::engine::decompressor;
use crate::game::map::anim::MapAnimation;

const LAYOUT_SIZE: usize = 0x1000;
const CRAM_VALUE_MAP: [u8; 8] = [0, 52, 87, 116, 144, 172, 206, 255];

fn decode_genesis_color(first: u8, second: u8) -> Color {
    let word = u16::from_be_bytes([first, second]);
    let r3 = ((word >> 1) & 0x07) as usize;
    let g3 = ((word >> 5) & 0x07) as usize;
    let b3 = ((word >> 9) & 0x07) as usize;
    Color::RGB(CRAM_VALUE_MAP[r3], CRAM_VALUE_MAP[g3], CRAM_VALUE_MAP[b3])
}

#[derive(Clone, Copy, Debug)]
pub struct MapLayoutBlock {
    pub block_index: u16,
    pub flags: u16,
}

#[derive(Clone, Copy)]
pub struct Warp {
    pub src_x: u8, pub src_y: u8,
    pub dest_map: u8, pub dest_x: u8, pub dest_y: u8, pub facing: u8,
}

#[derive(Clone, Copy)]
pub struct StepEvent {
    pub trig_x: u8, pub trig_y: u8,
    pub src_x: u8, pub src_y: u8,
    pub width: u8, pub height: u8,
    pub dest_x: u8, pub dest_y: u8,
}

#[derive(Clone, Copy)]
pub struct RoofEvent {
    pub trig_x: u8, pub trig_y: u8,
    pub src_x: u8, pub src_y: u8,
    pub width: u8, pub height: u8,
    pub dest_x: u8, pub dest_y: u8,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct MapItem {
    pub x: u8, pub y: u8, pub flag: u16, pub item_id: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct MapArea {
    pub start_x: u8, pub start_y: u8,
    pub end_x: u8, pub end_y: u8,
}

pub struct MapData {
    pub layout: Vec<MapLayoutBlock>,
    pub blockset: Vec<u16>,
    pub tilesets: Vec<Vec<u8>>,
    pub palette: Vec<Color>,
    pub warps: Vec<Warp>,
    pub step_events: Vec<StepEvent>,
    pub items: Vec<MapItem>,
    pub roof_events: Vec<RoofEvent>,
    pub areas: Vec<MapArea>,
    pub animations: Vec<MapAnimation>,
}

pub fn load_map_data() -> MapData {
    println!("Загрузка карты 3 (Granseal)...");

    let blocks_raw = std::fs::read("src/data/maps/entries/map03/0-blocks.bin").expect("Нет 0-blocks.bin");
    let blockset = map_block_subroutine(&blocks_raw);
    let layout_raw = std::fs::read("src/data/maps/entries/map03/1-layout.bin").expect("Нет 1-layout.bin");
    let layout_words = map_layout_subroutine(&layout_raw);
    let layout: Vec<MapLayoutBlock> = layout_words.iter().map(|&w| MapLayoutBlock { block_index: w & 0x3ff, flags: w & 0xfc00 }).collect();

    let mut tilesets = Vec::new();
    for id in [0u32, 37, 43, 53, 66, 74] { // <-- Добавили 74-й
        let path = format!("src/data/graphics/maps/maptilesets/maptileset{:03}.bin", id);
        let comp = std::fs::read(&path).unwrap_or_else(|_| panic!("Нет {}", path));
        tilesets.push(decompressor::decompress(&comp).unwrap());
    }

    // ЧИТАЕМ ПАЛИТРУ ИЗ ФАЙЛА (больше никаких хардкодов)
    let palette_path = "src/data/graphics/maps/mappalettes/mappalette00.bin";
    let pal_data = std::fs::read(palette_path)
        .unwrap_or_else(|_| panic!("Файл {} не найден", palette_path));

    let mut palette = Vec::new();
    for i in 0..16 {
        palette.push(decode_genesis_color(pal_data[i * 2], pal_data[i * 2 + 1]));
    }

    let warps = vec![
        Warp { src_x: 255, src_y: 1, dest_map: 19, dest_x: 26, dest_y: 30, facing: 0 },
        Warp { src_x: 0, src_y: 255, dest_map: 66, dest_x: 29, dest_y: 32, facing: 1 },
        Warp { src_x: 50, src_y: 23, dest_map: 44, dest_x: 1, dest_y: 25, facing: 3 },
        Warp { src_x: 50, src_y: 24, dest_map: 44, dest_x: 1, dest_y: 25, facing: 3 },
        Warp { src_x: 50, src_y: 25, dest_map: 44, dest_x: 1, dest_y: 25, facing: 3 },
        Warp { src_x: 46, src_y: 7, dest_map: 3, dest_x: 59, dest_y: 12, facing: 2 },
        Warp { src_x: 59, src_y: 12, dest_map: 3, dest_x: 46, dest_y: 7, facing: 1 },
        Warp { src_x: 3, src_y: 3, dest_map: 3, dest_x: 54, dest_y: 3, facing: 3 },
        Warp { src_x: 54, src_y: 3, dest_map: 3, dest_x: 3, dest_y: 3, facing: 3 },
    ];

    let step_events = vec![
        StepEvent { trig_x: 4, trig_y: 8, src_x: 62, src_y: 0, width: 1, height: 1, dest_x: 4, dest_y: 8 },
        StepEvent { trig_x: 12, trig_y: 12, src_x: 62, src_y: 0, width: 1, height: 1, dest_x: 12, dest_y: 12 },
        StepEvent { trig_x: 19, trig_y: 12, src_x: 62, src_y: 0, width: 1, height: 1, dest_x: 19, dest_y: 12 },
        StepEvent { trig_x: 32, trig_y: 15, src_x: 62, src_y: 0, width: 1, height: 1, dest_x: 32, dest_y: 15 },
        StepEvent { trig_x: 38, trig_y: 24, src_x: 62, src_y: 0, width: 1, height: 1, dest_x: 38, dest_y: 24 },
        StepEvent { trig_x: 41, trig_y: 13, src_x: 62, src_y: 0, width: 1, height: 1, dest_x: 41, dest_y: 13 },
    ];

    let roof_events = vec![
        RoofEvent { trig_x: 4, trig_y: 8, src_x: 255, src_y: 255, width: 7, height: 8, dest_x: 2, dest_y: 32 },
        RoofEvent { trig_x: 7, trig_y: 22, src_x: 255, src_y: 255, width: 6, height: 6, dest_x: 5, dest_y: 48 },
        RoofEvent { trig_x: 8, trig_y: 22, src_x: 255, src_y: 255, width: 6, height: 6, dest_x: 5, dest_y: 48 },
        RoofEvent { trig_x: 12, trig_y: 12, src_x: 255, src_y: 255, width: 6, height: 6, dest_x: 10, dest_y: 38 },
        RoofEvent { trig_x: 19, trig_y: 12, src_x: 255, src_y: 255, width: 6, height: 5, dest_x: 17, dest_y: 39 },
        RoofEvent { trig_x: 24, trig_y: 26, src_x: 51, src_y: 20, width: 9, height: 7, dest_x: 22, dest_y: 51 },
        RoofEvent { trig_x: 25, trig_y: 26, src_x: 51, src_y: 20, width: 9, height: 7, dest_x: 22, dest_y: 51 },
        RoofEvent { trig_x: 32, trig_y: 15, src_x: 255, src_y: 255, width: 5, height: 6, dest_x: 30, dest_y: 41 },
        RoofEvent { trig_x: 38, trig_y: 24, src_x: 255, src_y: 255, width: 5, height: 5, dest_x: 36, dest_y: 51 },
        RoofEvent { trig_x: 41, trig_y: 13, src_x: 255, src_y: 255, width: 9, height: 8, dest_x: 39, dest_y: 37 },
    ];

    let items = vec![
        MapItem { x: 6, y: 18, flag: 220, item_id: 0 },
        MapItem { x: 59, y: 17, flag: 128, item_id: 1 },
    ];

    // Зоны из 2-areas.asm
    let areas = vec![
        // mainLayerStart 0, 0 / mainLayerEnd 50, 31
        MapArea { start_x: 0, start_y: 0, end_x: 50, end_y: 31 },
        // mainLayerStart 51, 0 / mainLayerEnd 61, 9
        MapArea { start_x: 51, start_y: 0, end_x: 61, end_y: 9 },
        // mainLayerStart 51, 10 / mainLayerEnd 61, 19
        MapArea { start_x: 51, start_y: 10, end_x: 61, end_y: 19 },
    ];

    let animations = vec![
        MapAnimation {
            timer: 20,
            is_frame2: false,
        }
    ];

    MapData { layout, blockset, tilesets, palette, warps, step_events, items, roof_events, areas, animations }

}

struct BitStream<'a> {
    data: &'a [u8],
    pos: usize,
    barrel: Vec<u16>,
}

impl<'a> BitStream<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, pos: 0, barrel: Vec::new() } }
    fn refill(&mut self) -> bool {
        if self.pos + 1 >= self.data.len() { return false; }
        let word = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        for i in (0..16).rev() { self.barrel.push((word >> i) & 1); }
        true
    }
    fn next_bit(&mut self) -> Option<u16> {
        if self.barrel.is_empty() && !self.refill() { return None; }
        Some(self.barrel.remove(0))
    }
    fn next_bits(&mut self, n: usize) -> Option<u16> {
        let mut v = 0u16;
        for _ in 0..n { v = (v << 1) | self.next_bit()?; }
        Some(v)
    }
}

fn map_block_subroutine(data: &[u8]) -> Vec<u16> {
    let mut stack: Vec<u16> = vec![0x0100; 9];
    stack.extend_from_slice(&[
        0x032e, 0x032f, 0x0b2e, 0x033e, 0x033f, 0x0b3e, 0x034e, 0x034f, 0x0b4e,
        0x032c, 0x032d, 0x0b2c, 0x033c, 0x033d, 0x0b3c, 0x034e, 0x034f, 0x0b4e,
    ]);
    if data.len() < 2 { return stack; }
    let cmds = u16::from_be_bytes([data[0], data[1]]);
    let num_cmds = (cmds >> 2) & 0x3FFF;
    let mut stream = BitStream::new(data);
    stream.barrel.push((cmds >> 1) & 1);
    stream.barrel.push(cmds & 1);
    stream.pos = 2;
    let mut sets: HashMap<u32, u16> = HashMap::new();
    let mut cmd_str = String::new();

    for _ in 0..num_cmds {
        loop {
            let bit = match stream.next_bit() { Some(b) => b, None => return stack };
            cmd_str.push(if bit == 0 { '0' } else { '1' });
            if matches!(cmd_str.as_str(), "00" | "01" | "100" | "101" | "110" | "111") { break; }
        }
        match cmd_str.as_str() {
            "00" => stack.push(*stack.last().unwrap()),
            "01" => {
                let last = *stack.last().unwrap();
                if last & 0x0800 != 0 { stack.push(last.wrapping_sub(1)); }
                else { stack.push(last.wrapping_add(1)); }
            }
            "100" => {
                let last = *stack.last().unwrap();
                let ofs = ((last & 0x03ff) as u32) * 2 + (last & 0x0800) as u32;
                stack.push(*sets.get(&ofs).unwrap_or(&0));
            }
            "101" => {
                let third = stack[stack.len() - 3];
                let ofs = ((third & 0x03ff) as u32) * 2 + (third & 0x0800) as u32 + 0x1000;
                stack.push(*sets.get(&ofs).unwrap_or(&0));
            }
            "110" | "111" => {
                let top: u16 = if cmd_str == "110" { *stack.last().unwrap() & 0x9800 } else {
                    let b0 = stream.next_bit().unwrap_or(0); let b1 = stream.next_bit().unwrap_or(0); let b2 = stream.next_bit().unwrap_or(0);
                    (b0 << 15) | (b1 << 12) | (b2 << 11)
                };
                let flag = stream.next_bit().unwrap_or(0);
                let bottom: u16 = if flag == 0 {
                    let val = stream.next_bits(5).unwrap_or(0) as i32;
                    let negate = stream.next_bit().unwrap_or(0);
                    let last = *stack.last().unwrap();
                    ((if negate == 1 { -val } else { val } + (last & 0x07ff) as i32) & 0xFFFF) as u16
                } else {
                    let mut val = stream.next_bits(9).unwrap_or(0);
                    if val >= 384 {
                        let extra = stream.next_bit().unwrap_or(0);
                        val = (val << 1) | extra;
                        val -= 384;
                    }
                    val.wrapping_add(256)
                };
                let full = bottom | top;
                let last = *stack.last().unwrap();
                let ofs1 = ((last & 0x03ff) as u32) * 2 + (last & 0x0800) as u32;
                let third = stack[stack.len() - 3];
                let ofs2 = ((third & 0x03ff) as u32) * 2 + (third & 0x0800) as u32 + 0x1000;
                sets.insert(ofs1, full);
                sets.insert(ofs2, full);
                stack.push(full);
            }
            _ => unreachable!(),
        }
        cmd_str.clear();
    }
    stack
}

fn map_layout_subroutine(data: &[u8]) -> Vec<u16> {
    let mut stream = BitStream::new(data);
    let mut mapdata: Vec<u16> = Vec::new();
    let mut highest: u16 = 2;
    let mut lminis: HashMap<u16, Vec<u16>> = HashMap::new();
    let mut uminis: HashMap<u16, Vec<u16>> = HashMap::new();
    let mut cmd = String::new();
    let mut safety = 0usize;

    while mapdata.len() < LAYOUT_SIZE {
        safety += 1;
        if safety > 200_000 { break; }
        let mut next = true;
        let mut miniadd = true;
        let mut blk: u16 = 0;
        let bit = match stream.next_bit() { Some(b) => b, None => break };
        cmd.push(if bit == 0 { '0' } else { '1' });

        match cmd.as_str() {
            "00" => {
                highest += 1; blk = highest;
                let b = stream.next_bit().unwrap_or(0); cmd.push(if b == 0 { '0' } else { '1' });
                if cmd.ends_with('0') {
                    let b = stream.next_bit().unwrap_or(0); cmd.push(if b == 0 { '0' } else { '1' });
                    if cmd.ends_with('1') { blk |= 0xc000; }
                } else {
                    let b = stream.next_bit().unwrap_or(0); cmd.push(if b == 0 { '0' } else { '1' });
                    if cmd.ends_with('0') {
                        let b = stream.next_bit().unwrap_or(0); cmd.push(if b == 0 { '0' } else { '1' });
                        let last = cmd.chars().last().unwrap().to_digit(10).unwrap() as u16;
                        blk |= (1 + last) * 0x4000;
                    } else {
                        let top = stream.next_bits(6).unwrap_or(0);
                        blk |= top.wrapping_shl(10);
                    }
                }
                mapdata.push(blk);
            }
            "01" => {
                let mut cnt = 0usize;
                loop {
                    if cmd == "01" || cmd.ends_with('0') {
                        let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                        cmd.push(if b == 0 { '0' } else { '1' });
                        cnt += 1; if cnt > 20 { break; }
                    } else { break; }
                }
                if cnt == 0 { return mapdata; }
                cnt -= 1;
                let mcnt = cnt;
                let mut num: u32 = 0;
                for _ in 0..cnt {
                    let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                    num = (num << 1) | b as u32;
                }
                num = num.saturating_add(1u32 << mcnt);
                let typ = match stream.next_bit() { Some(b) => b, None => return mapdata };
                for _ in 0..num {
                    if mapdata.len() >= LAYOUT_SIZE { break; }
                    let off = if typ == 0 { 0x40 } else { 1 };
                    let src = mapdata.len();
                    let v = if src >= off { mapdata[src - off] } else { 0 };
                    mapdata.push(v);
                }
                miniadd = false;
            }
            "1" => {
                let mut custom_ovr = if mapdata.is_empty() { true }
                else if mapdata.len() >= 64 {
                    let last = mapdata[mapdata.len() - 1] & 0x3ff;
                    let upper = mapdata[mapdata.len() - 64] & 0x3ff;
                    !lminis.contains_key(&last) && !uminis.contains_key(&upper)
                } else { false };

                if !custom_ovr {
                    let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                    cmd.push(if b == 0 { '0' } else { '1' });
                    if cmd.ends_with('1') && !mapdata.is_empty() {
                        let last = mapdata[mapdata.len() - 1] & 0x3ff;
                        if lminis.contains_key(&last) {
                            if mapdata.len() >= 64 {
                                let upper = mapdata[mapdata.len() - 64] & 0x3ff;
                                if !uminis.contains_key(&upper) { custom_ovr = true; }
                                else { let b = match stream.next_bit() { Some(b) => b, None => return mapdata }; cmd.push(if b == 0 { '0' } else { '1' }); }
                            } else { let b = match stream.next_bit() { Some(b) => b, None => return mapdata }; cmd.push(if b == 0 { '0' } else { '1' }); }
                        }
                    }
                }

                if cmd.ends_with('0') && !custom_ovr {
                    let last = if !mapdata.is_empty() { mapdata[mapdata.len() - 1] & 0x3ff } else { 0 };
                    let upper = if mapdata.len() >= 64 { mapdata[mapdata.len() - 64] & 0x3ff } else { 0 };
                    let lst: Vec<u16>;
                    if mapdata.len() > 1 && lminis.contains_key(&last) && cmd.len() < 3 { lst = lminis[&last].clone(); }
                    else if mapdata.len() > 64 && uminis.contains_key(&upper) { lst = uminis[&upper].clone(); }
                    else { lst = uminis.get(&0).cloned().unwrap_or_else(|| vec![0]); }

                    blk = if lst.len() == 1 { lst[0] }
                    else if lst.is_empty() { 0 }
                    else {
                        let mut cnt = 0usize;
                        loop {
                            if !cmd.ends_with('1') && cnt < lst.len() - 1 {
                                let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                                cmd.push(if b == 0 { '0' } else { '1' });
                                cnt += 1;
                            } else { break; }
                        }
                        let last_bit = cmd.chars().last().unwrap().to_digit(10).unwrap() as i32;
                        let idx = cnt as i32 - last_bit;
                        let real = if idx < 0 { lst.len() as i32 + idx } else { idx };
                        lst[real as usize]
                    };
                    mapdata.push(blk);
                } else {
                    let start = cmd.len();
                    let mut cnt = highest;
                    while cnt != 0 {
                        let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                        cmd.push(if b == 0 { '0' } else { '1' });
                        cnt /= 2;
                    }
                    blk = u16::from_str_radix(&cmd[start..], 2).unwrap_or(0);
                    let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                    cmd.push(if b == 0 { '0' } else { '1' });

                    if cmd.ends_with('0') {
                        let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                        cmd.push(if b == 0 { '0' } else { '1' });
                        if cmd.ends_with('1') { blk |= 0xc000; }
                    } else {
                        let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                        cmd.push(if b == 0 { '0' } else { '1' });
                        if cmd.ends_with('0') {
                            let b = match stream.next_bit() { Some(b) => b, None => return mapdata };
                            cmd.push(if b == 0 { '0' } else { '1' });
                            let last = cmd.chars().last().unwrap().to_digit(10).unwrap() as u16;
                            blk |= (1 + last) * 0x4000;
                        } else {
                            let top = stream.next_bits(6).unwrap_or(0);
                            blk |= top.wrapping_shl(10);
                        }
                    }
                    mapdata.push(blk);
                }
            }
            _ => { next = false; }
        }

        if next {
            if miniadd && !mapdata.is_empty() {
                let last = if mapdata.len() > 1 { mapdata[mapdata.len() - 2] & 0x3ff } else { 0 };
                let entry = lminis.entry(last).or_insert_with(Vec::new);
                if entry.is_empty() || entry[0] != blk {
                    entry.insert(0, blk);
                    if entry.iter().filter(|&&x| x == blk).count() > 1 {
                        let mut found = false;
                        entry.retain(|&x| { if x == blk { if found { false } else { found = true; true } } else { true } });
                    }
                    entry.truncate(4);
                }
                let upper = if mapdata.len() > 64 { mapdata[mapdata.len() - 65] & 0x3ff } else { 0 };
                let entry = uminis.entry(upper).or_insert_with(Vec::new);
                if entry.is_empty() || entry[0] != blk {
                    entry.insert(0, blk);
                    if entry.iter().filter(|&&x| x == blk).count() > 1 {
                        let mut found = false;
                        entry.retain(|&x| { if x == blk { if found { false } else { found = true; true } } else { true } });
                    }
                    entry.truncate(4);
                }
            }
            cmd.clear();
        }
    }
    while mapdata.len() < LAYOUT_SIZE { mapdata.push(0); }
    mapdata.truncate(LAYOUT_SIZE);
    mapdata
}