// src/game/screens/witch.rs
use sdl3::pixels::Color;
use std::fs;

use crate::engine::renderer::Renderer;
use crate::engine::input::InputState;
use crate::engine::decompressor;
use crate::engine::base_tiles::Vram;

const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 224;

const WITCH_BACKGROUND_WORDS: usize = 896;
const WITCH_HEAD_LAYOUT_OFFSET: usize = 0x700;
const WITCH_HEAD_COLUMNS: usize = 7;
const WITCH_HEAD_ROWS: usize = 7;

const WITCH_TILE_VRAM_BASE: usize = 256;
const BALLOON_TILE_VRAM_BASE: usize = 1024;

const MENU_X: i32 = 48;
const MENU_Y: i32 = 128;
const MENU_WIDTH: i32 = 224;
const MENU_HEIGHT: i32 = 96;

const CRAM_VALUE_MAP: [u8; 8] = [0, 52, 87, 116, 144, 172, 206, 255];

fn decode_genesis_color(first: u8, second: u8) -> Color {
    let word = u16::from_be_bytes([first, second]);
    let r3 = ((word >> 1) & 0x07) as usize;
    let g3 = ((word >> 5) & 0x07) as usize;
    let b3 = ((word >> 9) & 0x07) as usize;
    Color::RGB(CRAM_VALUE_MAP[r3], CRAM_VALUE_MAP[g3], CRAM_VALUE_MAP[b3])
}

fn read_required(path: &str, description: &str) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|err| {
        panic!("Не удалось прочитать {} ({}): {}", description, path, err)
    })
}

fn decode_palette(raw: &[u8], description: &str) -> Vec<Color> {
    if raw.len() % 2 != 0 {
        panic!("{} имеет нечётный размер: {}", description, raw.len());
    }
    raw.chunks_exact(2)
        .map(|pair| decode_genesis_color(pair[0], pair[1]))
        .collect()
}

pub struct WitchData {
    pub tiles: Vec<u8>,
    pub layout: Vec<u8>,
    pub palette: Vec<Color>,
    pub choice_palette: Vec<Color>,
    pub base_palette: Vec<Color>,
    pub balloon_tiles: Vec<u8>,
    pub bubble_anim: Vec<u8>,
    pub font_data: Vec<u8>,
}

impl WitchData {
    pub fn load() -> Self {
        let comp_tiles = read_required("src/data/graphics/specialscreens/witchscreen/witchtiles.bin", "witchtiles.bin");
        let tiles = decompressor::decompress(&comp_tiles).expect("Ошибка STACK-распаковки witchtiles.bin");
        let layout = read_required("src/data/graphics/specialscreens/witchscreen/witchlayout.bin", "witchlayout.bin");
        
        let raw_palette = read_required("src/data/graphics/specialscreens/witchscreen/witchpalette.bin", "witchpalette.bin");
        let palette = decode_palette(&raw_palette, "witchpalette.bin");

        let raw_choice_palette = read_required("src/data/graphics/specialscreens/witchscreen/witchchoicepalette.bin", "witchchoicepalette.bin");
        let choice_palette = decode_palette(&raw_choice_palette, "witchchoicepalette.bin");

        let raw_base_palette = read_required("src/data/graphics/tech/basepalette.bin", "basepalette.bin");
        let base_palette = decode_palette(&raw_base_palette, "basepalette.bin");

        let comp_balloon = read_required("src/data/graphics/specialscreens/witchscreen/speechballoontiles.bin", "speechballoontiles.bin");
        let balloon_tiles = decompressor::decompress(&comp_balloon).expect("Ошибка распаковки speechballoontiles.bin");

        let bubble_anim = read_required("src/data/graphics/specialscreens/witchscreen/witchbubbleanimation.bin", "witchbubbleanimation.bin");
        let font_data = read_required("src/data/graphics/tech/fonts/variablewidthfont.bin", "variablewidthfont.bin");

        Self { tiles, layout, palette, choice_palette, base_palette, balloon_tiles, bubble_anim, font_data }
    }
}

pub struct WitchState {
    pub menu_index: u8,
    pub menu_enabled: u8,
    pub anim_counter: u8,
}

impl WitchState {
    pub fn new(enabled: u8) -> Self {
        let enabled = enabled & 0x0F;
        let first_enabled = (0..4).find(|&index| enabled & (1u8 << index) != 0).unwrap_or(0);
        Self { menu_index: first_enabled as u8, menu_enabled: enabled, anim_counter: 10 }
    }
}

