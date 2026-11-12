// src/game/screens/title.rs
use sdl3::pixels::Color;
use std::sync::OnceLock;
use crate::engine::renderer::Renderer;
use crate::engine::decompressor;

// Genesis 3-bit color levels expanded to 8-bit RGB values.
const CRAM_VALUE_MAP: [u8; 8] = [0, 52, 87, 116, 144, 172, 206, 255];

// Decode a Genesis/Mega Drive color word stored as two bytes into SDL RGB.
fn decode_genesis_color(first: u8, second: u8) -> Color {
    let word = ((first as u16) << 8) | (second as u16);
    let r3 = ((word >> 1) & 0x07) as usize;
    let g3 = ((word >> 5) & 0x07) as usize;
    let b3 = ((word >> 9) & 0x07) as usize;
    Color::RGB(CRAM_VALUE_MAP[r3], CRAM_VALUE_MAP[g3], CRAM_VALUE_MAP[b3])
}


pub struct TitleAssets {
    pub tiles: Vec<u8>,
    pub layout_a: Vec<u8>,
    pub layout_b: Vec<u8>,
    pub palette: Vec<Color>,
}

static ASSETS: OnceLock<TitleAssets> = OnceLock::new();

fn load_assets() -> TitleAssets {
    let comp_tiles = std::fs::read("src/data/graphics/specialscreens/titlescreen/titlescreentiles.bin").expect("Нет titlescreentiles.bin");
    let tiles = decompressor::decompress(&comp_tiles).expect("Ошибка распаковки");
    let layout_a = std::fs::read("src/data/graphics/specialscreens/titlescreen/titlescreenlayoutA.bin").expect("Нет layout A");
    let layout_b = std::fs::read("src/data/graphics/specialscreens/titlescreen/titlescreenlayoutB.bin").expect("Нет layout B");
    let raw_pal = std::fs::read("src/data/graphics/specialscreens/titlescreen/titlescreenpalettes.bin").expect("Нет палитры");

    let palette = (0..(raw_pal.len() / 2))
        .map(|i| decode_genesis_color(raw_pal[i * 2], raw_pal[i * 2 + 1]))
        .collect();

    TitleAssets { tiles, layout_a, layout_b, palette }
}

pub fn preload() {
    ASSETS.get_or_init(load_assets);
}


