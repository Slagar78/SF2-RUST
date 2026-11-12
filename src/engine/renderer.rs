// src/engine/renderer.rs
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;
use crate::game::render_mode::RenderMode;

// Обертка, которая запоминает пиксели в оперативку, чтобы xBRZ мог их прочитать
pub struct RendererTexture {
    pub texture: Texture<'static>,
    pub framebuffer: Vec<u8>,
}

impl RendererTexture {
    pub fn with_lock<F>(&mut self, rect: Option<sdl3::rect::Rect>, f: F) -> Result<(), sdl3::Error>
    where F: FnOnce(&mut [u8], usize)
    {
        let framebuffer = &mut self.framebuffer;
        self.texture.with_lock(rect, |buffer, pitch| {
            f(buffer, pitch);
            // Копируем нарисованное в RAM
            for y in 0..224 {
                let src_off = y * pitch;
                let dst_off = y * 320 * 4;
                framebuffer[dst_off..dst_off + 320 * 4].copy_from_slice(&buffer[src_off..src_off + 320 * 4]);
            }
        })
    }
}

pub struct Renderer {
    pub canvas: Canvas<Window>,
    pub texture: RendererTexture,
    pub texture_xbrz: Texture<'static>,
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
        let mut texture_xbrz = texture_creator.create_texture_streaming(PixelFormat::RGBA32, 640, 448).unwrap();
        texture_xbrz.set_scale_mode(sdl3::render::ScaleMode::Linear);

        let texture = unsafe { std::mem::transmute::<Texture, Texture<'static>>(texture) };
        let texture_xbrz = unsafe { std::mem::transmute::<Texture, Texture<'static>>(texture_xbrz) };

        let texture = RendererTexture {
            texture,
            framebuffer: vec![0u8; 320 * 224 * 4],
        };

        Self { canvas, texture, texture_xbrz, mode }
    }

    pub fn present(&mut self, fill_screen: bool, crop_sides: bool) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();

        if self.mode == RenderMode::Normal {
            // Берем готовый кадр из RAM и увеличиваем
            let scaled = xbrz::scale_rgba(&self.texture.framebuffer, 320, 224, 2);
            
            // Заливаем результат в текстуру экрана
            self.texture_xbrz.update(None, &scaled, 640 * 4).unwrap();

            if !fill_screen {
                let dst = sdl3::render::FRect::new(64.0, 112.0, 640.0, 448.0);
                self.canvas.copy(&self.texture_xbrz, None, Some(dst)).unwrap();
            } else if crop_sides {
                let src = sdl3::render::FRect::new(32.0 * 2.0, 0.0, 256.0 * 2.0, 224.0 * 2.0);
                self.canvas.copy(&self.texture_xbrz, Some(src), None).unwrap();
            } else {
                self.canvas.copy(&self.texture_xbrz, None, None).unwrap();
            }
        } else {
            // Original (как было)
            self.canvas.copy(&self.texture.texture, None, None).unwrap();
        }

        self.canvas.present();
    }
}