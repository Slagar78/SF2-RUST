// src/game/map.rs
pub mod loader;
pub mod camera;
pub mod draw;
pub mod events;
pub mod anim;

pub use loader::load_map_data;
pub use camera::MapState;