// src/engine/renderer.rs
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;

pub struct Renderer {
    pub canvas: Canvas<Window>,
    /// Streaming RGBA32 texture (320x224) that receives the software framebuffer.
    /// The lifetime is widened to `'static` via transmute; see `Renderer::new`
    /// for the safety rationale.
    pub texture: Texture<'static>,
}

impl Renderer {
    pub fn new(sdl_context: &sdl3::Sdl) -> Self {
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem.window("SF2 PC Port", 320, 224)
            .position_centered()
            .build()
            .unwrap();

        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        
        // Streaming texture: we lock it each frame and write RGBA pixels directly
        // into its backing buffer, then copy it to the canvas in `present`.
        let texture = texture_creator.create_texture_streaming(PixelFormat::RGBA32, 320, 224).unwrap();

        // SAFETY: `texture` would normally borrow from `texture_creator`, a local
        // that is dropped at the end of this function. The transmute widens its
        // lifetime to `'static` so it can be stored next to `canvas` in the struct.
        // This is sound because:
        //   1. `canvas` and `texture` are stored together and therefore share the
        //      same effective lifetime.
        //   2. The underlying SDL_Renderer is kept alive by shared ownership inside
        //      `Canvas` / `Texture`, so the `TextureCreator` local being dropped
        //      does not invalidate the texture.
        // If the struct layout ever changes (fields split apart, or `texture` moved
        // to a longer-lived owner), this transmute must be revisited.
        let texture = unsafe { std::mem::transmute::<Texture, Texture<'static>>(texture) };

        Self { canvas, texture }
    }

    /// Blit the streaming texture to the canvas and flip.
    /// Called once per rendered frame, after the framebuffer has been written.
    pub fn present(&mut self) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        self.canvas.copy(&self.texture, None, None).unwrap();
        self.canvas.present();
    }
}