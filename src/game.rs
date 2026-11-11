// src/game.rs
pub mod state;
pub mod screens;
pub mod map;
pub mod player;
pub mod menu;
pub mod ally_parser;
pub mod item_parser;
pub mod transition;

use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use std::time::Duration;
use std::collections::HashSet;
use crate::engine::renderer::Renderer;
use crate::engine::input::{InputState, INPUT_A, INPUT_B, INPUT_C, INPUT_UP, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT};
use crate::engine::base_tiles;
use crate::engine::decompressor;
use crate::engine::time::TimeState;
use crate::game::state::GameState;
use crate::game::player::{PlayerState, MoveResult, Facing, TILE_SIZE_FIXED};
use crate::game::map::{MapState, load_map_data};
use crate::game::map::events::MapEventsState;
use crate::game::menu::window_engine::WindowEngine;
use crate::game::menu::layouts;
use crate::game::menu::member_menu::MemberMenuState;
use crate::game::menu::member_status_menu::MemberStatusState;
use crate::game::ally_parser::load_force_members;

// Genesis 3-bit color levels expanded to 8-bit RGB values.
const CRAM_VALUE_MAP: [u8; 8] = [0, 52, 87, 116, 144, 172, 206, 255];

// Decode a Genesis/Mega Drive color word stored as two bytes into SDL RGB.
fn decode_genesis_color(first: u8, second: u8) -> sdl3::pixels::Color {
    let word = ((first as u16) << 8) | (second as u16);
    let r3 = ((word >> 1) & 0x07) as usize;
    let g3 = ((word >> 5) & 0x07) as usize;
    let b3 = ((word >> 9) & 0x07) as usize;
    sdl3::pixels::Color::RGB(CRAM_VALUE_MAP[r3], CRAM_VALUE_MAP[g3], CRAM_VALUE_MAP[b3])
}

