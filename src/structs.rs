use std::{
    num::TryFromIntError,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex}
};

use async_tungstenite::WebSocketStream;
use hyper::upgrade::Upgraded;
use rosu_memory::memory::{
    process::{Process, ProcessTraits},
    signature::Signature
};

use rosu_pp::{Beatmap, BeatmapExt, GameMode,
              PerformanceAttributes, GradualPerformance,
              beatmap::EffectPoint, ScoreState, AnyPP
};

use serde::Serialize;
use serde_repr::Serialize_repr;
use eyre::Result;

use crate::network::smol_hyper::SmolIo;
pub type Arm<T> = Arc<Mutex<T>>;
pub type Clients = Arm<Vec<WebSocketStream<SmolIo<Upgraded>>>>;
const MODS: [(u32, &str); 31] = [
    (1 << 0, "NF"),
    (1 << 1, "EZ"),
    (1 << 2, "TD"),
    (1 << 3, "HD"),
    (1 << 4, "HR"),
    (1 << 5, "SD"),
    (1 << 6, "DT"),
    (1 << 7, "RX"),
    (1 << 8, "HT"),
    (1 << 9, "NC"),
    (1 << 10, "FL"),
    (1 << 11, "AU"),
    (1 << 12, "SO"),
    (1 << 13, "AP"),
    (1 << 14, "PF"),
    (1 << 15, "K4"),
    (1 << 16, "K5"),
    (1 << 17, "K6"),
    (1 << 18, "K7"),
    (1 << 19, "K8"),
    (1 << 20, "FI"),
    (1 << 21, "RN"),
    (1 << 22, "CN"),
    (1 << 23, "TP"),
    (1 << 24, "K9"),
    (1 << 25, "Coop"),
    (1 << 26, "K1"),
    (1 << 27, "K3"),
    (1 << 28, "K2"),
    (1 << 29, "V2"),
    (1 << 30, "MR"),
];
#[derive(Serialize_repr, Debug, Default, PartialEq, Eq, Clone, Copy)]
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

impl StaticAddresses {
    pub fn new(p: &Process) -> Result<Self> {
        let _span = tracy_client::span!("static addresses");

        let base_sign = Signature::from_str("F8 01 74 04 83 65")?;
        let status_sign = Signature::from_str("48 83 F8 04 73 1E")?;
        let menu_mods_sign = Signature::from_str(
            "C8 FF ?? ?? ?? ?? ?? 81 0D ?? ?? ?? ?? 00 08 00 00"
        )?;

        let rulesets_sign = Signature::from_str(
            "7D 15 A1 ?? ?? ?? ?? 85 C0"
        )?;

        let playtime_sign = Signature::from_str(
            "5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04"
        )?;

        let skin_sign = Signature::from_str("75 21 8B 1D")?;

        Ok(Self {
            base: p.read_signature(&base_sign)?,
            status: p.read_signature(&status_sign)?,
            menu_mods: p.read_signature(&menu_mods_sign)?,
            rulesets: p.read_signature(&rulesets_sign)?,
            playtime: p.read_signature(&playtime_sign)?,
            skin: p.read_signature(&skin_sign)?,
        })
    }
}


pub struct State {
    pub addresses: StaticAddresses,
    pub clients: Clients,
    pub values: Arm<OutputValues>,
    pub ivalues: InnerValues,
}

// Inner values that used only inside
// reading loop and shouldn't be
// shared between any threads
#[derive(Default)]
pub struct InnerValues {
    pub gradual_performance_current:
        Option<GradualPerformance<'static>>,

    pub current_beatmap_perf: Option<PerformanceAttributes>,

    pub addresses: StaticAddresses
}

impl InnerValues {
    pub fn reset(&mut self) {
        self.current_beatmap_perf = None;
        self.gradual_performance_current = None;
    }
}

#[derive(Debug, Default, Serialize)]
pub struct OutputValues {
    /// Absolute path to the osu! folder
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
    pub prev_status: GameStatus,
    #[serde(skip)]
    pub prev_menu_mods: u32,

    pub skin: String,

    pub beatmap_full_path: PathBuf,

    pub artist: String,
    pub title: String,
    pub creator: String,
    pub difficulty: String,
    pub map_id: i32,
    pub mapset_id: i32,

