// src/engine/font.rs
use std::fs;
use std::collections::HashMap;
use sdl3::pixels::Color;

pub struct FontChar {
    pub width: u8,
    pub pixels: [[bool; 12]; 15],
}

pub struct FontManager {
    pub chars: Vec<FontChar>,
    pub ascii_map: HashMap<char, usize>,
}

impl FontManager {
    pub fn new() -> Self {
        let mut chars = Vec::new();
        
        // 1. Загружаем шрифт
        if let Ok(data) = fs::read("src/data/graphics/tech/fonts/variablewidthfont.bin") {
            for chunk in data.chunks(32) {
                if chunk.len() < 32 { break; }
                
                let width = chunk[1] & 0x0F;
                let mut pixels = [[false; 12]; 15];
                
                for row in 0..15 {
                    let b1 = chunk[2 + row * 2];
                    let b2 = chunk[3 + row * 2];
                    
                    for bit in 0..8 {
                        pixels[row][bit] = (b1 & (0x80 >> bit)) != 0;
                    }
                    for bit in 0..4 {
                        pixels[row][8 + bit] = (b2 & (0x80 >> bit)) != 0;
                    }
                }
                chars.push(FontChar { width, pixels });
            }
        }

        // 2. Загружаем карту ASCII -> Индекс
        let mut ascii_map = HashMap::new();
        if let Ok(content) = fs::read_to_string("src/data/scripting/text/asciitotextsymbolmap.asm") {
            let mut current_ascii = 0;
            for line in content.lines() {
                let clean = line.split(';').next().unwrap().trim();
                if clean.starts_with("dc.b ") {
                    let replaced = clean.replace("dc.b", "");
                    let parts: Vec<&str> = replaced.split(',').map(|s| s.trim()).collect();
                    if let Ok(idx) = parts[0].parse::<usize>() {
                        // Если индекс больше 0, добавляем в карту
                        if idx > 0 && current_ascii < 256 {
                            if let Some(c) = char::from_u32(current_ascii as u32) {
                                // ВАЖНО: Вычитаем 1, так как массивы в Rust начинаются с 0
                                ascii_map.insert(c, idx - 1);
                            }
                        }
                    }
                    current_ascii += 1;
                }
            }
        }

        Self { chars, ascii_map }
    }

    pub fn draw_text(&self, buf: &mut [u8], pitch: usize, text: &str, start_x: i32, start_y: i32, color: Color) {
        let mut x = start_x;
        let mut y = start_y;

        for ch in text.chars() {
            if ch == '\n' {
                x = start_x;
                y += 15;
                continue;
            }

            // Берем индекс из распарсенной карты. Если символа нет, берем 0 (пробел).
            let char_idx = *self.ascii_map.get(&ch).unwrap_or(&0);
            
            if let Some(font_char) = self.chars.get(char_idx) {
                for row in 0..15 {
                    for col in 0..12 {
                        if font_char.pixels[row][col] {
                            let px = x + col as i32;
                            let py = y + row as i32;
                            
                            if px >= 32 && px < 288 && py >= 0 && py < 224 {
                                let off = py as usize * pitch + px as usize * 4;
                                if off + 3 < buf.len() {
                                    buf[off] = color.r;
                                    buf[off + 1] = color.g;
                                    buf[off + 2] = color.b;
                                    buf[off + 3] = 255;
                                }
                            }
                        }
                    }
                }
                
                let w = if font_char.width == 0 { 0 } else { font_char.width as i32 + 1 };
                x += w; 
            }
        }
    }
}