// src/game/state.rs
#[derive(PartialEq, Clone, Copy)]
pub enum GameState {
    SegaLogo,
    TitleScreen,
    WitchScreen,
    Map,
    Menu,
    MemberMenu,
    MemberStatus,
}