fn put_pixel(buf: &mut [u8], pitch: usize, x: i32, y: i32, color: Color) {
    if x < 0 || x >= SCREEN_WIDTH || y < 0 || y >= SCREEN_HEIGHT { return; }
    let offset = y as usize * pitch + x as usize * 4;
    if offset + 3 >= buf.len() { return; }
    buf[offset] = color.r; buf[offset + 1] = color.g; buf[offset + 2] = color.b; buf[offset + 3] = 255;
}

fn draw_tile(buf: &mut [u8], pitch: usize, tile: &[u8], x: i32, y: i32, palette: &[Color], palette_offset: usize, h_flip: bool, v_flip: bool) {
    if tile.len() < 32 { return; }
    for row in 0..8 {
        for col in 0..8 {
            let source_col = if h_flip { 7 - col } else { col };
            let source_row = if v_flip { 7 - row } else { row };
            let byte_index = source_row * 4 + source_col / 2;
            let byte = tile[byte_index];
            let color_index = if source_col % 2 == 0 { ((byte >> 4) & 0x0F) as usize } else { (byte & 0x0F) as usize };
            if color_index == 0 { continue; }
            let palette_index = palette_offset + color_index;
            let Some(&color) = palette.get(palette_index) else { continue; };
            put_pixel(buf, pitch, x + col as i32, y + row as i32, color);
        }
    }
}

fn char_to_vwf_idx(ch: char) -> Option<usize> {
    match ch {
        ' ' => Some(1), 'A'..='Z' => Some((ch as u8 - b'A' + 2) as usize),
        'a'..='z' => Some((ch as u8 - b'a' + 28) as usize), '.' => Some(54),
        ',' => Some(55), '!' => Some(56), '?' => Some(57), '\'' => Some(60), _ => None,
    }
}

fn draw_vwf_text(buf: &mut [u8], pitch: usize, font_data: &[u8], text: &str, mut x: i32, y: i32, color: Color) {
    for ch in text.chars() {
        let Some(index) = char_to_vwf_idx(ch) else { continue; };
        let offset = index * 32;
        if offset + 31 >= font_data.len() { continue; }
        let width = (font_data[offset] & 0x0F) as i32;
        for row in 0..15 {
            let byte1 = font_data[offset + 1 + row * 2];
            let byte2 = font_data[offset + 2 + row * 2];
            let bits = u16::from_be_bytes([byte1, byte2]);
            for pixel in 0..12 {
                if bits & (1 << (15 - pixel)) != 0 {
                    put_pixel(buf, pitch, x + pixel as i32, y + row as i32, color);
                }
            }
        }
        x += width;
    }
}

fn draw_window(buf: &mut [u8], pitch: usize, data: &WitchData, vram: &Vram, x: i32, y: i32, width: i32, height: i32) {
    let cols = (width / 8) as usize;
    let rows = (height / 8) as usize;
    const CORNER: usize = 0; const H_BORDER: usize = 1; const V_BORDER: usize = 2; const SPACE: usize = 3;

    for r in 0..rows {
        for c in 0..cols {
            let is_top = r == 0; let is_bottom = r == rows - 1;
            let is_left = c == 0; let is_right = c == cols - 1;
            let (tile_idx, h_flip, v_flip) = if is_top && is_left { (CORNER, false, false) }
                else if is_top && is_right { (CORNER, true, false) }
                else if is_bottom && is_left { (CORNER, false, true) }
                else if is_bottom && is_right { (CORNER, true, true) }
                else if is_top { (H_BORDER, false, false) }
                else if is_bottom { (H_BORDER, false, true) }
                else if is_left { (V_BORDER, false, false) }
                else if is_right { (V_BORDER, true, false) }
                else { (SPACE, false, false) };
            if let Some(tile) = vram.tile(tile_idx) {
                draw_tile(buf, pitch, tile, x + c as i32 * 8, y + r as i32 * 8, &data.base_palette, 0, h_flip, v_flip);
            }
        }
    }
}

fn draw_witch_layout_tile(buf: &mut [u8], pitch: usize, data: &WitchData, word: u16, x: i32, y: i32) {
    let vdp_tile_index = (word & 0x07FF) as usize;
    if vdp_tile_index == 0 { return; }
    let tile_index = if vdp_tile_index >= WITCH_TILE_VRAM_BASE { vdp_tile_index - WITCH_TILE_VRAM_BASE } else { vdp_tile_index };
    let start = tile_index * 32;
    if start + 32 > data.tiles.len() { return; }
    let palette_index = ((word >> 13) & 0x03) as usize;
    let palette_offset = if palette_index == 3 { 16 } else { 0 };
    let h_flip = word & 0x0800 != 0;
    let v_flip = word & 0x1000 != 0;
    draw_tile(buf, pitch, &data.tiles[start..start+32], x, y, &data.palette, palette_offset, h_flip, v_flip);
}

