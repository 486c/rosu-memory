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

#[derive(Default)]
pub struct StaticAdresses {
    pub base: usize,
    pub status: usize,
    pub menu_mods: usize,
    pub rulesets: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Values {
    pub artist: String,
    pub folder: String,
    pub beatmap_file: String,

    pub status: GameStatus,

    pub ar: f32,
    pub cs: f32,
    pub hp: f32,
    pub od: f32,
    
    // Gameplay info
    pub hit_300: i16,
    pub hit_100: i16,
    pub hit_50: i16,
    pub hit_geki: i16,
    pub hit_katu: i16,
    pub hit_miss: i16,
    pub combo: i16,
    pub max_combo: i16,
    pub mode: i32,

    // Calculated each iteration
    pub current_pp: f64,
    pub fc_pp: f64,

    pub passed_objects: usize,

    pub menu_mods: u32,
    pub mods: u32,

    pub plays: i32,
}
