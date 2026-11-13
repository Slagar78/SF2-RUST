// src/game/menu/portrait.rs
use sdl3::pixels::Color;
use crate::engine::decompressor;

const CRAM_VALUE_MAP: [u8; 8] = [0, 52, 87, 116, 144, 172, 206, 255];

fn decode_genesis_color(first: u8, second: u8) -> Color {
    let word = u16::from_be_bytes([first, second]);
    let r3 = ((word >> 1) & 0x07) as usize;
    let g3 = ((word >> 5) & 0x07) as usize;
    let b3 = ((word >> 9) & 0x07) as usize;
    Color::RGB(CRAM_VALUE_MAP[r3], CRAM_VALUE_MAP[g3], CRAM_VALUE_MAP[b3])
}

#[derive(Clone, Copy)]
pub struct EyeEntry {
    pub x: u8,
    pub y: u8,
    pub alt_x: u8,
    pub alt_y: u8,
}

#[derive(Clone)]
pub struct PortraitData  {
    pub tiles: Vec<u8>,
    pub palette: Vec<Color>,
    pub eyes: Vec<EyeEntry>,
    pub mouth: Vec<EyeEntry>, // <-- ДОБАВЛЕНО
}

pub fn load_portrait(index: usize) -> Option<PortraitData> {
    let path = format!("src/data/graphics/portraits/portrait{:02}.bin", index);
    let data = std::fs::read(&path).ok()?;

    let mut pos = 0;
    // 1. Читаем данные глаз
    let eye_count = u16::from_be_bytes([data[pos], data[pos+1]]) as usize;
    pos += 2;
    
    let mut eyes = Vec::new();
    for _ in 0..eye_count {
        let x = data[pos];
        let y = data[pos+1];
        let alt_x = data[pos+2];
        let alt_y = data[pos+3];
        eyes.push(EyeEntry { x, y, alt_x, alt_y });
        pos += 4;
    }
    
    // 2. Читаем данные рта (ВМЕСТО ПРОПУСКА)
    let mouth_count = u16::from_be_bytes([data[pos], data[pos+1]]) as usize;
    pos += 2;
    
    let mut mouth = Vec::new();
    for _ in 0..mouth_count {
        let x = data[pos];
        let y = data[pos+1];
        let alt_x = data[pos+2];
        let alt_y = data[pos+3];
        mouth.push(EyeEntry { x, y, alt_x, alt_y });
        pos += 4;
    }

    // 3. Читаем палитру (16 цветов = 32 байта)
    let mut palette = Vec::new();
    for _ in 0..16 {
        palette.push(decode_genesis_color(data[pos], data[pos+1]));
        pos += 2;
    }

    // 4. Распаковываем тайлы (остаток файла)
    let comp_tiles = &data[pos..];
    let tiles = decompressor::decompress(comp_tiles).unwrap_or_default();

    Some(PortraitData { tiles, palette, eyes, mouth })
}