    pub beatmap_folder: String,
    pub beatmap_id: i32,
    pub beatmap_file: String,
    pub background_file: String,
    pub background_path_full: PathBuf,
    pub playtime: i32,
    pub menu_mode: i32,

    pub status: GameStatus,

    pub stars: f64,
    pub stars_mods: f64,
    pub current_stars: f64,

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

    #[serde(default = "SS")]
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

    pub passed_objects: usize,
    #[serde(skip)]
    pub delta_sum: usize,

    pub menu_mods: u32,
    pub mods: u32,
    pub mods_str: Vec<&'static str>,

    pub plays: i32,

    pub last_obj_time: f64,
    pub first_obj_time: f64,
}

impl OutputValues {
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

        self.mods_str.clear();

        self.current_pp = 0.0;
        self.fc_pp = 0.0;
        self.ss_pp = 0.0;

        self.passed_objects = 0;

        self.unstable_rate = 0.0;

        self.bpm = 0.0;
        self.current_bpm = 0.0;
        self.prev_passed_objects = 0;
        self.delta_sum = 0;
        self.kiai_now = false;
        self.playtime = 0;
    }

    #[inline]
    pub fn gameplay_gamemode(&self) -> GameMode {
        GameMode::from(self.mode as u8)
    }
    
    #[inline]
    pub fn menu_gamemode(&self) -> GameMode {
        GameMode::from(self.menu_mode as u8)
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
    pub fn get_readable_mods(&mut self) -> Vec<&'static str> {
        let _span = tracy_client::span!("get_readable_mods");
        let mut mods: Vec<&'static str> = MODS.iter()
            .filter_map(|(idx, name)| 
                (self.mods & idx > 0).then_some(*name)
            )
            .collect();
        if mods.contains(&"NC") {
            mods.retain(|x| x != &"DT");
        }
        if mods.contains(&"PF") {
            mods.retain(|x| x != &"SD");
        }
        mods
    }

    pub fn get_accuracy(&self) -> f64 {
        let _span = tracy_client::span!("calculate accuracy");
        if self.passed_objects == 0 {
          return 1.
        }
        match self.gameplay_gamemode() {
            GameMode::Osu => 
                (self.hit_300 as f64 * 6. 
                 + self.hit_100 as f64 * 2. 
                 + self.hit_50 as f64)
                / 
                ((self.hit_300 
                 + self.hit_100 
                 + self.hit_50 
                 + self.hit_miss) as f64 * 6.
            ),
            GameMode::Taiko =>
                (self.hit_300 as f64 * 2. + self.hit_100 as f64)
                / 
                ((self.hit_300 
                 + self.hit_100 
                 + self.hit_50 
                 + self.hit_miss) as f64 * 2.),
            GameMode::Catch =>
                (self.hit_300 + self.hit_100 + self.hit_50) as f64
                / 
                (self.hit_300 + self.hit_100 + self.hit_50 
                 + self.hit_katu + self.hit_miss) as f64,
            GameMode::Mania =>
                ((self.hit_geki + self.hit_300) as f64 
                 * 6. + self.hit_katu as f64 
                 * 4. + self.hit_100 as f64 
                 * 2. + self.hit_50 as f64)
                / 
                ((self.hit_geki 
                 + self.hit_300 
                 + self.hit_katu 
                 + self.hit_100 
                 + self.hit_50 
                 + self.hit_miss) as f64 * 6.
            )
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
                } else if ratio300 > 0.9 
                    && self.hit_miss == 0 
                    && ratio50 <= 0.1 {
                    "S"
                } else if ratio300 > 0.8 
                    && self.hit_miss == 0 || ratio300 > 0.9 {
                    "A"
                } else if ratio300 > 0.7 
                    && self.hit_miss == 0 
                    || ratio300 > 0.8 {
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
                } else if ratio300 > 0.8 
                    && self.hit_miss == 0 
                    || ratio300 > 0.9 {
                    "A"
                } else if ratio300 > 0.7 
                    && self.hit_miss == 0 
                    || ratio300 > 0.8 {
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

    pub fn get_current_bpm(&self) -> f64 {
        let _span = tracy_client::span!("get current bpm");
        if let Some(beatmap) = &self.current_beatmap {
            60000.0 / beatmap
                .timing_point_at(self.playtime as f64)
                .beat_len
        } else {
            self.current_bpm
        }
    }

    pub fn get_kiai(&self) -> bool {
        let _span = tracy_client::span!("get_kiai");
        if let Some(beatmap) = &self.current_beatmap {
            // TODO: get rid of extra allocation?
            let kiai_data: Option<EffectPoint> = beatmap
                .effect_point_at(self.playtime as f64);
            if let Some(kiai) = kiai_data {
                kiai.kiai
            } else {
                self.kiai_now
            }
        } else {
            self.kiai_now
        }
    }

    pub fn update_current_pp(&mut self, ivalues: &mut InnerValues) {
        let _span = tracy_client::span!("get_current_pp");
        if let Some(beatmap) = &self.current_beatmap {
            let score_state = ScoreState {
                max_combo: self.max_combo as usize,
                n_geki: self.hit_geki as usize,
                n_katu: self.hit_katu as usize,
                n300: self.hit_300 as usize,
                n100: self.hit_100 as usize,
                n50: self.hit_50 as usize,
                n_misses: self.hit_miss as usize,
            };

            let passed_objects = self.passed_objects;
            let prev_passed_objects = self.prev_passed_objects;
            let delta = passed_objects - prev_passed_objects;

            let gradual = ivalues
                .gradual_performance_current
                .get_or_insert_with(|| {
                    // TODO: required until we rework the struct
                    let static_beatmap = unsafe {
                        extend_lifetime(beatmap)
                    };
                    GradualPerformance::new(
                        static_beatmap,
                        self.mods
                    )
                });

            // delta can't be 0 as processing 0 actually processes 1 object
            if (delta > 0) && (self.delta_sum <= prev_passed_objects) {
                self.delta_sum += delta;
                let attributes_option = gradual.nth(score_state, delta - 1);
                match attributes_option {
                    Some(attributes) => {
                        self.current_pp = attributes.pp();
                        self.current_stars = attributes.stars();
                    }
                    None => { println!("Failed to calculate current pp/sr") }
                }

            }
        }
    }

    pub fn get_fc_pp(&mut self, ivalues: &mut InnerValues) -> f64 {
        let _span = tracy_client::span!("get_fc_pp");
        if let Some(beatmap) = &self.current_beatmap {
            if ivalues.current_beatmap_perf.is_some() {
                if let Some(attributes) =
                    ivalues.current_beatmap_perf.clone() {
                    let fc_pp = AnyPP::new(beatmap)
                        .attributes(attributes.clone())
                        .mods(self.mods)
                        .n300(self.hit_300 as usize)
                        .n100(self.hit_100 as usize)
                        .n50(self.hit_50 as usize)
                        .n_geki(self.hit_geki as usize)
                        .n_katu(self.hit_katu as usize)
                        .n_misses(self.hit_miss as usize)
                        .calculate();
                    fc_pp.pp()
                }
                else {
                    0.0
                }
            } else {
                let attr = AnyPP::new(beatmap)
                    .mods(self.mods)
                    .mode(self.gameplay_gamemode())
                    .calculate();
                let ss_pp = attr.pp();
                self.ss_pp = ss_pp;
                ivalues.current_beatmap_perf = Some(attr);
                ss_pp
            }
        } else {
            0.0
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

    pub fn update_stars(&mut self) {
        let _span = tracy_client::span!("update stars");

        if let Some(beatmap) = &self.current_beatmap {
            self.stars = beatmap
                .stars()
                .calculate()
                .stars();

            let mods = {
                if self.status == GameStatus::Playing {
                    self.mods
                } else {
                    self.menu_mods
                }
            };

            self.stars_mods = beatmap
                .stars()
                .mods(mods)
                .calculate()
                .stars();
        }
    }

    pub fn update_full_paths(&mut self) {
        let _span = tracy_client::span!("update_full_paths");

        // beatmap_full_path is expection because
        // it depends on previous state

        self.background_path_full = self.osu_path.join("Songs/");
        self.background_path_full.push(&self.beatmap_folder);
        self.background_path_full.push(&self.background_file);
    }
}

unsafe fn extend_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_mod_conversion() {
        let mut values = OutputValues {
            mods: 88,
            ..Default::default()
        };
        assert_eq!(vec!["HD", "HR", "DT"], values.get_readable_mods());

        values.mods = 584;
        assert_eq!(vec!["HD", "NC"], values.get_readable_mods());

        values.mods = 1107561552;
        assert_eq!(
            vec!["HR","DT","FL","AU","K7","Coop","MR"], 
            values.get_readable_mods()
        );
    }
}