pub fn update_and_draw(renderer: &mut Renderer, mut frame: u32) {
    let a = ASSETS.get_or_init(load_assets);
    let tiles = &a.tiles;
    let layout_a = &a.layout_a;
    let layout_b = &a.layout_b;
    let palette = &a.palette;


    //    0- 99 : fade in "SHINING FORCE" logo
    //  100-131 : scroll background down from +24 to +8   (32 frames)
    //  132-147 : scroll background down from  +8 to  0   (16 frames)
    //  148-197 : fade in the "II" graphic                (50 frames)
    //  198-317 : fade in "PRESS START"                   (120 frames)
    //  318-917 : blink "PRESS START" every 15 frames     (600 frames)
    let fade_logo: f32;
    let mut scroll_bg: i32 = 24;
    let mut fade_ii: f32 = 0.0;
    let mut fade_press: f32 = 0.0;
    let mut press_start_visible = false;

    if frame < 100 {
        fade_logo = (frame as f32 / 100.0).min(1.0);
    } else {
        fade_logo = 1.0;
        frame -= 100;
        if frame < 32 {
            scroll_bg = 24 - (frame / 2) as i32;
        } else {
            frame -= 32;
            if frame < 16 {
                // Second scroll segment: continue from +8 down to 0.
                scroll_bg = 8 - (frame / 2) as i32;
            } else {
                scroll_bg = 0;
                frame -= 16;
                if frame < 50 {
                    fade_ii = (frame as f32 / 50.0).min(1.0);
                } else {
                    fade_ii = 1.0;
                    frame -= 50;
                    if frame < 120 {
                        fade_press = (frame as f32 / 120.0).min(1.0);
                    } else {
                        fade_press = 1.0;
                        frame -= 120;
                        if frame < 600 {
                            press_start_visible = (frame % 30) < 15;
                        }
                    }
                }
            }
        }
    }

    renderer.texture.with_lock(None, |buffer, pitch| {
        // 1. Clear the framebuffer to opaque black.
        for y in 0..224 {
            for x in 0..320 {
                let offset = y * pitch + x * 4;
                buffer[offset] = 0; buffer[offset + 1] = 0; buffer[offset + 2] = 0; buffer[offset + 3] = 255;
            }
        }

        // 2. Draw the background "II" from plane B.
        //    Plane B uses a 32x11 tilemap; its origin is (32, 32) in screen
        //    space. Fade-in multiplier `fade_ii` is applied per output pixel,
        //    so brightness ramps smoothly without extra passes.
        //
        //    Genesis plane word layout:
        //      bits 0-10 : tile index  (0x07FF)
        //      bit  11   : H flip      (0x0800)
        //      bit  12   : V flip      (0x1000)
        //      bit  13   : palette     (0x2000)
        //      bit  14   : priority    (0x4000)
        //    `attr | 0x2000` forces bit 13 so "II" always selects palette >= 1.
        if fade_ii > 0.0 {
            for ty in 0..11 {
                for tx in 0..32 {
                    let index = ty * 32 + tx;
                    if index * 2 + 1 >= layout_b.len() { break; }
                    let word = u16::from_be_bytes([layout_b[index*2], layout_b[index*2+1]]);
                    // A zero word marks an empty cell in the plane map.
                    if word == 0 { continue; }

                    let attr = word | 0x2000;
                    let pal_idx = ((attr >> 13) & 0x03) as usize;
                    let abs_tile_idx = (attr & 0x07FF) as usize;
                    
                    // Plane B stores tile indices offset by +256; the real
                    // offset into the decompressed sheet is `index - 256`.
                    if abs_tile_idx < 256 { continue; }
                    let tile_idx = abs_tile_idx - 256;

                    if tile_idx * 32 + 32 > tiles.len() { continue; }
                    let tile = &tiles[tile_idx * 32..(tile_idx + 1) * 32];

                    let h_flip = (attr & 0x0800) != 0;
                    let v_flip = (attr & 0x1000) != 0;

                    let x_base = 32 + tx as i32 * 8;
                    let y_base = 32 + ty as i32 * 8;

                    for row in 0..8 {
                        for col in 0..8 {
                            let actual_col = if h_flip { 7 - col } else { col };
                            let actual_row = if v_flip { 7 - row } else { row };

                            // 4bpp tile: two pixels per byte, high nibble = left pixel.
                            let byte_idx = actual_row * 4 + (actual_col / 2);
                            let byte = tile[byte_idx];
                            let ci = if actual_col % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;

                            // Palette index 0 is transparent.
                            if ci == 0 { continue; }

                            let color_idx = pal_idx * 16 + ci;
                            let Some(c) = palette.get(color_idx) else { continue; };
                            
                            // Apply the fade-in multiplier to the final color.
                            let r = (c.r as f32 * fade_ii) as u8;
                            let g = (c.g as f32 * fade_ii) as u8;
                            let b = (c.b as f32 * fade_ii) as u8;
                            
                            let px = x_base + col as i32;
                            let py = y_base + row as i32;

                            if px >= 0 && px < 320 && py >= 0 && py < 224 {
                                let offset = (py as usize) * pitch + (px as usize) * 4;
                                buffer[offset] = r; buffer[offset + 1] = g; buffer[offset + 2] = b; buffer[offset + 3] = 255;
                            }
                        }
                    }
                }
            }
        }

        // 3. Draw "SHINING FORCE" from plane A on top of "II".
        //    Palette 0 tiles are the main logo (fade with `fade_logo`);
        //    other palettes are the prompt row (fade with `fade_press`).
        //
        //    Tiles 576..607 are the "PRESS START" prompt row. While the prompt
        //    is in its blink-off phase, that row is skipped so the prompt
        //    appears to flash.
        let offset_x = 32;
        for i in 0..(layout_a.len() / 2) {
            if i >= 576 && i < 576 + 32 && !press_start_visible {
                continue;
            }

            let word = u16::from_be_bytes([layout_a[i*2], layout_a[i*2+1]]);
            // A zero tile index means an empty cell.
            if (word & 0x07FF) == 0 { continue; }

            // Plane A may or may not have the +256 tile offset that plane B
            // uses; normalize to a direct offset into the tile sheet.
            let abs_tile_idx = (word & 0x07FF) as usize;
            let tile_idx = if abs_tile_idx >= 256 { abs_tile_idx - 256 } else { abs_tile_idx };
            
            let pal_idx = ((word >> 13) & 0x03) as usize; 
            let h_flip = (word & 0x0800) != 0;
            let v_flip = (word & 0x1000) != 0;
            
            let tx = (i % 32) as i32;
            let ty = (i / 32) as i32;

            // Palette 0 = logo, any other palette = prompt row.
            let brightness = if pal_idx == 0 { fade_logo } else { fade_press };
            if brightness <= 0.0 { continue; }

            if tile_idx * 32 + 32 > tiles.len() { continue; }
            let tile = &tiles[tile_idx * 32..(tile_idx + 1) * 32];

            for row in 0..8 {
                for col in 0..8 {
                    let actual_col = if h_flip { 7 - col } else { col };
                    let actual_row = if v_flip { 7 - row } else { row };

                    let byte_idx = actual_row * 4 + (actual_col / 2);
                    let byte = tile[byte_idx];
                    let ci = if actual_col % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;

                    if ci == 0 { continue; } 

                    let color_idx = pal_idx * 16 + ci;
                    let Some(c) = palette.get(color_idx) else { continue; };

                    let r = (c.r as f32 * brightness) as u8;
                    let g = (c.g as f32 * brightness) as u8;
                    let b = (c.b as f32 * brightness) as u8;
                    
                    // Plane A is horizontally offset by 32 px; vertical
                    // position depends on the current background scroll.
                    let px = offset_x + tx * 8 + col as i32;
                    let py = ty * 8 + row as i32 + scroll_bg;

                    if px >= 0 && px < 320 && py >= 0 && py < 224 {
                        let offset = (py as usize) * pitch + (px as usize) * 4;
                        buffer[offset] = r; buffer[offset + 1] = g; buffer[offset + 2] = b; buffer[offset + 3] = 255;
                    }
                }
            }
        }
    }).unwrap();
}