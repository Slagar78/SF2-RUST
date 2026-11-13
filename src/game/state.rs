// src/game/state.rs
#[derive(PartialEq, Clone, Copy)]
pub enum GameState {
    Launcher,
    SegaLogo,
    TitleScreen,
    WitchScreen,
    Map,
    Dialogue,
    Script,
    Menu,
    MemberMenu,
    MemberStatus,
    MagicMenu,
}