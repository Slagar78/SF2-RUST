// src/game/flags.rs
#![allow(dead_code)]

pub struct GameFlags {
    flags: Vec<bool>,
}

impl GameFlags {
    pub fn new() -> Self {
        // В SF2 используется до 1024 флагов (от 0 до 1023)
        Self { flags: vec![false; 1024] }
    }

    pub fn get(&self, id: u16) -> bool {
        self.flags.get(id as usize).copied().unwrap_or(false)
    }

    pub fn set(&mut self, id: u16) {
        if (id as usize) < self.flags.len() {
            self.flags[id as usize] = true;
        }
    }

    pub fn clear(&mut self, id: u16) {
        if (id as usize) < self.flags.len() {
            self.flags[id as usize] = false;
        }
    }
}