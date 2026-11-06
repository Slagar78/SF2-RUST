// src/engine/time.rs
use std::time::Instant;

/// Fixed-timestep clock.
///
/// `update` accumulates real elapsed time, and `can_tick` drains it in
/// `frame_time` chunks. Callers typically loop:
///
///     time.update();
///     while time.can_tick() { /* step game logic */ }
///
/// This decouples simulation rate from render rate: the simulation always
/// advances in exact `frame_time` steps, and leftover time is carried over
/// to the next iteration.
pub struct TimeState {
    last_time: Instant,
    accumulator: f32,
    frame_time: f32,
}

impl TimeState {
    pub fn new(fps: f32) -> Self {
        Self {
            last_time: Instant::now(),
            accumulator: 0.0,
            frame_time: 1.0 / fps,
        }
    }

    /// Sample the wall clock and add the elapsed time to the accumulator.
    /// Called once per main-loop iteration.
    pub fn update(&mut self) {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        
        // Clamp large deltas to 0.25 s so a stalled window (breakpoint, drag,
        // OS sleep) cannot cause a huge burst of catch-up ticks.
        let delta_time = delta_time.min(0.25);
        self.accumulator += delta_time;
    }

    /// Consume one `frame_time` worth of accumulated time and return `true`
    /// if a logic tick is due. The caller should invoke this in a loop until
    /// it returns `false`, so all pending ticks are drained each iteration.
    pub fn can_tick(&mut self) -> bool {
        if self.accumulator >= self.frame_time {
            self.accumulator -= self.frame_time;
            true
        } else {
            false
        }
    }
}