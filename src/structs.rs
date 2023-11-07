use std::{num::TryFromIntError, path::PathBuf};

use rosu_pp::{Beatmap, GameMode, GradualPerformanceAttributes, PerformanceAttributes};
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
    Unknown,
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
            _ => Self::Unknown,
        }
    }
}

#[derive(Serialize_repr, Debug, Default, PartialEq, Eq)]
#[repr(i16)]
pub enum BeatmapStatus {
    #[default]
    Unknown = 0,
    Unsubmitted = 1,
    Unranked = 2,
    Unused = 3,
    Ranked = 4,
    Approved = 5,
    Qualified = 6,
    Loved = 7,
}

impl From<i16> for BeatmapStatus {
    fn from(value: i16) -> Self {
        match value {
            1 => Self::Unsubmitted,
            2 => Self::Unranked,
            3 => Self::Unused,
            4 => Self::Ranked,
            5 => Self::Approved,
            6 => Self::Qualified,
            7 => Self::Loved,
            _ => Self::Unknown,
        }
    }
}

#[derive(Default)]
pub struct StaticAddresses {
    pub base: usize,
    pub status: usize,
    pub menu_mods: usize,
    pub rulesets: usize,
    pub playtime: usize,
    pub skin: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Values {
    #[serde(skip)]
    pub osu_path: PathBuf,

    #[serde(skip)]
    pub hit_errors: Vec<i32>,

    #[serde(skip)]
    pub current_beatmap: Option<Beatmap>,

    #[serde(skip)]
    pub prev_combo: i16,
    #[serde(skip)]
    pub prev_hit_miss: i16,
    #[serde(skip)]
    pub prev_playtime: i32,
    #[serde(skip)]
    pub prev_passed_objects: usize,
    #[serde(skip)]
    pub gradual_performance_current: Option<GradualPerformanceAttributes<'static>>,
    #[serde(skip)]
    pub delta_sum: usize,

    pub skin: String,

    pub skin: String,

    pub artist: String,
    pub folder: String,
    pub beatmap_file: String,
    pub playtime: i32,
    pub menu_mode: i32,

    pub status: GameStatus,

    pub ar: f32,
    pub cs: f32,
    pub hp: f32,
    pub od: f32,

    pub beatmap_status: BeatmapStatus,
    
    // Gameplay info
    pub username: String,
    pub score: i32,
    pub hit_300: i16,
    pub hit_100: i16,
    pub hit_50: i16,
    pub hit_geki: i16,
    pub hit_katu: i16,
    pub hit_miss: i16,
    pub accuracy: f64,
    pub combo: i16,
    pub max_combo: i16,
    pub mode: i32,
    pub slider_breaks: i16,
    pub unstable_rate: f64,
    pub grade: &'static str,
    pub current_hp: f64,
    pub current_hp_smooth: f64,

    // BPM of current selected beatmap
    pub bpm: f64,

    // BPM calculated during gameplay
    pub current_bpm: f64,
    pub kiai_now: bool,

    // Calculated each iteration
    pub current_pp: f64,
    pub fc_pp: f64,
    pub ss_pp: f64,
    #[serde(skip)]
    pub current_beatmap_perf: Option<PerformanceAttributes>,

    pub passed_objects: usize,

    pub menu_mods: u32,
    pub mods: u32,

    pub plays: i32,
}

impl Values {
    pub fn reset_gameplay(&mut self) {
        let _span = tracy_client::span!("reset gameplay!");

        self.slider_breaks = 0;
        self.username.clear();
        self.skin.clear();
        self.score = 0;
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
        self.current_hp = 0.0;
        self.current_hp_smooth = 0.0;

        self.prev_combo = 0;
        self.prev_hit_miss = 0;
        self.prev_playtime = 0;


        self.current_pp = 0.0;
        self.fc_pp = 0.0;
        self.current_beatmap_perf = None;

        self.passed_objects = 0;

        self.unstable_rate = 0.0;

        self.bpm = 0.0;
        self.current_bpm = 0.0;
        self.prev_passed_objects = 0;
        self.delta_sum = 0;
        self.gradual_performance_current = None;
        self.kiai_now = false;
    }

    // TODO PR to rosu-pp to add From<u8> trait?
    pub fn gameplay_gamemode(&self) -> GameMode {
        match self.mode {
            0 => GameMode::Osu,
            1 => GameMode::Taiko,
            2 => GameMode::Catch,
            3 => GameMode::Mania,
            _ => GameMode::Osu // Defaulting to osu
        }
    }
    
    // Waiting for new rosu-pp version
    pub fn menu_gamemode(&self) -> GameMode {
        match self.menu_mode {
            0 => GameMode::Osu,
            1 => GameMode::Taiko,
            2 => GameMode::Catch,
            3 => GameMode::Mania,
            _ => GameMode::Osu // Defaulting to osu
        }
    }

    pub fn passed_objects(&self) -> Result<usize, TryFromIntError> {
        let _span = tracy_client::span!("passed objects");

        let value = match self.gameplay_gamemode() {
            GameMode::Osu => 
                self.hit_300 + self.hit_100 
                + self.hit_50 + self.hit_miss,
            GameMode::Taiko => 
                self.hit_300 + self.hit_100 + self.hit_miss,
            GameMode::Catch => 
                self.hit_300 + self.hit_100 
                + self.hit_50 + self.hit_miss
                + self.hit_katu,
            GameMode::Mania => 
                self.hit_300 + self.hit_100 
                + self.hit_50 + self.hit_miss
                + self.hit_katu + self.hit_geki,
        };

        usize::try_from(value)
    }

