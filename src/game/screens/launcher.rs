// src/game/screens/launcher.rs
use crate::engine::renderer::Renderer;
use crate::engine::input::{InputState, INPUT_UP, INPUT_DOWN, INPUT_A, INPUT_C, INPUT_START};
use crate::game::render_mode::RenderMode;
use sdl3::pixels::Color;

pub struct LauncherState {
    pub selected: usize,
    pub last_input: u8,
    pub confirmed: Option<RenderMode>,
}

impl LauncherState {
    pub fn new() -> Self { Self { selected: 0, last_input: 0, confirmed: None } }
}

pub fn update_and_draw(renderer: &mut Renderer, state: &mut LauncherState, input: &InputState) {
    let cur = input.current_player_input;

    // Навигация вверх/вниз
    if cur & INPUT_UP != 0 && state.last_input & INPUT_UP == 0 && state.selected > 0 {
        state.selected -= 1;
    }
    if cur & INPUT_DOWN != 0 && state.last_input & INPUT_DOWN == 0 && state.selected < 1 {
        state.selected += 1;
    }

    // Подтверждение: A, C (кнопка D) или START (Enter)
    let confirm_buttons = INPUT_A | INPUT_C | INPUT_START;
    let confirm_pressed = (cur & confirm_buttons) != 0;
    let confirm_was_pressed = (state.last_input & confirm_buttons) != 0;

    if confirm_pressed && !confirm_was_pressed {
        state.confirmed = Some(if state.selected == 0 {
            RenderMode::Normal
        } else {
            RenderMode::Original
        });
    }
    state.last_input = cur;

    // Рисование
    renderer.texture.with_lock(None, |buf, pitch| {
        // Заливаем всё черным
        for y in 0..224 {
            for x in 0..320 {
                let o = y * pitch + x * 4;
                buf[o] = 0; buf[o+1] = 0; buf[o+2] = 0; buf[o+3] = 255;
            }
        }

        // Рисуем скругленные кнопки (синяя и серая)
        draw_rounded_btn(buf, pitch, 50, 70, 220, 36, state.selected == 0);
        draw_rounded_btn(buf, pitch, 50, 120, 220, 36, state.selected == 1);

        // Цвет текста - белый
        let text_color = Color::RGB(255, 255, 255); 
        
        // Рисуем текст крупнее (scale = 2)
        // NORMAL: 6 букв * (6*2) = 72. Центр 160. Старт: 160 - 36 = 124
        draw_text(buf, pitch, "NORMAL", 124, 82, text_color, 2);
        
        // ORIGINAL: 8 букв * (6*2) = 96. Центр 160. Старт: 160 - 48 = 112
        draw_text(buf, pitch, "ORIGINAL", 112, 132, text_color, 2);
    }).unwrap();
}

// Функция для рисования кнопки со скругленными углами
fn draw_rounded_btn(buf: &mut [u8], pitch: usize, x: i32, y: i32, w: i32, h: i32, active: bool) {
    // Синий (50, 150, 255) для выбранной, темно-серый (80, 80, 80) для невыбранной
    let (r, g, b) = if active { (50, 150, 255) } else { (80, 80, 80) };
    let radius = 6;

    for dy in 0..h {
        for dx in 0..w {
            let mut skip = false;
            if dx < radius && dy < radius {
                if (radius - dx)*(radius - dx) + (radius - dy)*(radius - dy) > radius * radius { skip = true; }
            } else if dx >= w - radius && dy < radius {
                if (dx - (w - radius - 1))*(dx - (w - radius - 1)) + (radius - dy)*(radius - dy) > radius * radius { skip = true; }
            } else if dx < radius && dy >= h - radius {
                if (radius - dx)*(radius - dx) + (dy - (h - radius - 1))*(dy - (h - radius - 1)) > radius * radius { skip = true; }
            } else if dx >= w - radius && dy >= h - radius {
                if (dx - (w - radius - 1))*(dx - (w - radius - 1)) + (dy - (h - radius - 1))*(dy - (h - radius - 1)) > radius * radius { skip = true; }
            }

            if skip { continue; }

            let px = x + dx;
            let py = y + dy;
            if px >= 0 && px < 320 && py >= 0 && py < 224 {
                let o = py as usize * pitch + px as usize * 4;
                buf[o] = r; buf[o+1] = g; buf[o+2] = b; buf[o+3] = 255;
            }
        }
    }
}

// Встроенный шрифт 5x7 с поддержкой масштабирования
fn draw_text(buf: &mut [u8], pitch: usize, text: &str, mut x: i32, y: i32, color: Color, scale: i32) {
    for ch in text.chars() {
        let bitmap = match ch {
            'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
            'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
            'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
            'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
            'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
            'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
            'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
            'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
            _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        };

        for (row, &bits) in bitmap.iter().enumerate() {
            for col in 0..5 {
                if (bits & (1 << (4 - col))) != 0 {
                    // Рисуем увеличенные пиксели (scale x scale)
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = x + (col * scale) + sx;
                            let py = y + (row as i32 * scale) + sy;
                            if px >= 0 && px < 320 && py >= 0 && py < 224 {
                                let o = py as usize * pitch + px as usize * 4;
                                buf[o] = color.r; buf[o+1] = color.g; buf[o+2] = color.b; buf[o+3] = 255;
                            }
                        }
                    }
                }
            }
        }
        // Сдвиг для следующей буквы (5 ширины + 1 пробел = 6, умножаем на масштаб)
        x += 6 * scale; 
    }
}