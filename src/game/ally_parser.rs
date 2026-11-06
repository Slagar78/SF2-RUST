// src/game/ally_parser.rs
use std::fs;

pub struct Combatant {
    pub name: String,
    pub class: String,
    pub level: u8,
    pub items: Vec<String>,
    /// (spell_name, required_level) pairs parsed from the ally stats .asm files.
    pub spells: Vec<(String, u8)>,
}

/// Parse all starting force members from the SF2DISASM `.asm` sources.
///
/// Data is spread across three files:
///   - `allynames.asm`      : `allyName "BOWIE"` — display names, in member order.
///   - `allystartdefs.asm`  : per-member `startClass` / `startLevel` / `startItems`
///                            blocks, in the same order as the names.
///   - `stats/allystatsNN.asm` : per-member spell lists for each class.
///
/// Missing files are treated as empty (via `unwrap_or_default`), so the game can
/// still boot with a partial asset set. The member index ties the three sources
/// together positionally, so their order must stay in sync.
pub fn load_force_members() -> Vec<Combatant> {
    let path_names = "src/data/stats/allies/allynames.asm";
    let content_names = fs::read_to_string(path_names).unwrap_or_default();
    let mut names = Vec::new();

    // Extract display names from `allyName "..."` directives.
    // Everything after a `;` is treated as a comment and discarded.
    for line in content_names.lines() {
        let line = line.split(';').next().unwrap().trim();
        if let Some(start) = line.find("allyName \"") {
            let rest = &line[start + 10..];
            if let Some(end) = rest.find('"') {
                names.push(rest[..end].to_string());
            }
        }
    }

    let path_defs = "src/data/stats/allies/allystartdefs.asm"; 
    let content_defs = fs::read_to_string(path_defs).unwrap_or_default();
    
    let mut members = Vec::new();
    // Skip comments and blank lines up front so the state machine below only
    // sees directive lines.
    let mut lines = content_defs.lines()
        .map(|l| l.split(';').next().unwrap().trim())
        .filter(|l| !l.is_empty())
        .peekable();

    let mut index = 0;

    while let Some(line) = lines.next() {
        // Each member's block starts with `startClass <CLASS>`.
        if line.starts_with("startClass") {
            let class = line.split_whitespace().nth(1).unwrap_or("SDMN").to_string();
            
            let mut level = 1;
            let mut items = Vec::new();
            
            // The next directive is expected to be `startLevel <N>`.
            if let Some(lvl_line) = lines.next() {
                if lvl_line.starts_with("startLevel") {
                    level = lvl_line.split_whitespace().nth(1).unwrap_or("1").parse().unwrap_or(1);
                }
            }
            
            // `startItems` may span multiple lines; continuation lines are
            // introduced with `&`. We keep consuming until we have at least four
            // slots, matching SF2's fixed item slot count.
            if let Some(item_line) = lines.next() {
                if item_line.starts_with("startItems") {
                    let mut item_str = item_line.replace("startItems", "").replace("&", "").trim().to_string();
                    let mut items_temp: Vec<String> = item_str.split(',').map(|s| s.trim().to_string()).collect();
                    
                    while items_temp.len() < 4 {
                        if let Some(next_line) = lines.next() {
                            item_str = next_line.replace("&", "").trim().to_string();
                            items_temp.extend(item_str.split(',').map(|s| s.trim().to_string()));
                        } else { break; }
                    }
                    // Normalize raw item tokens: drop empties and placeholder
                    // slots (`NOTHING`), strip the `|EQUIPPED` flag suffix, and
                    // convert `_` to spaces for display.
                    items = items_temp.into_iter()
                        .filter(|s| !s.is_empty() && s != "NOTHING")
                        .map(|s| s.replace("|EQUIPPED", "").replace("_", " ")) 
                        .collect();
                }
            }
            
            // Spells are read from the per-member stats file. That file may
            // contain multiple `forClass` blocks; we only care about the one
            // matching this member's starting class, which is the first block
            // (the parser stops at the next `forClass` directive).
            let mut spells = Vec::new();
            let stats_path = format!("src/data/stats/allies/stats/allystats{:02}.asm", index);
            if let Ok(stats_content) = fs::read_to_string(stats_path) {
                let mut in_spell_list = false;
                for s_line in stats_content.lines() {
                    let s_line = s_line.split(';').next().unwrap().trim();
                    if s_line.is_empty() { continue; }

                    // Stop after the first class block; later blocks describe
                    // promoted/alternate classes and would pollute this list.
                    if s_line.starts_with("forClass") {
                        if in_spell_list { break; }
                    }

                    // `spellList` entries are (level, name) pairs, optionally
                    // spread over multiple lines via `&` continuations.
                    // The name may carry a `|FLAGS` suffix that we drop here.
                    if s_line.starts_with("spellList") || in_spell_list {
                        in_spell_list = true;
                        let clean = s_line.replace("spellList", "").replace("&", "").replace(",", " ");
                        let parts: Vec<&str> = clean.split_whitespace().collect();
                        
                        for chunk in parts.chunks(2) {
                            if chunk.len() == 2 {
                                let lvl = chunk[0].parse::<u8>().unwrap_or(1);
                                let spell_name = chunk[1].split('|').next().unwrap_or(chunk[1]).to_string();
                                // Only keep spells the starting level is high enough
                                // to have learned already.
                                if level >= lvl {
                                    spells.push((spell_name, lvl));
                                }
                            }
                        }
                    }
                }
            }

            let name = names.get(index).unwrap_or(&"UNKNOWN".to_string()).clone();
            
            members.push(Combatant { name, class, level, items, spells });
            index += 1;
        }
    }
    
    members
}