// src/engine/input.rs
#![allow(dead_code)]

use sdl3::keyboard::Keycode;
use std::collections::HashSet;
use std::time::Duration;

// Bitmask for the Sega Genesis 3-button controller layout, plus START.
// Bit assignments must stay stable: they are OR-ed together and passed
// directly into game logic that tests individual bits.
pub const INPUT_UP: u8    = 0x01;
pub const INPUT_DOWN: u8  = 0x02;
pub const INPUT_LEFT: u8  = 0x04;
pub const INPUT_RIGHT: u8 = 0x08;
pub const INPUT_B: u8     = 0x10; // Button B (Cancel / back)
pub const INPUT_C: u8     = 0x20; // Button C (confirm, in combination with A)
pub const INPUT_A: u8     = 0x40; // Button A (Menu / confirm)
pub const INPUT_START: u8 = 0x80; // Start

// Mask covering every valid input bit; used for "any input at all?" checks.
pub const ALL_BUTTONS_MASK: u8 = INPUT_UP | INPUT_DOWN | INPUT_LEFT | INPUT_RIGHT | INPUT_B | INPUT_C | INPUT_A | INPUT_START;

pub struct InputState {
    /// Raw button state as sampled from the keyboard this frame.
    pub player_1_input: u8,
    /// Input snapshot consumed by gameplay code; may diverge from
    /// `player_1_input` when the SF2 input-clearing routine (`sub_15a4`)
    /// suppresses repeated presses.
    pub current_player_input: u8,
    /// Emulates RAM byte $FFDE9E from the original game, which tracks the
    /// last-known button state for edge detection.
    pub byte_ffde9e: u8,
    /// Emulates RAM byte $FFDE9F: how many frames a held button has been
    /// suppressed by `sub_15a4`.
    pub byte_ffde9f: u8,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            player_1_input: 0,
            current_player_input: 0,
            byte_ffde9e: 0,
            byte_ffde9f: 0,
        }
    }

    /// Sample the currently held keys into the input bitmask.
    /// Called once per event-pump iteration; the resulting mask is what
    /// gameplay code inspects via `current_player_input`.
    pub fn update_player_inputs(&mut self, keys: &HashSet<Keycode>) {
        let mut p1: u8 = 0;
        
        // Directional pad.
        if keys.contains(&Keycode::Up)    { p1 |= INPUT_UP; }
        if keys.contains(&Keycode::Down)  { p1 |= INPUT_DOWN; }
        if keys.contains(&Keycode::Left)  { p1 |= INPUT_LEFT; }
        if keys.contains(&Keycode::Right) { p1 |= INPUT_RIGHT; }
        
        // Sega Genesis face buttons, mapped to A/S/D on the keyboard.
        if keys.contains(&Keycode::A)     { p1 |= INPUT_A; }
        if keys.contains(&Keycode::S)     { p1 |= INPUT_B; }
        if keys.contains(&Keycode::D)     { p1 |= INPUT_C; }
        
        if keys.contains(&Keycode::Return) { p1 |= INPUT_START; }

        self.player_1_input = p1;
        self.current_player_input = p1;
    }

    /// Approximate one VBlank interval (~60 Hz). Used by the blocking wait
    /// loops below, which emulate SF2's busy-wait input routines.
    fn wait_for_vint() {
        std::thread::sleep(Duration::from_millis(16));
    }

    /// Block until any button is pressed.
    /// Mirrors SF2's "wait for input" idiom; do not use outside startup /
    /// scripted sequences, since it stalls the game loop.
    pub fn wait_for_player_input(&mut self) {
        loop {
            if (self.current_player_input & ALL_BUTTONS_MASK) != 0 {
                break;
            }
            Self::wait_for_vint();
        }
    }

    /// Block until a *new* press occurs: first wait for all buttons to be
    /// released, then wait for any button to be pressed. This filters out
    /// a held button from the previous action.
    pub fn wait_for_player1_new_input(&mut self) {
        loop {
            if (self.player_1_input & ALL_BUTTONS_MASK) == 0 {
                break;
            }
            Self::wait_for_vint();
        }
        loop {
            if (self.player_1_input & ALL_BUTTONS_MASK) != 0 {
                break;
            }
            Self::wait_for_vint();
        }
    }

    /// Port of SF2's `sub_15A4` input-suppression routine.
    ///
    /// If a button is held continuously, keep counting frames in $FFDE9F
    /// and, for the first 10 frames, clear `player_1_input` so the press is
    /// not seen by gameplay. After 10 frames the counter is released, which
    /// lets the button register again (starting a new "press" edge).
    /// When the button is released, both tracking bytes are cleared.
    pub fn sub_15a4(&mut self) {
        let d7 = self.player_1_input & self.byte_ffde9e;
        if d7 != 0 {
            self.byte_ffde9f = self.byte_ffde9f.wrapping_add(1);
            if self.byte_ffde9f < 0xA {
                self.player_1_input = 0;
                return;
            }
        }
        self.byte_ffde9e = 0;
        self.byte_ffde9f = 0;
    }

    /// Block for up to ~1 second of VBlanks (59 frames), or until input.
    pub fn wait_for_input_for_1_second(&mut self) {
        for _ in 0..59 {
            if (self.player_1_input & ALL_BUTTONS_MASK) != 0 {
                break;
            }
            Self::wait_for_vint();
        }
    }

    /// Block for up to ~3 seconds of VBlanks (179 frames), or until input.
    pub fn wait_for_input_for_3_seconds(&mut self) {
        for _ in 0..179 {
            if (self.player_1_input & ALL_BUTTONS_MASK) != 0 {
                break;
            }
            Self::wait_for_vint();
        }
    }
}