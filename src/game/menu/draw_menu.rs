// src/game/menu/draw_menu.rs
use crate::game::menu::window_engine::WindowEngine;

pub fn draw_menu(_buf: &mut [u8], _pitch: usize, engine: &mut WindowEngine, slot: usize, menu_choice: usize) {
    let entry = engine.get_window_entry(slot);
    if !entry.is_active { return; }

    // Строки из diamondmenu.asm (aMEMBER, aMAGIC, aITEM, aSEARCH)
    let menu_texts = ["MEMBER", "MAGIC", "ITEM", "SEARCH"];
    let text = menu_texts[menu_choice];

    // ОЧИЩАЕМ ОБЛАСТЬ (ровно 6 пробелов, как ширина окошка)
    engine.write_string(slot, 11, 4, "      "); 
    
    // ПИШЕМ НОВОЕ СЛОВО
    engine.write_string(slot, 11, 4, text);
}