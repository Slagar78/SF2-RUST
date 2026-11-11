// src/game/state.rs
#[derive(PartialEq, Clone, Copy)]
pub enum GameState {
    Launcher,
    SegaLogo,
    TitleScreen,
    WitchScreen,
    Map,
    Menu,
    MemberMenu,
    MemberStatus,
    MagicMenu,
}