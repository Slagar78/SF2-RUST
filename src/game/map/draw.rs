use sdl3::pixels::Color;
use crate::game::map::loader::{MapData, MapLayoutBlock, MapArea};
use crate::game::map::camera::MapState;

fn put_pixel(buf: &mut [u8], pitch: usize, x: i32, y: i32, color: Color) {
    if x < 32 || x >= 288 || y < 0 || y >= 224 { return; } 
    let offset = y as usize * pitch + x as usize * 4;
    if offset + 3 >= buf.len() { return; }
    buf[offset] = color.r;
    buf[offset + 1] = color.g;
    buf[offset + 2] = color.b;
    buf[offset + 3] = 255;
}

pub fn draw_map(buf: &mut [u8], pitch: usize, map_data: &MapData, state: &MapState, layer: u8, player_px_x: i32, player_px_y: i32) {
    // Очищаем экран черным при отрисовке нижнего слоя
    if layer == 0 {
        for y in 0..224 {
            for x in 0..320 {
                let offset = y * pitch + x * 4;
                if offset + 3 < buf.len() {
                    buf[offset] = 0; buf[offset+1] = 0; buf[offset+2] = 0; buf[offset+3] = 255;
                }
            }
        }
    }

    let scroll_x = state.view_x;
    let scroll_y = state.view_y;
    let x_offset = 32;

    // Находим текущую зону игрока (используем переданные координаты)
    let tile_x = (player_px_x / 24) as u8;
    let tile_y = (player_px_y / 24) as u8;
    
    let mut active_area = MapArea { 
        start_x: 0, start_y: 0, 
        end_x: 63, end_y: 63, 
        scnd_x: 0, scnd_y: 0 
    };
    
    for area in &map_data.areas {
        if tile_x >= area.start_x && tile_x <= area.end_x && tile_y >= area.start_y && tile_y <= area.end_y {
            active_area = *area;
            break;
        }
    }

    // ВАЖНО: Перебираем ВСЮ карту 64x64
    for by in 0..64 {
        for bx in 0..64 {
            let block = &map_data.layout[by * 64 + bx];
            if block.block_index == 0 { continue; }

            let is_secondary = bx >= active_area.scnd_x as usize && by >= active_area.scnd_y as usize;

            let draw_x = if is_secondary { bx - active_area.scnd_x as usize } else { bx };
            let draw_y = if is_secondary { by - active_area.scnd_y as usize } else { by };

            draw_block(buf, pitch, map_data, block, draw_x, draw_y, scroll_x, scroll_y, x_offset, layer);
        }
    }
}

fn draw_block(buf: &mut [u8], pitch: usize, map_data: &MapData, block: &MapLayoutBlock, bx: usize, render_by: usize, scroll_x: i32, scroll_y: i32, x_offset: i32, layer: u8) {
    let b_idx = block.block_index as usize;
    let off = b_idx * 9;
    if off + 9 > map_data.blockset.len() || block.block_index == 0 { return; }

    for ti in 0..9 {
        let word = map_data.blockset[off + ti];
        let is_high_priority = (word & 0x8000) != 0;

        // Слой 0 рисует только фон. Слой 1 рисует только то, что перекрывает игрока (в т.ч. верхушки деревьев и стены).
        if layer == 0 && is_high_priority { continue; }
        if layer == 1 && !is_high_priority { continue; }

        let hf = word & 0x0800 != 0;
        let vf = word & 0x1000 != 0;
        let pal = ((word & 0x6000) >> 13) as usize;
        let tile_idx = (word & 0x07FF) as i32;
        let base = tile_idx - 256;

        let (tset, local) = if base >= 0 && base < 6 * 128 {
            let set_val = (base / 128) as usize;
            let ti_local = (base % 128) as usize;
            (&map_data.tilesets[set_val], ti_local)
        } else {
            (&map_data.tilesets[0], 0usize)
        };

        let to = local * 32;
        if to + 32 > tset.len() { continue; }
        let tile = &tset[to..to + 32];

        let tx = (bx * 24 + (ti % 3) * 8) as i32 - scroll_x + x_offset;
        let ty = (render_by * 24 + (ti / 3) * 8) as i32 - scroll_y;

        for py in 0..8 {
            for px in 0..8 {
                let out_x = tx + px as i32;
                let out_y = ty + py as i32;

                if out_x >= 32 && out_x < 288 && out_y >= 0 && out_y < 224 {
                    let spx = if hf { 7 - px } else { px };
                    let spy = if vf { 7 - py } else { py };
                    let bi = spy * 4 + spx / 2;
                    let byte = tile[bi];
                    let ci = if spx % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;
                    
                    if ci == 0 { continue; }

                    let fi = pal * 16 + ci;
                    let color = if fi < map_data.palette.len() {
                        map_data.palette[fi]
                    } else {
                        Color::RGB(255, 0, 255)
                    };
                    put_pixel(buf, pitch, out_x, out_y, color);
                }
            }
        }
    }
}