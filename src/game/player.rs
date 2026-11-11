use sdl3::pixels::Color;
use crate::engine::decompressor;
use crate::game::map::loader::MapData;
use crate::engine::input::{INPUT_UP, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT};
use crate::game::map::events::{check_tile_events, MapEventResult, MapEventsState};

// Tile size in fixed-point units: 24 real px per tile × 16 = 384.
// All pixel coordinates in this module are stored ×16 so movement can advance
// by sub-pixel amounts without float math (divide by 16 to get screen pixels).
pub const TILE_SIZE_FIXED: i32 = 24 * 16;

// Each 8x8 4bpp tile is 32 bytes: 8 rows × 4 bytes per row.
pub const TILE_BYTES: usize = 32;

// Player sprites are 3x3 tile grids (24x24 px) = 9 tiles per animation frame.
pub const FRAME_TILES: usize = 9;
pub const FRAME_BYTES: usize = FRAME_TILES * TILE_BYTES;

// Per-tick movement speed in fixed-point units (32 / 16 = 2 real px per tick).
// Historical note: originally 64 (4 px/tick); halved to 32 to match SF2's pace.
pub const SPEED: i32 = 32;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Facing { Down, Up, Left, Right }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MoveResult {
    None,
    Warp(u8, u8, u8, u8),
}

pub struct PlayerState {
    pub pixel_x: i32, pub pixel_y: i32,
    /// Snap target in fixed-point units; `pixel_*` lerps toward this.
    pub target_x: i32, pub target_y: i32,
    /// Remaining distance of the current step, in fixed-point units.
    /// Zero on both axes means the sprite is settled.
    pub travel_x: i32, pub travel_y: i32,
    /// Per-tick delta applied to `pixel_*` while a step is in progress.
    pub velocity_x: i32, pub velocity_y: i32,
    pub tile_x: i32, pub tile_y: i32,
    pub facing: Facing,
    pub anim_counter: u8,
    pub mapsprite_frame: u8,
    /// Direction of the currently active step; `None` when idle.
    pub current_direction: Option<Facing>,
    /// Tick counter that suppresses input (e.g. after a warp rollback).
    pub input_delayer: u8,
    /// Whether the player is currently standing on a staircase tile.
    pub on_stairs: bool,
    /// Warp request queued for the moment the current step completes.
    /// The teleport is deferred so the sprite arrives exactly tile-centered.
    pub pending_warp: Option<(u8, u8, u8, u8)>,
    /// Landing tile of the most recent warp; suppresses immediate re-trigger
    /// of the same warp tile on arrival.
    pub last_warp_tile: Option<(i32, i32)>,
}

/// Direction-to-offset helper. Currently unused; kept for potential
/// pathfinding/AI code that needs the same facing-to-vector mapping.
#[allow(dead_code)]
pub fn direction_to_vector(dir: Facing) -> (i32, i32) {
    match dir {
        Facing::Up    => (0, -1),
        Facing::Down  => (0,  1),
        Facing::Left  => (-1, 0),
        Facing::Right => ( 1, 0),
    }
}

impl PlayerState {
    pub fn new(tile_x: i32, tile_y: i32) -> Self {
        let px = tile_x * TILE_SIZE_FIXED;
        let py = tile_y * TILE_SIZE_FIXED;
        Self {
            pixel_x: px, pixel_y: py, target_x: px, target_y: py,
            travel_x: 0, travel_y: 0, velocity_x: 0, velocity_y: 0,
            tile_x, tile_y, facing: Facing::Down,
            anim_counter: 1, mapsprite_frame: 0,
            current_direction: None, input_delayer: 0,
            on_stairs: false, pending_warp: None, last_warp_tile: None,
        }
    }

    pub fn is_moving(&self) -> bool {
        self.travel_x != 0 || self.travel_y != 0
    }

