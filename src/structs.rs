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

//TODO use bitflags & enum & bitflags iterator for converting to string?
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
pub enum GameState {
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

impl From<u32> for GameState {
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
pub struct BeatmapPathValues {
    /// Absolute beatmap file path
    /// Example: `/path/to/osu/Songs/124321 Artist - Title/my_map.osu`
    pub beatmap_full_path: PathBuf,

    /// Relative to osu! folder beatmap folder path
    /// Example: `124321 Artist - Title`
    pub beatmap_folder: String,

    /// Relative to beatmap folder background file path
    /// Example: `my_map.osu`
    pub beatmap_file: String,

    /// Relative to beatmap folder background file path
    /// Example: `background.jpg`
    pub background_file: String,

    /// Absolute background file path
    /// Example: `/path/to/osu/Songs/beatmap/background.jpg`
    pub background_path_full: PathBuf,
}

#[derive(Debug, Default, Serialize)]
pub struct BeatmapValues {
    pub artist: String,
    pub title: String,
    pub creator: String,
    pub difficulty: String,

    /// ID of particular difficulty inside mapset
    pub map_id: i32,

    /// ID of whole mapset
    pub mapset_id: i32,

    pub ar: f32,
    pub cs: f32,
    pub hp: f32,
    pub od: f32,

    pub beatmap_status: BeatmapStatus,

    /// Time in milliseconds of last object of beatmap
    pub last_obj_time: f64,

    /// Time in milliseconds of first object of beatmap
    pub first_obj_time: f64,

    /// BPM of current selected beatmap
    pub bpm: f64,
    
    /// Paths of files used by beatmap
    /// .osu file, background file, etc
    pub paths: BeatmapPathValues,
}

#[derive(Debug, Default, Serialize)]
pub struct GameplayValues {
    #[serde(skip)]
    pub hit_errors: Vec<i32>,

    pub mods: u32,

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

    pub passed_objects: usize,

    #[serde(default = "SS")]
    pub grade: &'static str,
    pub current_hp: f64,
    pub current_hp_smooth: f64,
}

impl GameplayValues {
    #[inline]
    pub fn gamemode(&self) -> GameMode {
        let _span = tracy_client::span!("gamplay gamemode");
        GameMode::from(self.mode as u8)
    }

