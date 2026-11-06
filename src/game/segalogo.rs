use sdl3::pixels::Color;
use crate::engine::renderer::Renderer;
use crate::game::segalogo_data::{LOGO_ANIMATION, FINAL_FRAME};

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

// A single 4x4-tile (32x32 px) block of the SEGA logo positioned in screen space.
pub struct LogoSprite {
    pub tile_index: usize,
    pub x: i32,
    pub y: i32,
}

// Resolve the sprite list for a given animation frame.
// Frames past the end of the animation clamp to FINAL_FRAME.
pub fn get_logo_frame(frame_index: usize) -> Vec<LogoSprite> {
    let data = if frame_index >= LOGO_ANIMATION.len() {
        FINAL_FRAME
    } else {
        LOGO_ANIMATION[frame_index]
    };
    let mut sprites = Vec::new();
    for (x, y, tile) in data.iter() {
        // Convert tile index to a byte offset (each tile is 16 words = 32 bytes).
        let calc_tile = (*tile as usize) * 16;
        // Convert game-space coordinates to screen space: the logo data is anchored
        // around the 320x224 viewport, so shift by half-tile centering offsets.
        let calc_x = (*x as i32) + 240 - 129; 
        let calc_y = 224 - (*y as i32) - 124;
        sprites.push(LogoSprite { tile_index: calc_tile, x: calc_x, y: calc_y });
    }
    sprites
}

pub fn update_and_draw(renderer: &mut Renderer, frame: u32) {
    // NOTE: tile and palette assets are re-read and decompressed from disk on
    // every frame. This is acceptable for a short intro screen, but should be
    // hoisted into a cached asset if this pattern is reused elsewhere.
    let tiles_data = std::fs::read("src/data/graphics/tech/segalogotiles.bin").expect("Нет файла тайлов");
    let tiles = crate::engine::decompressor::decompress(&tiles_data).expect("Ошибка распаковки");
    
    let pal_data = std::fs::read("src/data/graphics/tech/segalogopalette.bin").expect("Нет палитры");
    let mut palette = Vec::new();
    for i in 0..(pal_data.len() / 2) {
        palette.push(decode_genesis_color(pal_data[i*2], pal_data[i*2+1]));
    }

    let sprites = get_logo_frame(frame as usize);

    renderer.texture.with_lock(None, |buffer, pitch| {
        // Fill the framebuffer with opaque black.
        for y in 0..224 {
            for x in 0..320 {
                let off = y * pitch + x * 4;
                buffer[off] = 0; buffer[off+1] = 0; buffer[off+2] = 0; buffer[off+3] = 255;
            }
        }
        // Draw each 4x4-tile logo block.
        for spr in sprites {
            for tx in 0..4 {
                for ty in 0..4 {
                    // Tiles are laid out row-major within the 4x4 block (4 tiles per row).
                    let tile_idx = spr.tile_index + (tx * 4) + ty;
                    if tile_idx * 32 + 32 > tiles.len() { continue; }
                    let tile = &tiles[tile_idx * 32..(tile_idx + 1) * 32];
                    
                    // 4bpp tile: 32 bytes = 8x8 pixels, two pixels per byte
                    // (high nibble = left pixel, low nibble = right pixel).
                    for row in 0..8 {
                        for col in 0..8 {
                            let byte_idx = row * 4 + (col / 2);
                            let byte = tile[byte_idx];
                            let ci = if col % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;
                            // Palette index 0 is transparent; out-of-range indices are skipped.
                            if ci == 0 || ci >= palette.len() { continue; }
                            
                            let c = palette[ci];
                            let px = spr.x + (tx as i32 * 8) + col as i32;
                            let py = spr.y + (ty as i32 * 8) + row as i32;
                            
                            // Clip against the viewport.
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