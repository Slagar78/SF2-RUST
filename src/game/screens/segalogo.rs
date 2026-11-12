// src/game/screens/segalogo.rs
use sdl3::pixels::Color;
use std::sync::OnceLock;
use crate::engine::renderer::Renderer;
use crate::engine::decompressor;
use super::segalogo_data::{LOGO_ANIMATION, FINAL_FRAME};

const CRAM_VALUE_MAP: [u8; 8] = [0, 52, 87, 116, 144, 172, 206, 255];

fn decode_genesis_color(first: u8, second: u8) -> Color {
    let word = ((first as u16) << 8) | (second as u16);
    let r3 = ((word >> 1) & 0x07) as usize;
    let g3 = ((word >> 5) & 0x07) as usize;
    let b3 = ((word >> 9) & 0x07) as usize;
    Color::RGB(CRAM_VALUE_MAP[r3], CRAM_VALUE_MAP[g3], CRAM_VALUE_MAP[b3])
}

pub struct SegaAssets {
    pub tiles: Vec<u8>,
    pub palette: Vec<Color>,
}

static ASSETS: OnceLock<SegaAssets> = OnceLock::new();

fn load_assets() -> SegaAssets {
    let tiles_data = std::fs::read("src/data/graphics/tech/segalogotiles.bin")
        .expect("Нет файла тайлов");
    let tiles = decompressor::decompress(&tiles_data).expect("Ошибка распаковки");

    let pal_data = std::fs::read("src/data/graphics/tech/segalogopalette.bin")
        .expect("Нет палитры");
    let palette = (0..(pal_data.len() / 2))
        .map(|i| decode_genesis_color(pal_data[i * 2], pal_data[i * 2 + 1]))
        .collect();

    SegaAssets { tiles, palette }
}

pub fn preload() {
    ASSETS.get_or_init(load_assets);
}

pub struct LogoSprite {
    pub tile_index: usize,
    pub x: i32,
    pub y: i32,
}

pub fn get_logo_frame(frame_index: usize) -> Vec<LogoSprite> {
    let data = if frame_index >= LOGO_ANIMATION.len() {
        FINAL_FRAME
    } else {
        LOGO_ANIMATION[frame_index]
    };
    let mut sprites = Vec::new();
    for (x, y, tile) in data.iter() {
        let calc_tile = (*tile as usize) * 16;
        let calc_x = (*x as i32) + 240 - 129; 
        let calc_y = 224 - (*y as i32) - 124;
        sprites.push(LogoSprite { tile_index: calc_tile, x: calc_x, y: calc_y });
    }
    sprites
}

pub fn update_and_draw(renderer: &mut Renderer, frame: u32) {
    let assets = ASSETS.get_or_init(load_assets);
    let tiles = &assets.tiles;
    let palette = &assets.palette;

    let sprites = get_logo_frame(frame as usize);

    // НАСТОЯЩАЯ ВСПЫШКА (как в ASM, после сборки логотипа)
    // На 160-м кадре экран резко становится белым, а потом гаснет.
    let flash_intensity: f32 = if frame >= 150 && frame < 170 {
        let progress = (frame - 150) as f32 / 10.0;
        if progress < 1.0 { progress } else { 2.0 - progress }
    } else {
        0.0
    };

    renderer.texture.with_lock(None, |buffer, pitch| {
        for y in 0..224 {
            for x in 0..320 {
                let off = y * pitch + x * 4;
                buffer[off] = 0; buffer[off+1] = 0; buffer[off+2] = 0; buffer[off+3] = 255;
            }
        }
        for spr in sprites {
            for tx in 0..4 {
                for ty in 0..4 {
                    let tile_idx = spr.tile_index + (tx * 4) + ty;
                    if tile_idx * 32 + 32 > tiles.len() { continue; }
                    let tile = &tiles[tile_idx * 32..(tile_idx + 1) * 32];
                    for row in 0..8 {
                        for col in 0..8 {
                            let byte_idx = row * 4 + (col / 2);
                            let byte = tile[byte_idx];
                            let ci = if col % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;
                            if ci == 0 || ci >= palette.len() { continue; }
                            
                            let mut c = palette[ci];
                            if flash_intensity > 0.0 {
                                let r = (c.r as f32 * (1.0 - flash_intensity) + 255.0 * flash_intensity) as u8;
                                let g = (c.g as f32 * (1.0 - flash_intensity) + 255.0 * flash_intensity) as u8;
                                let b = (c.b as f32 * (1.0 - flash_intensity) + 255.0 * flash_intensity) as u8;
                                c = Color::RGB(r, g, b);
                            }

                            let px = spr.x + (tx as i32 * 8) + col as i32;
                            let py = spr.y + (ty as i32 * 8) + row as i32;
                            if px >= 0 && px < 320 && py >= 0 && py < 224 {
                                let off = (py as usize) * pitch + (px as usize) * 4;
                                buffer[off] = c.r; buffer[off + 1] = c.g; buffer[off + 2] = c.b; buffer[off + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }).unwrap();
}