    /// Attempt to start a step from the held directional input.
    /// Returns `MoveResult::Warp(..)` only when a previously queued warp fires,
    /// which happens strictly after the current step settles (matches SF2's
    /// "warp on step completion" timing).
    pub fn try_move(&mut self, input_bits: u8, map: &mut MapData, events: &mut MapEventsState) -> MoveResult {
        if self.is_moving() { return MoveResult::None; }

        // Deferred warp dispatch: `pending_warp` is set by `tick` once a step
        // lands on a warp tile. Consuming it here fires the teleport only when
        // the sprite is fully settled, and remembers the landing tile so the
        // arrival does not re-trigger the same warp.
        if let Some((dest_map, x, y, facing)) = self.pending_warp.take() {
            self.last_warp_tile = Some((x as i32, y as i32));
            return MoveResult::Warp(dest_map, x, y, facing);
        }

        if self.input_delayer > 0 {
            self.input_delayer -= 1;
            return MoveResult::None;
        }

        let up = (input_bits & INPUT_UP) != 0;
        let down = (input_bits & INPUT_DOWN) != 0;
        let left = (input_bits & INPUT_LEFT) != 0;
        let right = (input_bits & INPUT_RIGHT) != 0;

        if !up && !down && !left && !right { return MoveResult::None; }

        // === ПОВОРОТ НА МЕСТЕ ===
        // Определяем приоритетное направление (как в Ruby: Right, Left, Down, Up)
        let intended_dir = if right { Facing::Right }
                           else if left { Facing::Left }
                           else if down { Facing::Down }
                           else { Facing::Up };

        // Если нажали в другую сторону — поворачиваемся и стоим 1 кадр
        if self.facing != intended_dir && !self.on_stairs {
            self.facing = intended_dir;
            self.anim_counter = 0; 
            return MoveResult::None; 
        }
        // ======================================

        // Candidate steps in priority order. `add_cand` deduplicates, so the
        // first legal candidate wins when two inputs map to the same offset.
        let mut candidates: Vec<(i32, i32, Facing)> = Vec::new();
        let mut add_cand = |dx: i32, dy: i32, dir: Facing| {
            if !candidates.contains(&(dx, dy, dir)) {
                candidates.push((dx, dy, dir));
            }
        };

        // Tile flags encode collision/staircase state in the top two bits:
        //   0b01xx = left-hand staircase
        //   0b10xx = right-hand staircase
        //   0b11xx = solid wall
        let cur_idx = (self.tile_y * 64 + self.tile_x) as usize;
        let cur_flags = if cur_idx < map.layout.len() { map.layout[cur_idx].flags } else { 0 };
        
        let on_stairs_left  = (cur_flags & 0xC000) == 0x4000;
        let on_stairs_right = (cur_flags & 0xC000) == 0x8000;

        // Helper: does the tile at (tx, ty) have a staircase flag?
        let is_target_stairs = |tx: i32, ty: i32| -> bool {
            if tx < 0 || tx >= 64 || ty < 0 || ty >= 64 { return false; }
            let idx = (ty * 64 + tx) as usize;
            if idx >= map.layout.len() { return false; }
            let flags = map.layout[idx].flags;
            (flags & 0xC000) == 0x4000 || (flags & 0xC000) == 0x8000
        };

        // Staircase movement. On stairs SF2 permits diagonal steps aligned with
        // the staircase direction, and the two staircase orientations (left-hand
        // vs. right-hand) are mirror images of each other.
        if on_stairs_left {
            // Убрали UP. Поднимаемся только по кнопке ВЛЕВО.
            if left { 
                add_cand(-1, -1, Facing::Left); 
                add_cand(-1, 0, Facing::Left);
            }
            // Оставляем DOWN и RIGHT для спуска.
            if down { 
                // Diagonal descent is only valid when the next tile continues the staircase.
                if is_target_stairs(self.tile_x + 1, self.tile_y + 1) {
                    add_cand(1, 1, Facing::Right); 
                }
                add_cand(0, 1, Facing::Down);
            }
            if right { 
                if is_target_stairs(self.tile_x + 1, self.tile_y + 1) {
                    add_cand(1, 1, Facing::Right); 
                }
                add_cand(1, 0, Facing::Right);
            }
        } else if on_stairs_right {
            // Убрали UP. Поднимаемся только по кнопке ВПРАВО.
            if right { 
                add_cand(1, -1, Facing::Right); 
                add_cand(1, 0, Facing::Right);
            }
            // Оставляем DOWN и LEFT для спуска.
            if down { 
                if is_target_stairs(self.tile_x - 1, self.tile_y + 1) {
                    add_cand(-1, 1, Facing::Left); 
                }
                add_cand(0, 1, Facing::Down);
            }
            if left { 
                if is_target_stairs(self.tile_x - 1, self.tile_y + 1) {
                    add_cand(-1, 1, Facing::Left); 
                }
                add_cand(-1, 0, Facing::Left);
            }
        } else {
            // Open ground: cardinal movement only, plus diagonal entry onto an
            // adjacent staircase triggered by holding two directions at once.
            if up    { add_cand(0, -1, Facing::Up); }
            if down  { add_cand(0,  1, Facing::Down); }
            if left  { add_cand(-1, 0, Facing::Left); }
            if right { add_cand(1,  0, Facing::Right); }
            
            if up && left    { add_cand(-1, -1, Facing::Left); }
            if up && right   { add_cand(1, -1, Facing::Right); }
            if down && left  { add_cand(-1, 1, Facing::Left); }
            if down && right { add_cand(1, 1, Facing::Right); }
        }

        self.current_direction = None;
        // Try candidates in priority order; the first legal step is committed.
        for (dx, dy, dir) in candidates {
            if let Some(result) = self.begin_step(dx, dy, map, events) {
                self.current_direction = Some(dir);
                self.facing = dir;
                return result;
            }
        }
        MoveResult::None
    }

