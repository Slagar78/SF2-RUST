// src/engine/renderer.rs
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;
use crate::game::render_mode::RenderMode;

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
    last_hash: u64,
    cache_valid: bool,
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
        let mut texture_xbrz = texture_creator.create_texture_streaming(PixelFormat::RGBA32, 768, 672).unwrap();
        texture_xbrz.set_scale_mode(sdl3::render::ScaleMode::Nearest);

        let texture = unsafe { std::mem::transmute::<Texture, Texture<'static>>(texture) };
        let texture_xbrz = unsafe { std::mem::transmute::<Texture, Texture<'static>>(texture_xbrz) };

        let texture = RendererTexture {
            texture,
            framebuffer: vec![0u8; 320 * 224 * 4],
        };

        Self {
            canvas, texture, texture_xbrz, mode,
            last_hash: 0,
            cache_valid: false,
        }
    }

    pub fn present(&mut self, _fill_screen: bool, _crop_sides: bool) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();

        if self.mode == RenderMode::Normal {
            use std::hash::Hasher;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            hasher.write(&self.texture.framebuffer);
            let hash = hasher.finish();

            if !self.cache_valid || hash != self.last_hash {
                // 1. Извлекаем игровое поле 256×224 из буфера 320×224
                let mut content = vec![0u8; 256 * 224 * 4];
                for y in 0..224 {
                    let src = y * 320 * 4 + 32 * 4;
                    let dst = y * 256 * 4;
                    content[dst..dst + 256 * 4].copy_from_slice(&self.texture.framebuffer[src..src + 256 * 4]);
                }

                // 2. xBRZ 2x: 256×224 → 512×448 (меньше сглаживания, чем 3x)
                let scaled_2x = xbrz::scale_rgba(&content, 256, 224, 2);

                // 3. Nearest до 512×448 → 768×672 (коэффициент 1.5)
                let mut final_scaled = vec![0u8; 768 * 672 * 4];
                for y in 0..672usize {
                    let src_y = (y as f32 / 1.5) as usize;
                    for x in 0..768usize {
                        let src_x = (x as f32 / 1.5) as usize;
                        let src_idx = (src_y * 512 + src_x) * 4;
                        let dst_idx = (y * 768 + x) * 4;
                        final_scaled[dst_idx..dst_idx + 4]
                            .copy_from_slice(&scaled_2x[src_idx..src_idx + 4]);
                    }
                }

                self.texture_xbrz.update(None, &final_scaled, 768 * 4).unwrap();
                self.last_hash = hash;
                self.cache_valid = true;
            }

            self.canvas.copy(&self.texture_xbrz, None, None).unwrap();
        } else {
            self.canvas.copy(&self.texture.texture, None, None).unwrap();
        }

        self.canvas.present();
    }
}