// src/game/combatant.rs
#![allow(dead_code)]

pub struct Combatant {
    pub name: &'static str,
    pub class: &'static str,
    pub level: u8,
    pub hp: u8,
    pub max_hp: u8,
    pub mp: u8,
    pub max_mp: u8,
    pub attack: u8,
    pub defense: u8,
    pub agility: u8,
    pub movement: u8,
    pub items: Vec<&'static str>,
    pub spells: Vec<&'static str>,
}

pub fn get_force_members() -> Vec<Combatant> {
    vec![
        Combatant {
            name: "BOWIE",
            class: "SDMN",
            level: 1,
            hp: 16,
            max_hp: 16,
            mp: 0,
            max_mp: 0,
            attack: 6,
            defense: 4,
            agility: 4,
            movement: 6,
            items: vec!["Wooden Sword", "Medical Herb"],
            spells: vec!["EGRESS"],
        }
    ]
}