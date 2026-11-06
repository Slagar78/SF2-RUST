// src/game/map/camera.rs
use crate::game::map::loader::MapData;

pub struct MapState {
    pub player_x: i32,
    pub player_y: i32,
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub target_scroll_x: i32,
    pub target_scroll_y: i32,
}

const SCREEN_W: i32 = 256; // H32
const SCREEN_H: i32 = 224;

// Мертвые зоны из ASM (96 и 112 от краев)
const DZ_LEFT: i32 = 96;
const DZ_RIGHT: i32 = 256 - 112; // 144
const DZ_TOP: i32 = 96;
const DZ_BOTTOM: i32 = 224 - 112; // 112

impl MapState {
    pub fn new(x: i32, y: i32) -> Self {
        let mut s = Self {
            player_x: x,
            player_y: y,
            scroll_x: 0,
            scroll_y: 0,
            target_scroll_x: 0,
            target_scroll_y: 0,
        };
        
        s.scroll_x = (x - SCREEN_W / 2).max(0);
        s.scroll_y = (y - SCREEN_H / 2).max(0);
        s.target_scroll_x = s.scroll_x;
        s.target_scroll_y = s.scroll_y;
        s
    }

    pub fn update_target(&mut self, map_data: &MapData) {
        let tile_x = (self.player_x / 24) as u8;
        let tile_y = (self.player_y / 24) as u8;

        let mut min_x = 0;
        let mut min_y = 0;
        let mut max_x = 64 * 24;
        let mut max_y = 64 * 24;

        // Ищем текущую зону
        for area in &map_data.areas {
            if tile_x >= area.start_x && tile_x <= area.end_x && tile_y >= area.start_y && tile_y <= area.end_y {
                min_x = area.start_x as i32 * 24;
                min_y = area.start_y as i32 * 24;
                max_x = (area.end_x as i32 + 1) * 24;
                max_y = (area.end_y as i32 + 1) * 24;
                break;
            }
        }

        let limit_x = (max_x - SCREEN_W).max(min_x);
        let limit_y = (max_y - SCREEN_H).max(min_y);

        // ПЛАВНАЯ ЛОГИКА БЕЗ ТРЯСКИ: если игрок внутри мертвой зоны, камера стоит.
        // Если вышел за нее, камера плавно двигается к новой позиции.
        let sx = self.player_x - self.target_scroll_x;
        if sx < DZ_LEFT {
            self.target_scroll_x = self.player_x - DZ_LEFT;
        } else if sx > DZ_RIGHT {
            self.target_scroll_x = self.player_x - DZ_RIGHT;
        }

        let sy = self.player_y - self.target_scroll_y;
        if sy < DZ_TOP {
            self.target_scroll_y = self.player_y - DZ_TOP;
        } else if sy > DZ_BOTTOM {
            self.target_scroll_y = self.player_y - DZ_BOTTOM;
        }

        // Жестко прижимаем к границам зоны (это убирает тряску)
        self.target_scroll_x = self.target_scroll_x.clamp(min_x, limit_x);
        self.target_scroll_y = self.target_scroll_y.clamp(min_y, limit_y);
    }

    pub fn tick(&mut self) {
        let dx = self.target_scroll_x - self.scroll_x;
        let dy = self.target_scroll_y - self.scroll_y;

        let speed = 4; 
        if dx.abs() <= speed { self.scroll_x = self.target_scroll_x; }
        else { self.scroll_x += dx.signum() * speed; }

        if dy.abs() <= speed { self.scroll_y = self.target_scroll_y; }
        else { self.scroll_y += dy.signum() * speed; }
    }
}