pub fn run() {
    let sdl_context = sdl3::init().unwrap();
    let mut renderer = Renderer::new(&sdl_context);
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Load the base VRAM tile set required by the renderer.
    let mut vram = base_tiles::load_base_tiles().expect("Нет basetiles.bin");

    if let Ok(raw_menu_tiles) = std::fs::read("src/data/graphics/tech/menus/mainmenutiles.bin") {
        let d = decompressor::decompress(&raw_menu_tiles).unwrap_or(raw_menu_tiles);
        if d.len() >= 3456 {
            // Используем VRAM_DIAMOND_MENU (0x38000) + смещения
            let base = crate::engine::base_tiles::VRAM_DIAMOND_MENU;
            vram.write_bytes(base, &d[2880..3456]).expect("Ошибка Up");
            vram.write_bytes(base + 0x240, &d[576..1152]).expect("Ошибка Left");
            vram.write_bytes(base + 0x480, &d[1152..1728]).expect("Ошибка Right");
            vram.write_bytes(base + 0x6C0, &d[2304..2880]).expect("Ошибка Down");
        }
    }

    let mut map_data = load_map_data(3);
    let mut events_state = MapEventsState::new();
    
    let mut player_state = PlayerState::new(56, 3);
    let mut map_state = MapState::new(player_state.pixel_x / 16, player_state.pixel_y / 16);
    
    let player_sprites = player::load_player_sprites();
    
    let mut sprite_palette = vec![sdl3::pixels::Color::RGB(0,0,0); 64];
    let pal_data = std::fs::read("src/data/graphics/tech/basepalette.bin").expect("Нет basepalette.bin");
    for i in 0..16 {
        let c = decode_genesis_color(pal_data[i*2], pal_data[i*2+1]);
        sprite_palette[i] = c;
        sprite_palette[i + 16] = c;
        sprite_palette[i + 32] = c;
        sprite_palette[i + 48] = c;
    }
    
    let mut input_state = InputState::new();
    let mut pressed_keys = HashSet::new();
    
    let mut game_state = GameState::SegaLogo;
    let mut frame_counter = 0;

    let mut time_state = TimeState::new(59.92);

    let mut window_engine = WindowEngine::new();
    let mut menu_open = false;
    let mut menu_anim_timer: u16 = 20;
    let mut menu_choice: usize = 0;
    let mut last_input: u8 = 0;
    
    let mut diamond_menu_slot: Option<usize> = None;

    let mut member_menu = MemberMenuState::new();
    let mut member_status = MemberStatusState::new();
    let force_members = load_force_members();
    let _all_items = item_parser::load_items();

    let mut transition = transition::TransitionState::new();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} => break 'running,
                Event::KeyDown { keycode: Some(k), .. } => {
                    pressed_keys.insert(k);
                    
                    if k == Keycode::Escape {
                        if game_state == GameState::Menu || game_state == GameState::MemberMenu || game_state == GameState::MemberStatus {
                            if let Some(slot) = diamond_menu_slot { window_engine.delete_window(slot); }
                            member_menu.close(&mut window_engine);
                            member_status.close(&mut window_engine);
                            game_state = GameState::Map;
                            menu_open = false;
                        } else {
                            break 'running;
                        }
                    }
                    
                    if k == Keycode::Return || k == Keycode::Space {
                        match game_state {
                            GameState::SegaLogo => { game_state = GameState::TitleScreen; frame_counter = 0; }
                            GameState::TitleScreen => { game_state = GameState::WitchScreen; frame_counter = 0; }
                            GameState::WitchScreen => { game_state = GameState::Map; }
                            _ => {}
                        }
                    }
                }
                Event::KeyUp { keycode: Some(k), .. } => {
                    pressed_keys.remove(&k);
                }
                _ => {}
            }
        }

        input_state.update_player_inputs(&pressed_keys);

        time_state.update();
        let mut should_render = false;

        while time_state.can_tick() {
            frame_counter += 1;
            window_engine.tick();

            match game_state {
                GameState::SegaLogo => {
                    screens::segalogo::update_and_draw(&mut renderer, frame_counter);
                    if frame_counter > 200 { game_state = GameState::TitleScreen; frame_counter = 0; }
                    should_render = true; 
                }
                GameState::TitleScreen => {
                    screens::title::update_and_draw(&mut renderer, frame_counter);
                    should_render = true; 
                }
                GameState::WitchScreen => {
                    screens::witch::update_and_draw(&mut renderer, &vram, &input_state);
                    should_render = true; 
                }
                GameState::Map => {
                    // Открываем крест-меню по A или C
                    if ((input_state.current_player_input & (INPUT_A | INPUT_C)) != 0) && !menu_open {
                        if !player_state.is_moving() {
                            player_state.interact(&mut map_data);
                        }
                        game_state = GameState::Menu;
                        menu_open = true;
                        menu_anim_timer = 20;
                        menu_choice = 0;
                        last_input = input_state.current_player_input; 
                        
                        // Создаем крест меню ЗА ЭКРАНОМ (снизу) и двигаем наверх
                        diamond_menu_slot = window_engine.create_window_with_layout(0x0C20, 18, 6, &layouts::LAYOUT_DIAMOND_MENU);
                        if let Some(slot) = diamond_menu_slot {
                            window_engine.move_window(slot, 0x0C15, 4);
                        }
                    }

                    if !player_state.is_moving() {
                        let move_res = player_state.try_move(input_state.current_player_input, &mut map_data, &mut events_state);
                        
                        if let MoveResult::Warp(dest_map, x, y, facing) = move_res {
                            println!("WARP INITIATED! Target map: {}, X: {}, Y: {}", dest_map, x, y);
                            transition.start_warp(dest_map, x, y, facing);
                        }
                    }
                    
                    if let Some((dest_map, tx, ty, facing)) = transition.update(1.0 / 60.0) {
                        if dest_map != 255 {
                            println!("Загрузка карты через варп: {}", dest_map);
                            events_state.reset(); 
                            map_data = load_map_data(dest_map);
                        }

                        player_state.tile_x = tx as i32;
                        player_state.tile_y = ty as i32;
                        player_state.pixel_x = player_state.tile_x * TILE_SIZE_FIXED;
                        player_state.pixel_y = player_state.tile_y * TILE_SIZE_FIXED;
                        player_state.target_x = player_state.pixel_x;
                        player_state.target_y = player_state.pixel_y;
                        player_state.travel_x = 0;
                        player_state.travel_y = 0;
                        player_state.velocity_x = 0;
                        player_state.velocity_y = 0;
                        player_state.current_direction = None;
                        player_state.input_delayer = 10;

                        match facing {
                            0 => player_state.facing = Facing::Up,
                            1 => player_state.facing = Facing::Down,
                            2 => player_state.facing = Facing::Left,
                            3 => player_state.facing = Facing::Right,
                            _ => {}
                        }

                        map_state = MapState::new_for_warp(tx as i32, ty as i32, &map_data);
                        events_state.update_roofs_on_warp(&mut map_data, tx as i32, ty as i32);

                        let _ = crate::game::map::events::check_tile_events(
                            &mut player_state,
                            &mut map_data,
                            &mut events_state,
                            tx as i32,
                            ty as i32,
                        );
                    }

                    player_state.tick(&mut map_data, &mut events_state);

                    for anim in &mut map_data.animations {
                        anim.update(&mut map_data.tilesets);
                    }

                    map_state.player_x = (player_state.pixel_x / 16) + 12;
                    map_state.player_y = (player_state.pixel_y / 16) + 12;
                    map_state.update_target(&map_data);
                    map_state.tick();
                    should_render = true;
                }
                GameState::Menu => {
                    // БЛОКИРУЕМ ВВОД, ПОКА КРЕСТ НЕ ВЫЕХАЛ ДО КОНЦА
                    let mut is_animating = false;
                    if let Some(slot) = diamond_menu_slot {
                        if window_engine.entries[slot].is_animating() { is_animating = true; }
                    }

                    if !is_animating {
                        if (input_state.current_player_input & INPUT_B) != 0 {
                            game_state = GameState::Map;
                            menu_open = false;
                            if let Some(slot) = diamond_menu_slot {
                                window_engine.delete_window(slot);
                                diamond_menu_slot = None;
                            }
                        }

                        let current_input = input_state.current_player_input;
                        if current_input != last_input {
                            if (current_input & INPUT_UP) != 0 { menu_choice = 0; }
                            else if (current_input & INPUT_LEFT) != 0 { menu_choice = 1; }
                            else if (current_input & INPUT_RIGHT) != 0 { menu_choice = 2; }
                            else if (current_input & INPUT_DOWN) != 0 { menu_choice = 3; }

                            if (current_input & (INPUT_A | INPUT_C)) != 0 {
                                match menu_choice {
                                    0 => { // MEMBER
                                        if let Some(slot) = diamond_menu_slot {
                                            window_engine.delete_window(slot);
                                            diamond_menu_slot = None;
                                        }
                                        game_state = GameState::MemberMenu;
                                        member_menu.open(&mut window_engine, &mut vram, &mut sprite_palette);
                                    }
                                    1 => println!("Открыто меню МАГИИ"),
                                    2 => println!("Открыто меню ПРЕДМЕТОВ"),
                                    _ => {}
                                }
                            }
                            last_input = current_input;
                        } else if current_input == 0 {
                            last_input = 0;
                        }
                    }

                    player_state.tick(&mut map_data, &mut events_state);
                    menu_anim_timer = menu_anim_timer.saturating_sub(1);
                    if menu_anim_timer == 0 { menu_anim_timer = 20; }
                    should_render = true;
                }
                GameState::MemberMenu => {
                    // БЛОКИРУЕМ ВВОД, ПОКА ОКНА НЕ ВЫЕХАЛИ
                    let mut is_animating = false;
                    if let Some(s) = member_menu.window_slot_portrait { if window_engine.entries[s].is_animating() { is_animating = true; } }
                    if let Some(s) = member_menu.window_slot_summary { if window_engine.entries[s].is_animating() { is_animating = true; } }
                    if let Some(s) = member_menu.window_slot_list { if window_engine.entries[s].is_animating() { is_animating = true; } }

                    let current_input = input_state.current_player_input;

                    if !is_animating && !member_menu.closing {
                        if current_input != last_input {
                            if (current_input & (INPUT_A | INPUT_C)) != 0 {
                                member_menu.start_closing_animation(&mut window_engine);
                                member_menu.pending_status = true;
                                last_input = current_input;
                            } else if (current_input & INPUT_B) != 0 {
                                member_menu.start_closing_animation(&mut window_engine);
                                member_menu.pending_status = false;
                                last_input = current_input;
                            } else {
                                last_input = current_input;
                            }
                        } else if current_input == 0 {
                            last_input = 0;
                        }
                    }

                    if member_menu.closing {
                        let mut all_done = true;
                        if let Some(s) = member_menu.window_slot_portrait { if window_engine.entries[s].is_animating() { all_done = false; } }
                        if let Some(s) = member_menu.window_slot_summary { if window_engine.entries[s].is_animating() { all_done = false; } }
                        if let Some(s) = member_menu.window_slot_list { if window_engine.entries[s].is_animating() { all_done = false; } }
                        
                        if all_done && game_state == GameState::MemberMenu {
                            member_menu.close(&mut window_engine);
                            
                            if member_menu.pending_status {
                                member_menu.pending_status = false;
                                game_state = GameState::MemberStatus;
                                member_status.open(&mut window_engine, &mut vram, &mut sprite_palette);
                            } else {
                                game_state = GameState::Menu;
                                menu_open = true;
                                menu_anim_timer = 20;
                                menu_choice = 0;
                                diamond_menu_slot = window_engine.create_window_with_layout(0x0C20, 18, 6, &layouts::LAYOUT_DIAMOND_MENU);
                                if let Some(slot) = diamond_menu_slot {
                                    window_engine.move_window(slot, 0x0C15, 4);
                                }
                            }
                            last_input = current_input;
                        }
                    }

                    if !member_menu.closing {
                        member_menu.draw(&mut window_engine, &force_members);
                    }
                    should_render = true;
                }
                GameState::MemberStatus => {
                    let mut is_animating = false;
                    if let Some(s) = member_status.window_slot_portrait { if window_engine.entries[s].is_animating() { is_animating = true; } }
                    if let Some(s) = member_status.window_slot_status { if window_engine.entries[s].is_animating() { is_animating = true; } }
                    if let Some(s) = member_status.window_slot_kd { if window_engine.entries[s].is_animating() { is_animating = true; } }
                    if let Some(s) = member_status.window_slot_gold { if window_engine.entries[s].is_animating() { is_animating = true; } }

                    let current_input = input_state.current_player_input;

                    if !is_animating && !member_status.closing {
                        if current_input != last_input {
                            if (current_input & (INPUT_A | INPUT_B | INPUT_C)) != 0 {
                                member_status.start_closing_animation(&mut window_engine);
                                last_input = current_input;
                            } else {
                                last_input = current_input;
                            }
                        } else if current_input == 0 {
                            last_input = 0;
                        }
                    }

                    if member_status.closing {
                        let mut all_done = true;
                        if let Some(s) = member_status.window_slot_portrait { if window_engine.entries[s].is_animating() { all_done = false; } }
                        if let Some(s) = member_status.window_slot_status { if window_engine.entries[s].is_animating() { all_done = false; } }
                        if let Some(s) = member_status.window_slot_kd { if window_engine.entries[s].is_animating() { all_done = false; } }
                        if let Some(s) = member_status.window_slot_gold { if window_engine.entries[s].is_animating() { all_done = false; } }
                        
                        if all_done {
                            member_status.close(&mut window_engine);
                            game_state = GameState::MemberMenu; 
                            member_menu.open(&mut window_engine, &mut vram, &mut sprite_palette);
                            last_input = current_input;
                        }
                    } else {
                        member_status.tick_blink(&mut window_engine);
                    }

                    if !member_status.closing {
                        member_status.draw(&mut window_engine, &force_members[0]);
                    }
                    should_render = true;
                }
            }
        }

        if should_render {
            if game_state == GameState::Map || game_state == GameState::Menu || game_state == GameState::MemberMenu || game_state == GameState::MemberStatus {
                renderer.texture.with_lock(None, |buffer, pitch| {
                    map::draw::draw_map(buffer, pitch, &map_data, &map_state, 0);
                    player::draw_player(buffer, pitch, &player_sprites, &player_state, map_state.scroll_x - 32, map_state.scroll_y, &sprite_palette);
                    map::draw::draw_map(buffer, pitch, &map_data, &map_state, 1);
                    
                    if game_state == GameState::Menu || game_state == GameState::MemberMenu || game_state == GameState::MemberStatus {
                        window_engine.draw_windows(buffer, pitch, &vram, &sprite_palette, menu_anim_timer, menu_choice, 32);
                    }

                    if game_state == GameState::Menu {
                        if let Some(slot) = diamond_menu_slot {
                            menu::draw_menu::draw_menu(buffer, pitch, &mut window_engine, slot, menu_choice);
                        }
                    }

                    if game_state == GameState::MemberStatus && !member_status.closing {
                        member_status.draw_sprite_overlay(buffer, pitch, &vram, &window_engine, &sprite_palette);
                    }

                    transition.draw(buffer, pitch);
                }).unwrap();
            }
            
            renderer.present();
        }
        
        std::thread::sleep(Duration::from_millis(1));
    }
}