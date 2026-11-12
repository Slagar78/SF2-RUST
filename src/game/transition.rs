// src/game/transition.rs

#[derive(Clone, Copy, PartialEq)]
pub enum FadeState {
    None,
    FadeOut,
    Hold,
    FadeIn,
}

pub struct TransitionState {
    pub state: FadeState,
    /// Screen darkness: 0.0 = fully visible, 1.0 = fully black.
    pub alpha: f32,
    /// Remaining time to stay on a fully black screen before fading in.
    pub hold_timer: f32,
    /// Pending teleport request: (destination_map, tile_x, tile_y, facing).
    /// Consumed exactly once, when the screen reaches full black.
    pub pending_warp: Option<(u8, u8, u8, u8)>,
}

impl TransitionState {

    pub fn is_fading_out(&self) -> bool {
        self.state == FadeState::FadeOut
    }

    pub fn new() -> Self {
        Self {
            state: FadeState::None,
            alpha: 0.0,
            hold_timer: 0.0,
            pending_warp: None,
        }
    }

    /// Begin a warp transition. Ignored if a transition is already in progress,
    /// so overlapping warp requests do not corrupt the current fade.
    pub fn start_warp(&mut self, dest_map: u8, x: u8, y: u8, facing: u8) {
        if self.state == FadeState::None {
            self.pending_warp = Some((dest_map, x, y, facing));
            self.state = FadeState::FadeOut;
            self.alpha = 0.0;
        }
    }

    /// Advance the fade state machine.
    /// Returns the pending warp payload once the screen is fully black, which is
    /// the signal for the caller to physically relocate the player. The payload is
    /// taken (moved out) so it is delivered exactly once.
    pub fn update(&mut self, dt: f32) -> Option<(u8, u8, u8, u8)> {
        match self.state {
            FadeState::None => {}
            FadeState::FadeOut => {
                // Fade to black at ~3.0 units/s (roughly 0.33 s total).
                self.alpha += 3.0 * dt;
                if self.alpha >= 1.0 {
                    self.alpha = 1.0;
                    self.state = FadeState::Hold;
                    self.hold_timer = 0.2; // Brief pause on a fully black screen.
                    return self.pending_warp.take();
                }
            }
            FadeState::Hold => {
                self.hold_timer -= dt;
                if self.hold_timer <= 0.0 {
                    self.state = FadeState::FadeIn;
                }
            }
            FadeState::FadeIn => {
                // Fade back in at the same rate as the fade-out.
                self.alpha -= 3.0 * dt;
                if self.alpha <= 0.0 {
                    self.alpha = 0.0;
                    self.state = FadeState::None;
                }
            }
        }
        None
    }

    /// Darken the entire 320x224 framebuffer in place by `alpha`.
    /// The buffer is expected to be 32-bit RGBA, row-major, `pitch` bytes per row.
    pub fn draw(&self, buffer: &mut [u8], pitch: usize) {
        if self.alpha <= 0.0 { return; }

        // Remaining brightness of each channel, in the range 0.0..=1.0.
        let dark_factor = 1.0 - self.alpha;

        for y in 0..224 {
            for x in 0..320 {
                let offset = y * pitch + x * 4;
                // Bounds check guards against buffers shorter than the expected frame.
                if offset + 3 < buffer.len() {
                    buffer[offset] = (buffer[offset] as f32 * dark_factor) as u8;
                    buffer[offset + 1] = (buffer[offset + 1] as f32 * dark_factor) as u8;
                    buffer[offset + 2] = (buffer[offset + 2] as f32 * dark_factor) as u8;
                }
            }
        }
    }
}