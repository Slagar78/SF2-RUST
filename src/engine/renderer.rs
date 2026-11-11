// src/engine/renderer.rs
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;
use crate::game::render_mode::RenderMode;

pub struct Renderer {
    pub canvas: Canvas<Window>,
    /// Streaming RGBA32 texture (320x224) that receives the software framebuffer.
    pub texture: Texture<'static>,
    pub mode: RenderMode,
}

impl Renderer {
      pub fn new(sdl_context: &sdl3::Sdl, mode: RenderMode) -> Self {
          let video_subsystem = sdl_context.video().unwrap();
          let (w, h) = mode.window_size();
          let window = video_subsystem.window("SF2 PC Port", w, h)
              .position_centered()
              .build()
              .unwrap();

        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let texture = texture_creator.create_texture_streaming(PixelFormat::RGBA32, 320, 224).unwrap();
        let texture = unsafe { std::mem::transmute::<Texture, Texture<'static>>(texture) };

        Self { canvas, texture, mode }
    }

    pub fn present(&mut self, fill_screen: bool, crop_sides: bool) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        
        if self.mode == RenderMode::Normal {
            if !fill_screen {
                // SEGA LOGO (по центру)
                let dst = sdl3::render::FRect::new(64.0, 112.0, 640.0, 448.0);
                self.canvas.copy(&self.texture, None, Some(dst)).unwrap();
            } else if crop_sides {
                // ИГРА И МЕНЮ (вырезаем бока 320->256 и растягиваем)
                let src = sdl3::render::FRect::new(32.0, 0.0, 256.0, 224.0);
                self.canvas.copy(&self.texture, Some(src), None).unwrap();
            } else {
                // ТИТЛ И ВЕДЬМА (растягиваем на весь экран как есть)
                self.canvas.copy(&self.texture, None, None).unwrap();
            }
        } else {
            self.canvas.copy(&self.texture, None, None).unwrap();
        }
        
        self.canvas.present();
    }
}