    /// Commit a single-tile step if the destination is legal.
    /// Updates tile/target coordinates immediately and starts the pixel lerp
    /// that `tick` will advance each frame.
    fn begin_step(&mut self, dx: i32, dy: i32, map: &mut MapData, _events: &mut MapEventsState) -> Option<MoveResult> {
        let nx = self.tile_x + dx;
        let ny = self.tile_y + dy;
        if nx < 0 || nx >= 64 || ny < 0 || ny >= 64 { return None; }

        let idx = (ny * 64 + nx) as usize;
        if idx >= map.layout.len() { return None; }

        // A walled tile is passable only if a door event sits on it.
        let block_flags = map.layout[idx].flags;
        let is_wall = (block_flags & 0xC000) == 0xC000;
        let is_door = map.step_events.iter().any(|ev| ev.trig_x == nx as u8 && ev.trig_y == ny as u8);

        if is_wall && !is_door { return None; }

        // Clear the post-warp protection once we've moved off the landing tile,
        // allowing the same warp tile to fire again on a later visit.
        if let Some((wx, wy)) = self.last_warp_tile {
            if nx != wx || ny != wy {
                self.last_warp_tile = None;
            }
        }

        // Begin the step: update tile coords now, and lerp pixel coords toward
        // the new target from `tick`.
        self.tile_x = nx; self.tile_y = ny;
        self.target_x = nx * TILE_SIZE_FIXED; self.target_y = ny * TILE_SIZE_FIXED;
        self.travel_x = dx * TILE_SIZE_FIXED; self.travel_y = dy * TILE_SIZE_FIXED;
        // Умножаем скорость на множитель (12/16 = 0.75)
        self.velocity_x = dx * SPEED; 
        self.velocity_y = dy * SPEED;

        self.on_stairs = (block_flags & 0xC000) == 0x4000 || (block_flags & 0xC000) == 0x8000;

        Some(MoveResult::None)
    }

    pub fn interact(&mut self, map: &mut MapData) {
        crate::game::map::events::interact(self, map);
    }

    pub fn tick(&mut self, map: &mut MapData, events: &mut MapEventsState) {
        if self.travel_x != 0 || self.travel_y != 0 {
            self.pixel_x += self.velocity_x;
            self.pixel_y += self.velocity_y;
            
            // Snap to the target and stop velocity on any axis where the next
            // step would overshoot (sign change = past the target).
            let mut finished = false;
            if self.travel_x != 0 && (self.target_x - self.pixel_x).signum() != self.velocity_x.signum() {
                self.pixel_x = self.target_x; self.travel_x = 0; self.velocity_x = 0;
                finished = true;
            }
            if self.travel_y != 0 && (self.target_y - self.pixel_y).signum() != self.velocity_y.signum() {
                self.pixel_y = self.target_y; self.travel_y = 0; self.velocity_y = 0;
                finished = true;
            }

            // Step complete: sprite is pixel-aligned to the tile center.
            // Evaluate tile events unless we just landed from a warp
            // (avoiding an infinite warp loop on the arrival tile).
            if finished && !self.is_moving() {
                if self.last_warp_tile != Some((self.tile_x, self.tile_y)) {
                    let event_result = check_tile_events(self, map, events, self.tile_x, self.tile_y);
                    if let MapEventResult::Warp(dest_map, x, y, facing) = event_result {
                        self.pending_warp = Some((dest_map, x, y, facing));
                    }
                }
            }
        }

        // Two-frame walk cycle. The counter advances twice as fast while moving,
        // so the sprite toggles every 10 ticks walking and every 20 ticks idle.
        let is_walking = self.is_moving();
        let speed = if is_walking { 2 } else { 1 }; 
        
        self.anim_counter = self.anim_counter.wrapping_add(speed);
        
        if self.anim_counter >= 20 {
            self.anim_counter = 0;
            self.mapsprite_frame = if self.mapsprite_frame == 0 { 1 } else { 0 };
        }
    }
}

pub struct PlayerSprites {
    pub down: Vec<u8>,
    pub up: Vec<u8>,
    pub side: Vec<u8>,
}

/// Load and decompress one map-sprite sheet (Basic compression).
/// Missing files log a warning and yield an empty vec so the game can still
/// run without a complete asset set.
fn load_sheet(name: &str) -> Vec<u8> {
    let path = format!("src/data/graphics/mapsprites/{}", name);
    let raw = match std::fs::read(&path) {
        Ok(r) => r,
        Err(_) => { eprintln!("[player] Файл не найден: {}", name); return vec![]; }
    };
    let d = decompressor::decompress_basic(&raw);
    println!("[player] {} -> {} байт", name, d.len());
    d
}

