// src/game/map/camera.rs
use crate::game::map::loader::MapData;

pub struct MapState {
    pub view_x: i32,
    pub view_y: i32,
    pub target_view_x: i32,
    pub target_view_y: i32,
    pub is_scrolling: bool,
    
    // Новые поля для точной эмуляции ASM
    pub scroll_frames: u32,   // Счетчик рывков камеры (word_FFA828)
    pub view_x_fixed: i32,    // Субпиксельная позиция X (для скорости 1.5)
    pub view_y_fixed: i32,    // Субпиксельная позиция Y
}

const SCREEN_W: i32 = 256;
const SCREEN_H: i32 = 224;

const DZ_LEFT: i32 = 96;
const DZ_RIGHT: i32 = 144;
const DZ_TOP: i32 = 96;
const DZ_BOTTOM: i32 = 144;

impl MapState {
    pub fn new(target_x: i32, target_y: i32, map_data: &MapData) -> Self {
        let mut s = Self {
            view_x: (target_x - DZ_LEFT).max(0),
            view_y: (target_y - DZ_TOP).max(0),
            target_view_x: 0,
            target_view_y: 0,
            is_scrolling: false,
            scroll_frames: 0,
            view_x_fixed: 0,
            view_y_fixed: 0,
        };
        s.update_target(target_x, target_y, map_data);
        s.view_x = s.target_view_x;
        s.view_y = s.target_view_y;
        s.view_x_fixed = s.view_x * 16;
        s.view_y_fixed = s.view_y * 16;
        s
    }

    pub fn new_for_warp(tile_x: i32, tile_y: i32, map_data: &MapData) -> Self {
        let px = tile_x * 24;
        let py = tile_y * 24;
        let mut s = Self {
            view_x: (px - DZ_LEFT).max(0),
            view_y: (py - DZ_TOP).max(0),
            target_view_x: 0,
            target_view_y: 0,
            is_scrolling: false,
            scroll_frames: 0,
            view_x_fixed: 0,
            view_y_fixed: 0,
        };
        s.update_target(px, py, map_data);
        s.view_x = s.target_view_x;
        s.view_y = s.target_view_y;
        s.view_x_fixed = s.view_x * 16;
        s.view_y_fixed = s.view_y * 16;
        s
    }

    pub fn update_view(&mut self, target_x: i32, target_y: i32, map_data: &MapData) {
        self.update_target(target_x, target_y, map_data);
    }

    pub fn update_target(&mut self, target_x: i32, target_y: i32, map_data: &MapData) {
        if self.is_scrolling { return; }

        let tile_x = (target_x / 24) as u8;
        let tile_y = (target_y / 24) as u8;

        let mut min_x = 0;
        let mut min_y = 0;
        let mut max_x = 64 * 24;
        let mut max_y = 64 * 24;

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

        let mut scrolled = false;
        let sx = target_x - self.view_x;
        if sx < DZ_LEFT {
            self.target_view_x = target_x - DZ_LEFT;
            scrolled = true;
        } else if sx > DZ_RIGHT {
            self.target_view_x = target_x - DZ_RIGHT;
            scrolled = true;
        }

        let sy = target_y - self.view_y;
        if sy < DZ_TOP {
            self.target_view_y = target_y - DZ_TOP;
            scrolled = true;
        } else if sy > DZ_BOTTOM {
            self.target_view_y = target_y - DZ_BOTTOM;
            scrolled = true;
        }

        if scrolled {
            self.target_view_x = self.target_view_x.clamp(min_x, limit_x);
            self.target_view_y = self.target_view_y.clamp(min_y, limit_y);
            
            // Включаем скролл, ТОЛЬКО если камера реально сдвинулась с места
            if self.target_view_x != self.view_x || self.target_view_y != self.view_y {
                self.is_scrolling = true;
                self.scroll_frames += 1;
            }
        } else {
            self.scroll_frames = 0;
        }
    }

    // 
    pub fn can_open_menu(&self) -> bool {
        !self.is_scrolling 
            && self.view_x_fixed == self.target_view_x * 16 
            && self.view_y_fixed == self.target_view_y * 16
    }

    pub fn tick(&mut self) {
        if !self.is_scrolling { return; }

        // ТОЧНО КАК В ASM: 
        // Если счетчик <= 6, скорость 24 (1.5 пикселя). Иначе 32 (2 пикселя).
        let speed = if self.scroll_frames > 6 { 32 } else { 24 };

        let target_x_fixed = self.target_view_x * 16;
        let target_y_fixed = self.target_view_y * 16;

        let dx = target_x_fixed - self.view_x_fixed;
        let dy = target_y_fixed - self.view_y_fixed;

        if dx.abs() <= speed { self.view_x_fixed = target_x_fixed; }
        else { self.view_x_fixed += dx.signum() * speed; }

        if dy.abs() <= speed { self.view_y_fixed = target_y_fixed; }
        else { self.view_y_fixed += dy.signum() * speed; }

        // Обновляем обычные координаты для рендера (делим на 16)
        self.view_x = self.view_x_fixed / 16;
        self.view_y = self.view_y_fixed / 16;

        if self.view_x_fixed == target_x_fixed && self.view_y_fixed == target_y_fixed {
            self.is_scrolling = false;
        }
    }
}