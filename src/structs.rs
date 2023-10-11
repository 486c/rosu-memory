use miniserde::Serialize;

#[repr(u32)]
#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub enum GameStatus {
    PreSongSelect = 0,
    Playing = 2,
    SongSelect = 5,
    EditorSongSelect = 4,
    ResultScreen = 7,
    MultiplayerLobbySelect = 11,
    MultiplayerLobby = 12,

    #[default]
    Unkown,
}

impl From<u32> for GameStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::PreSongSelect,
            2 => Self::Playing,
            4 => Self::EditorSongSelect,
            5 => Self::SongSelect,
            7 => Self::ResultScreen,
            11 => Self::MultiplayerLobbySelect,
            12 => Self::MultiplayerLobby,
            _ => Self::Unkown,
        }
    }
}

pub struct StaticAdresses {
    pub base: usize,
    pub status: usize,
    pub menu_mods: usize,
    pub rulesets: usize,
}