/// Transpose a 3x3 grid of tiles from column-major to row-major order.
/// SF2 sprite sheets store the 9 tiles column-first, but the renderer indexes
/// them as (row, col), so a transpose is needed per frame.
fn reorder_tiles(tiles: &mut [u8]) {
    if tiles.len() < FRAME_BYTES { return; }
    let mut result = vec![0u8; FRAME_BYTES];
    for ty in 0..3 {
        for tx in 0..3 {
            let src = (tx * 3 + ty) * TILE_BYTES;
            let dst = (ty * 3 + tx) * TILE_BYTES;
            result[dst..dst + TILE_BYTES].copy_from_slice(&tiles[src..src + TILE_BYTES]);
        }
    }
    tiles.copy_from_slice(&result);
}

/// Load all directional walk sheets and normalize their tile order.
pub fn load_player_sprites() -> PlayerSprites {
    let mut up   = load_sheet("mapsprite000-0.bin");
    let mut side = load_sheet("mapsprite000-1.bin");
    let mut down = load_sheet("mapsprite000-2.bin");

    for sheet in [&mut down, &mut up, &mut side] {
        let frames = sheet.len() / FRAME_BYTES;
        for f in 0..frames {
            let start = f * FRAME_BYTES;
            let end = (f + 1) * FRAME_BYTES;
            reorder_tiles(&mut sheet[start..end]);
        }
    }
    PlayerSprites { down, up, side }
}

/// Draw the player sprite at its current position, offset by the camera scroll.
/// The sprite is a 3x3 tile grid (24x24 px). The `side` sheet is drawn facing
/// left and horizontally mirrored for right-facing frames. `palette` is the
/// 4-bank sprite palette; index 0 is treated as transparent.
pub fn draw_player(
    buf: &mut [u8], pitch: usize, sprites: &PlayerSprites,
    state: &PlayerState, scroll_x: i32, scroll_y: i32, palette: &[Color],
) {
    // Undo the fixed-point scaling to get real screen pixels.
    let real_px = state.pixel_x / 16;
    let real_py = state.pixel_y / 16;

    // `side` sheet faces left natively; mirror it for right-facing.
    let (sheet, h_flip) = match state.facing {
        Facing::Down  => (&sprites.down, false),
        Facing::Up    => (&sprites.up,   false),
        Facing::Right => (&sprites.side, true),
        Facing::Left  => (&sprites.side, false),
    };

    if sheet.is_empty() { return; }

    let frame_count = (sheet.len() / FRAME_BYTES).max(1);
    let frame_idx = (state.mapsprite_frame as usize) % frame_count;
    let frame_offset = frame_idx * FRAME_BYTES;

    let px_base = real_px - scroll_x;
    
    // Constant vertical anchor: the sprite sits 8 px above the tile origin so
    // the character's feet align with the base of the tile. No jump/bob offset
    // is applied — matches SF2's map-walk sprite alignment.
    let py_base = real_py - scroll_y - 8; 

    for ty in 0..3 {
        for tx_iter in 0..3 {
            // Reverse the horizontal tile order when mirroring.
            let tx = if h_flip { 2 - tx_iter } else { tx_iter };
            let tile_idx = frame_offset + (ty * 3 + tx) * TILE_BYTES;
            if tile_idx + TILE_BYTES > sheet.len() { continue; }
            let tile = &sheet[tile_idx..tile_idx + TILE_BYTES];

            let draw_x = px_base + (tx_iter as i32 * 8);
            let draw_y = py_base + (ty as i32 * 8);

            // 4bpp tile: 2 pixels per byte, high nibble = left pixel.
            for row in 0..8 {
                for col in 0..8 {
                    let src_col = if h_flip { 7 - col } else { col };
                    let byte_idx = row * 4 + (src_col / 2);
                    let byte = tile[byte_idx];
                    let ci = if src_col % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F } as usize;
                    if ci == 0 || ci >= palette.len() { continue; }
                    let c = palette[ci];

                    let px = draw_x + col as i32;
                    let py = draw_y + row as i32;
                    // Clip to the visible map viewport. The leftmost 32 px is
                    // reserved for the HUD/side window, so drawing starts at x=32.
                    if px >= 32 && px < 288 && py >= 0 && py < 224 {
                        let off = py as usize * pitch + px as usize * 4;
                        buf[off] = c.r; buf[off+1] = c.g; buf[off+2] = c.b; buf[off+3] = 255;
                    }
                }
            }
        }
    }
}