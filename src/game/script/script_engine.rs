// src/game/script/script_engine.rs
use std::fs;
use std::collections::HashMap;
use crate::game::map::entities::MapEntity;

#[derive(Clone, Debug)]
pub enum ScriptCommand {
    Move { entity: String, dx: i32, dy: i32 },
    SetFacing { entity: String, dir: u8 },
    Wait,
    End,
}

pub struct ScriptEngine {
    pub scripts: HashMap<String, Vec<ScriptCommand>>,
    pub active_script: Option<Vec<ScriptCommand>>,
    pub current_cmd: usize,
    pub timer: u8,
}

impl ScriptEngine {
    pub fn new(script_paths: &[String]) -> Self {
        let mut scripts = HashMap::new();
        
        // Читаем все файлы скриптов для карты
        for path in script_paths {
            let full_path = format!("src/{}", path);
            if let Ok(content) = fs::read_to_string(&full_path) {
                let mut current_script_name = String::new();
                let mut commands = Vec::new();

                for line in content.lines() {
                    let clean = line.split(';').next().unwrap().trim();
                    if clean.is_empty() { continue; }

                    let first_word = clean.split_whitespace().next().unwrap_or("");
                    
                    if first_word.ends_with(':') {
                        if !current_script_name.is_empty() && !commands.is_empty() {
                            scripts.insert(current_script_name.clone(), commands.clone());
                        }
                        current_script_name = first_word.trim_end_matches(':').to_string();
                        commands.clear();
                    } else if clean.starts_with("moveLeft ") {
                        let dist = clean["moveLeft ".len()..].parse::<i32>().unwrap_or(1);
                        commands.push(ScriptCommand::Move { entity: "TARGET".to_string(), dx: -dist, dy: 0 });
                    } else if clean.starts_with("moveRight ") {
                        let dist = clean["moveRight ".len()..].parse::<i32>().unwrap_or(1);
                        commands.push(ScriptCommand::Move { entity: "TARGET".to_string(), dx: dist, dy: 0 });
                    } else if clean.starts_with("moveUp ") {
                        let dist = clean["moveUp ".len()..].parse::<i32>().unwrap_or(1);
                        commands.push(ScriptCommand::Move { entity: "TARGET".to_string(), dx: 0, dy: -dist });
                    } else if clean.starts_with("moveDown ") {
                        let dist = clean["moveDown ".len()..].parse::<i32>().unwrap_or(1);
                        commands.push(ScriptCommand::Move { entity: "TARGET".to_string(), dx: 0, dy: dist });
                    } else if clean == "csc_end" {
                        commands.push(ScriptCommand::End);
                    } else if clean == "faceUp" {
                        commands.push(ScriptCommand::SetFacing { entity: "TARGET".to_string(), dir: 0 });
                    } else if clean == "faceDown" {
                        commands.push(ScriptCommand::SetFacing { entity: "TARGET".to_string(), dir: 1 });
                    } else if clean == "faceLeft" {
                        commands.push(ScriptCommand::SetFacing { entity: "TARGET".to_string(), dir: 2 });
                    } else if clean == "faceRight" {
                        commands.push(ScriptCommand::SetFacing { entity: "TARGET".to_string(), dir: 3 });
                    } else if clean.starts_with("csWait ") {
                        commands.push(ScriptCommand::Wait);
                    }
                }
                
                // Сохраняем последний скрипт из файла
                if !current_script_name.is_empty() && !commands.is_empty() {
                    scripts.insert(current_script_name, commands);
                }
            }
        }

        Self {
            scripts,
            active_script: None,
            current_cmd: 0,
            timer: 0,
        }
    }

    pub fn start(&mut self, script_name: &str) {
        if let Some(cmds) = self.scripts.get(script_name) {
            self.active_script = Some(cmds.clone());
            self.current_cmd = 0;
            self.timer = 30; // Пауза перед началом скрипта
        }
    }

    pub fn is_running(&self) -> bool {
        self.active_script.is_some()
    }

    pub fn tick(&mut self, entities: &mut Vec<MapEntity>, target_entity_name: &str) -> bool {
        // 1. Если стоим на паузе - пропускаем кадр
        if self.active_script.is_some() {
            if self.timer > 0 {
                self.timer -= 1;
                return true; 
            }
        }

        if let Some(script) = &self.active_script {
            if self.current_cmd >= script.len() {
                self.active_script = None;
                return false;
            }

            match &script[self.current_cmd] {
                ScriptCommand::Move { entity, dx, dy } => {
                    if let Some(npc) = entities.iter_mut().find(|e| e.event_id == target_entity_name || e.event_id == *entity) {
                        // Ждем, пока она физически не дойдет до конца предыдущего шага
                        if npc.travel_x != 0 || npc.travel_y != 0 {
                            return true; 
                        }

                        let target_facing = if *dx < 0 { 2 }
                            else if *dx > 0 { 3 }
                            else if *dy < 0 { 0 }
                            else { 1 };

                        if npc.facing != target_facing {
                            npc.facing = target_facing;
                            npc.original_facing = target_facing;
                            self.timer = 15; // Ждем, чтобы игрок увидел поворот
                            return true; 
                        }

                        let nx = npc.x as i32 + dx;
                        let ny = npc.y as i32 + dy;
                        
                        if nx >= 0 && nx < 64 && ny >= 0 && ny < 64 {
                            npc.target_x = nx * 384;
                            npc.target_y = ny * 384;
                            npc.travel_x = npc.target_x - npc.pixel_x;
                            npc.travel_y = npc.target_y - npc.pixel_y;
                            npc.is_scripting = true; 
                        }
                        
                        self.current_cmd += 1;
                        self.timer = 15; // Пауза после того, как доедет
                    } else {
                        self.current_cmd += 1;
                    }
                }
                ScriptCommand::SetFacing { entity, dir } => {
                    if let Some(npc) = entities.iter_mut().find(|e| e.event_id == target_entity_name || e.event_id == *entity) {
                        npc.facing = *dir;
                        npc.original_facing = *dir;
                    }
                    self.current_cmd += 1;
                    self.timer = 15;
                }
                ScriptCommand::Wait => {
                    self.current_cmd += 1;
                    self.timer = 30;
                }
                ScriptCommand::End => {
                    self.active_script = None;
                    return false;
                }
            }
        }
        self.active_script.is_some()
    }
}