    pub fn calculate_unstable_rate(&self) -> f64 {
        let _span = tracy_client::span!("calculate ur");

        if self.hit_errors.is_empty() {
            return 0.0
        };

        let hit_errors_len = self.hit_errors.len() as i32;

        let total: &i32 = &self.hit_errors.iter().sum();
        let average = total / hit_errors_len;

        let mut variance = 0;
        for hit in &self.hit_errors {
            variance += i32::pow(*hit - average, 2)
        }

        variance /= hit_errors_len;

        f64::sqrt(variance as f64) * 10.0
    }

    pub fn get_accuracy(&self) -> f64 {
        let _span = tracy_client::span!("calculate accuracy");
        if self.passed_objects == 0 {
          return 1.
        }
        match self.gameplay_gamemode() {
            GameMode::Osu => 
                (self.hit_300 as f64 * 6. + self.hit_100 as f64 * 2. + self.hit_50 as f64)
                / ((self.hit_300 + self.hit_100 + self.hit_50 + self.hit_miss) as f64 * 6.),
            GameMode::Taiko =>
                (self.hit_300 as f64 * 2. + self.hit_100 as f64)
                / ((self.hit_300 + self.hit_100 + self.hit_50 + self.hit_miss) as f64 * 2.),
            GameMode::Catch =>
                (self.hit_300 + self.hit_100 + self.hit_50) as f64
                / (self.hit_300 + self.hit_100 + self.hit_50 + self.hit_katu + self.hit_miss) as f64,
            GameMode::Mania =>
                ((self.hit_geki + self.hit_300) as f64 * 6. + self.hit_katu as f64 * 4. + self.hit_100 as f64 * 2. + self.hit_50 as f64)
                / ((self.hit_geki + self.hit_300 + self.hit_katu + self.hit_100 + self.hit_50 + self.hit_miss) as f64 * 6.)
        }
    }

    pub fn get_current_grade(&self) -> &'static str {
        let _span = tracy_client::span!("calculate current grade");
        let total_hits = self.passed_objects as f64;
        let base_grade = match self.gameplay_gamemode() {
            GameMode::Osu => {
                let ratio300 = self.hit_300 as f64 / total_hits;
                let ratio50 = self.hit_50 as f64 / total_hits;
                if self.accuracy == 1. {
                    "SS"
                } else if ratio300 > 0.9 && self.hit_miss == 0 && ratio50 <= 0.1 {
                    "S"
                } else if ratio300 > 0.8 && self.hit_miss == 0 || ratio300 > 0.9 {
                    "A"
                } else if ratio300 > 0.7 && self.hit_miss == 0 || ratio300 > 0.8 {
                    "B"
                } else if ratio300 > 0.6 {
                    "C"
                } else {
                    "D"
                }
            },
            GameMode::Taiko => {
                let ratio300 = self.hit_300 as f64 / total_hits;
                if self.accuracy == 1. {
                    "SS"
                } else if ratio300 > 0.9 && self.hit_miss == 0 {
                    "S"
                } else if ratio300 > 0.8 && self.hit_miss == 0 || ratio300 > 0.9 {
                    "A"
                } else if ratio300 > 0.7 && self.hit_miss == 0 || ratio300 > 0.8 {
                    "B"
                } else if ratio300 > 0.6 {
                    "C"
                } else {
                    "D"
                }
            },
            GameMode::Catch => {
                if self.accuracy == 1. {
                    "SS"
                } else if self.accuracy > 0.98 {
                    "S"
                } else if self.accuracy > 0.94 {
                    "A"
                } else if self.accuracy > 0.90 {
                    "B"
                } else if self.accuracy > 0.85 {
                    "C"
                } else {
                    "D"
                }
            },
            GameMode::Mania => {
                if self.accuracy == 1. {
                    "SS"
                } else if self.accuracy > 0.95 {
                    "S"
                } else if self.accuracy > 0.9 {
                    "A"
                } else if self.accuracy > 0.8 {
                    "B"
                } else if self.accuracy > 0.7 {
                    "C"
                } else {
                    "D"
                }
            }
        };
        // Hidden | Flashlight | Fade In
        match (base_grade, self.mods & (8 | 1024 | 1048576)) {
            ("SS", conj) if conj > 0 => "SSH",
            ("S", conj) if conj > 0 => "SH",
            _ => base_grade
        }
    }

    pub fn adjust_bpm(&mut self) {
        let _span = tracy_client::span!("adjust bpm");
        match self.status {
            GameStatus::Playing => {
                if self.mods & 64 > 0 {
                    self.unstable_rate /= 1.5;
                    self.current_bpm *= 1.5;
                    self.bpm *= 1.5;
                }

                if self.mods & 256 > 0 {
                    self.unstable_rate *= 0.75;
                    self.current_bpm *= 0.75;
                    self.bpm *= 0.75;
                }
            },
            GameStatus::SongSelect => {
                // Using menu mods when in SongSelect
                if self.menu_mods & 64 > 0 {
                    self.bpm *= 1.5;
                }

                if self.menu_mods & 256 > 0 {
                    self.bpm *= 0.75;
                }
            },
            _ => ()
        }
    }
}
