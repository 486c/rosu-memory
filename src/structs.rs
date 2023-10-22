use rosu_pp::Beatmap;
use serde::Serialize;
use serde_repr::Serialize_repr;

#[derive(Serialize_repr, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum GameStatus {
    PreSongSelect = 0,
    Playing = 2,
    SongSelect = 5,
    EditorSongSelect = 4,
    ResultScreen = 7,
    MultiplayerLobbySelect = 11,
    MultiplayerLobby = 12,
    MultiplayerResultScreen = 14,

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
            14 => Self::MultiplayerResultScreen,
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
    pub playtime: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Values {

    #[serde(skip)]
    pub current_beatmap: Option<Beatmap>,

    #[serde(skip)]
    pub prev_combo: i16,
    #[serde(skip)]
    pub prev_hit_miss: i16,
    #[serde(skip)]
    pub prev_playtime: i32,

    pub artist: String,
    pub folder: String,
    pub beatmap_file: String,
    pub playtime: i32,

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
    pub slider_breaks: i16,

    // Calculated each iteration
    pub current_pp: f64,
    pub fc_pp: f64,

    pub passed_objects: usize,

    pub menu_mods: u32,
    pub mods: u32,

    pub plays: i32,
}

impl Values {
    pub fn reset_gameplay(&mut self) {
        self.slider_breaks = 0;
        self.hit_300 = 0;
        self.hit_100 = 0;
        self.hit_50 = 0;
        self.hit_geki = 0;
        self.hit_katu = 0;
        self.hit_miss = 0;
        self.combo = 0;
        self.max_combo = 0;
        self.mode = 0;
        self.slider_breaks = 0;

        self.prev_combo = 0;
        self.prev_hit_miss = 0;
        self.prev_playtime = 0;


        self.current_pp = 0.0;
        self.fc_pp = 0.0;
    }
}

// TODO reset gameplay function
