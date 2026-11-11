// src/game/map/camera.rs
use crate::game::map::loader::MapData;

pub struct MapState {
    pub view_x: i32,
    pub view_y: i32,
    pub target_view_x: i32,
    pub target_view_y: i32,
    pub is_scrolling: bool,
    
    pub scroll_frames: u32,
    pub view_x_fixed: i32,
    pub view_y_fixed: i32,
    pub player_near_edge: bool,
}

const SCREEN_W: i32 = 256;
const SCREEN_H: i32 = 224;

const DZ_LEFT: i32 = 72;
const DZ_RIGHT: i32 = 160;
const DZ_TOP: i32 = 72;
const DZ_BOTTOM: i32 = 128;

impl MapState {
    fn get_area_bounds(tile_x: i32, tile_y: i32, map_data: &MapData) -> (i32, i32, i32, i32) {
        let mut min_x = 0;
        let mut min_y = 0;
        let mut max_x = 64 * 24;
        let mut max_y = 64 * 24;

        for area in &map_data.areas {
            if tile_x >= area.start_x as i32 && tile_x <= area.end_x as i32 && tile_y >= area.start_y as i32 && tile_y <= area.end_y as i32 {
                min_x = area.start_x as i32 * 24;
                min_y = area.start_y as i32 * 24;
                max_x = (area.end_x as i32 + 1) * 24;
                max_y = (area.end_y as i32 + 1) * 24;
                break;
            }
        }
        (min_x, min_y, max_x, max_y)
    }

    pub fn new(target_x: i32, target_y: i32, map_data: &MapData) -> Self {
        let tile_x = target_x / 24;
        let tile_y = target_y / 24;

        let (min_x, min_y, max_x, max_y) = Self::get_area_bounds(tile_x, tile_y, map_data);
        let limit_x = (max_x - SCREEN_W).max(min_x);
        let limit_y = (max_y - SCREEN_H).max(min_y);

        let target_view_x = (target_x - DZ_LEFT).clamp(min_x, limit_x);
        let target_view_y = (target_y - DZ_TOP).clamp(min_y, limit_y);

        Self {
            view_x: target_view_x,
            view_y: target_view_y,
            target_view_x,
            target_view_y,
            is_scrolling: false,
            scroll_frames: 0,
            view_x_fixed: target_view_x * 16,
            view_y_fixed: target_view_y * 16,
            player_near_edge: false,
        }
    }

    pub fn new_for_warp(tile_x: i32, tile_y: i32, map_data: &MapData) -> Self {
        let px = tile_x * 24;
        let py = tile_y * 24;
        
        let (min_x, min_y, max_x, max_y) = Self::get_area_bounds(tile_x, tile_y, map_data);
        let limit_x = (max_x - SCREEN_W).max(min_x);
        let limit_y = (max_y - SCREEN_H).max(min_y);

        let target_view_x = (px - DZ_LEFT).clamp(min_x, limit_x);
        let target_view_y = (py - DZ_TOP).clamp(min_y, limit_y);

        Self {
            view_x: target_view_x,
            view_y: target_view_y,
            target_view_x,
            target_view_y,
            is_scrolling: false,
            scroll_frames: 0,
            view_x_fixed: target_view_x * 16,
            view_y_fixed: target_view_y * 16,
            player_near_edge: false,
        }
    }

    pub fn update_view(&mut self, target_x: i32, target_y: i32, map_data: &MapData) {
        let px = target_x - self.view_x;
        let py = target_y - self.view_y;
        
        // Игрок считается "у края", если он в 2 тайлах (48px)
        self.player_near_edge = 
            (px <= 48) || (px >= 256 - 48 - 24) ||
            (py <= 48) || (py >= 224 - 48 - 24);

        self.update_target(target_x, target_y, map_data);
    }

    pub fn update_target(&mut self, target_x: i32, target_y: i32, map_data: &MapData) {
        let tile_x = (target_x / 24) as u8;
        let tile_y = (target_y / 24) as u8;

        let (min_x, min_y, max_x, max_y) = Self::get_area_bounds(tile_x as i32, tile_y as i32, map_data);
        let limit_x = (max_x - SCREEN_W).max(min_x);
        let limit_y = (max_y - SCREEN_H).max(min_y);

        let mut scrolled = false;
        let sx = target_x - self.view_x;
        let sy = target_y - self.view_y;

        // ТОЛЬКО 3 ТАЙЛА! (Никаких жестких лимитов на 2 тайла)
        if sx > DZ_RIGHT { 
            self.target_view_x = target_x - DZ_RIGHT;
            scrolled = true;
        } else if sx < DZ_LEFT { 
            self.target_view_x = target_x - DZ_LEFT;
            scrolled = true;
        }

        if sy > DZ_BOTTOM { 
            self.target_view_y = target_y - DZ_BOTTOM;
            scrolled = true;
        } else if sy < DZ_TOP { 
            self.target_view_y = target_y - DZ_TOP;
            scrolled = true;
        }

        if scrolled {
            self.target_view_x = self.target_view_x.clamp(min_x, limit_x);
            self.target_view_y = self.target_view_y.clamp(min_y, limit_y);
            
            if self.target_view_x != self.view_x || self.target_view_y != self.view_y {
                self.is_scrolling = true;
                self.scroll_frames += 1;
            }
        }
    }

    pub fn can_open_menu(&self) -> bool {
        !self.is_scrolling 
            && self.view_x_fixed == self.target_view_x * 16 
            && self.view_y_fixed == self.target_view_y * 16
    }

    pub fn tick(&mut self) {
        if !self.is_scrolling { return; }

        let target_x_fixed = self.target_view_x * 16;
        let target_y_fixed = self.target_view_y * 16;

        // Если игрок близко к краю — камера едет 32 (2 пикс). Иначе 24 (1.5 пикс).
        let speed = if self.player_near_edge { 32 } else { 24 };

        let dx = target_x_fixed - self.view_x_fixed;
        let dy = target_y_fixed - self.view_y_fixed;

        if dx.abs() <= speed {
            self.view_x_fixed = target_x_fixed;
        } else {
            self.view_x_fixed += dx.signum() * speed;
        }

        if dy.abs() <= speed {
            self.view_y_fixed = target_y_fixed;
        } else {
            self.view_y_fixed += dy.signum() * speed;
        }

        self.view_x = self.view_x_fixed / 16;
        self.view_y = self.view_y_fixed / 16;

        if self.view_x_fixed == target_x_fixed && self.view_y_fixed == target_y_fixed {
            self.is_scrolling = false;
            self.scroll_frames = 0;
        }
    }
}