    pub fn passed_objects(&self) -> Result<usize, TryFromIntError> {
        let _span = tracy_client::span!("passed objects");

        let value = match self.gamemode() {
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


    pub fn get_current_grade(&self) -> &'static str {
        let _span = tracy_client::span!("calculate current grade");
        let total_hits = self.passed_objects as f64;
        let base_grade = match self.gamemode() {
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

    pub fn update_accuracy(&mut self) {
        let _span = tracy_client::span!("calculate accuracy");

        let acc: f64 = 'blk: {
            if self.passed_objects == 0 {
                break 'blk 1.;
            }

            match self.gamemode() {
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
        };

        self.accuracy = acc;
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
}

#[derive(Debug, Default, Serialize)]
pub struct OutputValues {
    /// Absolute path to the osu! folder
    /// Example: `/path/to/osu`
    /// Used internally
    #[serde(skip)]
    pub osu_path: PathBuf,

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
    pub prev_status: GameState,
    #[serde(skip)]
    pub prev_menu_mods: u32,
    #[serde(skip)]
    pub prev_menu_mode: i32,
    #[serde(skip)]
    pub delta_sum: usize,
    
    /// Name of the current skin
    pub skin: String,

    /// Playtime in milliseconds
    /// `Playing` => represents your progress into current beatmap
    /// `SongSelect` => represents progress of mp3 preview
    /// Note: can be negative
    pub playtime: i32,

    /// Current gamemode on `SongSelect` state
    pub menu_mode: i32,
    
    /// Current state of the game
    pub state: GameState,
    
    /// Stars of current beatmap without any mods
    pub stars: f64,

    /// Stars of current beatmap taking in account state and mods
    /// `Playing` => using gameplay mods
    /// `SongSelect` => using menu_mods
    pub stars_mods: f64,

    /// Stars calculated during gameplay and based on 
    /// current gameplay mods and passed objects
    /// calculated gradually
    pub current_stars: f64,
    
    /// Gameplay info
    pub gameplay: GameplayValues,

    /// Beatmap info
    pub beatmap: BeatmapValues,

    /// BPM calculated during gameplay
    /// based on your progress into the beatmap and gameplay mods
    pub current_bpm: f64,

    /// Is kiai is active now
    /// based on your progress into the beatmap
    pub kiai_now: bool,
    
    /// Current PP during gameplay
    /// based on your progress into the beatmap and gameplay mods
    pub current_pp: f64,

    /// Fullcombo PP during gameplay
    /// based on your progress into the beatmap and gameplay mods
    /// basically just removes misses
    pub fc_pp: f64,

    /// SS PP's
    /// based on your progress into the beatmap and mods
    /// `Playing` => using gameplay mods
    /// `SongSelect` => using menu_mods
    pub ss_pp: f64,

    /// Mods on `SongSelect` state
    pub menu_mods: u32,

    /// String representation of current selected mods
    /// `Playing` => using gameplay mods
    /// `SongSelect` => using menu_mods
    pub mods_str: Vec<&'static str>,

    pub plays: i32,
    
}

impl OutputValues {
    // Reseting values should happen from `OutputValues` functions
    // Separating it in individual functions gonna decrease readability
    // a lot
    pub fn reset_gameplay(&mut self) {
        let _span = tracy_client::span!("reset gameplay!");

        self.prev_combo = 0;
        self.prev_hit_miss = 0;
        self.prev_playtime = 0;

        self.mods_str.clear();

        self.current_pp = 0.0;
        self.fc_pp = 0.0;
        self.ss_pp = 0.0;

        self.current_bpm = 0.0;
        self.prev_passed_objects = 0;
        self.delta_sum = 0;
        self.kiai_now = false;
        self.playtime = 0;

        self.gameplay.slider_breaks = 0;
        self.gameplay.score = 0;
        self.gameplay.hit_300 = 0;
        self.gameplay.hit_100 = 0;
        self.gameplay.hit_50 = 0;
        self.gameplay.hit_geki = 0;
        self.gameplay.passed_objects = 0;
        self.gameplay.hit_katu = 0;
        self.gameplay.hit_miss = 0;
        self.gameplay.combo = 0;
        self.gameplay.max_combo = 0;
        self.gameplay.mode = 0;
        self.gameplay.slider_breaks = 0;
        self.gameplay.current_hp = 0.0;
        self.gameplay.current_hp_smooth = 0.0;

        self.gameplay.unstable_rate = 0.0;
    }
    
    #[inline]
    pub fn menu_gamemode(&self) -> GameMode {
        let _span = tracy_client::span!("menu gamemody");
        GameMode::from(self.menu_mode as u8)
    }

    pub fn update_current_bpm(&mut self) {
        let _span = tracy_client::span!("get current bpm");

        let bpm = if let Some(beatmap) = &self.current_beatmap {
            60000.0 / beatmap
                .timing_point_at(self.playtime as f64)
                .beat_len
        } else {
            self.current_bpm
        };

        self.current_bpm = bpm;
    }

    pub fn update_kiai(&mut self) {
        let _span = tracy_client::span!("get_kiai");

        self.kiai_now = if let Some(beatmap) = &self.current_beatmap {
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

    /// Depends on `GameplayValues`
    pub fn update_current_pp(&mut self, ivalues: &mut InnerValues) {
        let _span = tracy_client::span!("get_current_pp");
        if let Some(beatmap) = &self.current_beatmap {
            let score_state = ScoreState {
                max_combo: self.gameplay.max_combo as usize,
                n_geki: self.gameplay.hit_geki as usize,
                n_katu: self.gameplay.hit_katu as usize,
                n300: self.gameplay.hit_300 as usize,
                n100: self.gameplay.hit_100 as usize,
                n50: self.gameplay.hit_50 as usize,
                n_misses: self.gameplay.hit_miss as usize,
            };

            let passed_objects = self.gameplay.passed_objects;
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
                        self.gameplay.mods
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

    /// Depends on `GameplayValues`
    pub fn update_fc_pp(&mut self, ivalues: &mut InnerValues) {
        let _span = tracy_client::span!("get_fc_pp");
        if let Some(beatmap) = &self.current_beatmap {
            if ivalues.current_beatmap_perf.is_some() {
                if let Some(attributes) =
                    ivalues.current_beatmap_perf.clone() {
                    let fc_pp = AnyPP::new(beatmap)
                        .attributes(attributes.clone())
                        .mode(self.gameplay.gamemode())
                        .mods(self.gameplay.mods)
                        .n300(self.gameplay.hit_300 as usize)
                        .n100(self.gameplay.hit_100 as usize)
                        .n50(self.gameplay.hit_50 as usize)
                        .n_geki(self.gameplay.hit_geki as usize)
                        .n_katu(self.gameplay.hit_katu as usize)
                        .n_misses(self.gameplay.hit_miss as usize)
                        .calculate();
                    self.fc_pp = fc_pp.pp();
                }
                else {
                    self.fc_pp = 0.0
                }
            } else {
                let attr = AnyPP::new(beatmap)
                    .mods(self.gameplay.mods)
                    .mode(self.gameplay.gamemode())
                    .calculate();

                let ss_pp = attr.pp();
                self.ss_pp = ss_pp;

                ivalues.current_beatmap_perf = Some(attr);

                self.fc_pp = ss_pp;
            }
        } else {
            self.fc_pp = 0.0
        }

    }

    /// Adjust bpm based on current state
    /// `Playing` => using gameplay mods
    /// `SongSelect` => using menu_mods
    ///
    /// Depends on `GameplayValues`
    pub fn adjust_bpm(&mut self) {
        let _span = tracy_client::span!("adjust bpm");
        match self.state {
            GameState::Playing => {
                if self.gameplay.mods & 64 > 0 {
                    self.gameplay.unstable_rate /= 1.5;
                    self.current_bpm *= 1.5;
                    self.beatmap.bpm *= 1.5;
                }

                if self.gameplay.mods & 256 > 0 {
                    self.gameplay.unstable_rate *= 0.75;
                    self.current_bpm *= 0.75;
                    self.beatmap.bpm *= 0.75;
                }
            },
            GameState::SongSelect => {
                // Using menu mods when in SongSelect
                if self.menu_mods & 64 > 0 {
                    self.beatmap.bpm *= 1.5;
                }

                if self.menu_mods & 256 > 0 {
                    self.beatmap.bpm *= 0.75;
                }
            },
            _ => ()
        }
    }


    pub fn update_stars_and_ss_pp(&mut self) {
        let _span = tracy_client::span!("update stars and ss_pp");

        if let Some(beatmap) = &self.current_beatmap {
            let mods = {
                if self.state == GameState::Playing {
                    self.gameplay.mods
                } else {
                    self.menu_mods
                }
            };

            let mode = {
                if self.state == GameState::Playing {
                    self.gameplay.gamemode()
                } else {
                    self.menu_gamemode()
                }
            };

            self.stars = beatmap
                .stars()
                .mode(mode)     // Catch convertions is 
                .calculate()    // broken so converting
                .stars();       // manually, read #57 & #55

            let attr = beatmap
                .pp()
                .mode(mode)    // ^
                .mods(mods)
                .calculate();

            self.stars_mods = attr.stars();
            self.ss_pp = attr.pp();
        }
    }
    
    pub fn update_readable_mods(&mut self) {
        let _span = tracy_client::span!("get_readable_mods");

        let mods_values = match self.state {
            GameState::Playing => self.gameplay.mods,
            GameState::SongSelect => self.menu_mods,
            _ => self.menu_mods,
        };

        self.mods_str.clear();

        MODS.iter()
            .for_each(|(idx, name)| {
                if let Some(m) = (mods_values & idx > 0).then_some(*name) {
                    self.mods_str.push(m);
                }
            });

        if self.mods_str.contains(&"NC") {
            self.mods_str.retain(|x| x != &"DT");
        }

        if self.mods_str.contains(&"PF") {
            self.mods_str.retain(|x| x != &"SD");
        }
    }

    /// Depends on `BeatmapValues` and `BeatmapPathValues`
    pub fn update_full_paths(&mut self) {
        let _span = tracy_client::span!("update_full_paths");

        // beatmap_full_path is expection because
        // it depends on previous state

        self.beatmap.paths.background_path_full 
            = self.osu_path.join("Songs/");

        self.beatmap.paths.background_path_full
            .push(&self.beatmap.paths.beatmap_folder);

        self.beatmap.paths.background_path_full
            .push(&self.beatmap.paths.background_file);
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
            state: GameState::Playing,
            gameplay: GameplayValues {
                mods: 88,
                ..Default::default()
            },
            ..Default::default()
        };

        values.update_readable_mods();
        assert_eq!(
            vec!["HD", "HR", "DT"], 
            values.mods_str
        );

        values.gameplay.mods = 584;
        values.update_readable_mods();
        assert_eq!(
            vec!["HD", "NC"],
            values.mods_str
        );

        values.gameplay.mods = 1107561552;
        values.update_readable_mods();
        assert_eq!(
            vec!["HR","DT","FL","AU","K7","Coop","MR"], 
            values.mods_str
        );
    }
}