pub fn update_and_draw(renderer: &mut Renderer, vram: &Vram, _input: &InputState) {
    let state = WitchState::new(0b1111);
    let data = WitchData::load();

    renderer.texture.with_lock(None, |buffer, pitch| {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                put_pixel(buffer, pitch, x, y, Color::RGB(0, 0, 0));
            }
        }

        // 1. Фон
        let words_to_draw = WITCH_BACKGROUND_WORDS.min(data.layout.len() / 2);
        for index in 0..words_to_draw {
            let offset = index * 2;
            let word = u16::from_be_bytes([data.layout[offset], data.layout[offset + 1]]);
            let tile_x = (index % 32) as i32; let tile_y = (index / 32) as i32;
            draw_witch_layout_tile(buffer, pitch, &data, word, 32 + tile_x * 8, tile_y * 8);
        }

        // 2. Голова
        let start_word = WITCH_HEAD_LAYOUT_OFFSET / 2;
        if start_word + WITCH_HEAD_COLUMNS * WITCH_HEAD_ROWS <= data.layout.len() / 2 {
            for row in 0..WITCH_HEAD_ROWS {
                for col in 0..WITCH_HEAD_COLUMNS {
                    let index = start_word + row * WITCH_HEAD_COLUMNS + col;
                    let offset = index * 2;
                    let word = u16::from_be_bytes([data.layout[offset], data.layout[offset + 1]]);
                    draw_witch_layout_tile(buffer, pitch, &data, word, 32 + 120 + col as i32 * 8, 64 + row as i32 * 8);
                }
            }
        }

        // 3. Окно
        draw_window(buffer, pitch, &data, vram, MENU_X, MENU_Y, MENU_WIDTH, MENU_HEIGHT);

        // 4. Пузыри
        let bubbles: [(i32, i32, &str, usize); 4] = [
            (0, 56, "START", 0), (16, 0, "CONT.", 1), (144, 0, "DEL.", 2), (160, 56, "COPY", 3),
        ];

        for (bx, by, text, bit) in bubbles {
            if state.menu_enabled & (1 << bit) == 0 { continue; }
            let anim_offset = if bit as u8 == state.menu_index {
                match state.anim_counter {
                    0..=4 => 0, 5..=9 => 0x50, 10..=14 => 0xA0, _ => 0x50,
                }
            } else { 0 };
            let frame_offset = bit * 0xF0 + anim_offset;
            let bubble_x = MENU_X + bx; let bubble_y = MENU_Y + by;

            for r in 0..5 {
                for c in 0..8 {
                    let idx = frame_offset + (r * 8 + c) * 2;
                    if idx + 1 >= data.bubble_anim.len() { continue; }
                    let source_word = u16::from_be_bytes([data.bubble_anim[idx], data.bubble_anim[idx + 1]]);
                    let word = source_word.wrapping_add(0xA300);
                    let tile_number = (word & 0x07FF) as usize;
                    if tile_number < BALLOON_TILE_VRAM_BASE { continue; }
                    let tile_idx = tile_number - BALLOON_TILE_VRAM_BASE;
                    let start = tile_idx * 32; let end = start + 32;
                    if end > data.balloon_tiles.len() { continue; }
                    let h_flip = word & 0x0800 != 0; let v_flip = word & 0x1000 != 0;
                    draw_tile(buffer, pitch, &data.balloon_tiles[start..end], bubble_x + c as i32 * 8, bubble_y + r as i32 * 8, &data.choice_palette, 0, h_flip, v_flip);
                }
            }
            let text_x = bubble_x + 8 + if bit >= 2 { 8 } else { 0 };
            let text_y = bubble_y + 16;
            draw_vwf_text(buffer, pitch, &data.font_data, text, text_x, text_y, Color::RGB(0, 0, 0));
        }

        // 5. Текст в окне
        draw_vwf_text(buffer, pitch, &data.font_data, "Hee, hee, hee...", MENU_X + 16, MENU_Y + 16, Color::RGB(255, 255, 255));
        draw_vwf_text(buffer, pitch, &data.font_data, "You're finally here!", MENU_X + 16, MENU_Y + 32, Color::RGB(255, 255, 255));
    }).unwrap();
}