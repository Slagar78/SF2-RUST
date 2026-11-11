// src/game/render_mode.rs

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RenderMode {
    Original,
    Normal,
}

impl RenderMode {
    pub fn window_size(self) -> (u32, u32) {
        match self {
            RenderMode::Original => (320, 224),
            RenderMode::Normal => (768, 672),
        }
    }
}