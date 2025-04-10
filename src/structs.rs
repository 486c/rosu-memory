use std::{
    num::TryFromIntError,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use async_tungstenite::WebSocketStream;
use hyper::upgrade::Upgraded;
use rosu_mem::{
    process::{Process, ProcessTraits},
    signature::Signature,
};

use rosu_pp::{
    any::{PerformanceAttributes, ScoreState},
    model::mode::GameMode,
    Beatmap, Difficulty, GradualPerformance, Performance,
};

use eyre::Result;
use serde::Serialize;
use serde_repr::Serialize_repr;

use crate::{
    network::smol_hyper::SmolIo,
    utils::{effect_point_at, timing_point_at},
};

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum WsKind {
    Gosu,
    Rosu,
}

pub struct WsClient {
    pub kind: WsKind,
    pub client: WebSocketStream<SmolIo<Upgraded>>,
}

pub type Arm<T> = Arc<Mutex<T>>;
pub type Clients = Arm<Vec<WsClient>>;

macro_rules! calculate_accuracy {
    ($self: expr) => {{
        match $self.gamemode() {
            GameMode::Osu => {
                ($self.hit_300 as f64 * 6. + $self.hit_100 as f64 * 2. + $self.hit_50 as f64)
                    / (($self.hit_300 + $self.hit_100 + $self.hit_50 + $self.hit_miss) as f64 * 6.)
            }
            GameMode::Taiko => {
                ($self.hit_300 as f64 * 2. + $self.hit_100 as f64)
                    / (($self.hit_300 + $self.hit_100 + $self.hit_50 + $self.hit_miss) as f64 * 2.)
            }
            GameMode::Catch => {
                ($self.hit_300 + $self.hit_100 + $self.hit_50) as f64
                    / ($self.hit_300
                        + $self.hit_100
                        + $self.hit_50
                        + $self.hit_katu
                        + $self.hit_miss) as f64
            }
            GameMode::Mania => {
                (($self.hit_geki + $self.hit_300) as f64 * 6.
                    + $self.hit_katu as f64 * 4.
                    + $self.hit_100 as f64 * 2.
                    + $self.hit_50 as f64)
                    / (($self.hit_geki
                        + $self.hit_300
                        + $self.hit_katu
                        + $self.hit_100
                        + $self.hit_50
                        + $self.hit_miss) as f64
                        * 6.)
            }
        }
    }};
}

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

#[derive(Serialize_repr, Debug, Default, PartialEq, Eq, Copy, Clone)]
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
    pub base: i32,
    pub status: i32,
    pub menu_mods: i32,
    pub rulesets: i32,
    pub playtime: i32,
    pub skin: i32,
    pub chat_checker: i32,
    pub audio_time_base: i32,
}

impl StaticAddresses {
    pub fn new(p: &Process) -> Result<Self> {
        let _span = tracy_client::span!("static addresses");

        let base_sign = Signature::from_str("F8 01 74 04 83 65")?;
        let status_sign = Signature::from_str("48 83 F8 04 73 1E")?;
        let menu_mods_sign =
            Signature::from_str("C8 FF ?? ?? ?? ?? ?? 81 0D ?? ?? ?? ?? 00 08 00 00")?;

        let rulesets_sign = Signature::from_str("7D 15 A1 ?? ?? ?? ?? 85 C0")?;

        let playtime_sign = Signature::from_str("5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04")?;

        let skin_sign = Signature::from_str("75 21 8B 1D")?;

        let chat_checker = Signature::from_str("0A D7 23 3C 00 00 ?? 01")?;

        let audio_time_base = Signature::from_str("DB 5C 24 34 8B 44 24 34")?;

        Ok(Self {
            base: p.read_signature(&base_sign)?,
            status: p.read_signature(&status_sign)?,
            menu_mods: p.read_signature(&menu_mods_sign)?,
            rulesets: p.read_signature(&rulesets_sign)?,
            playtime: p.read_signature(&playtime_sign)?,
            skin: p.read_signature(&skin_sign)?,
            chat_checker: p.read_signature(&chat_checker)?,
            audio_time_base: p.read_signature(&audio_time_base)?,
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
    pub gradual_performance_current: Option<GradualPerformance>,

    /// Used for recalculations on fc_pp
    pub current_beatmap_perf: Option<PerformanceAttributes>,
}

impl InnerValues {
    pub fn reset(&mut self) {
        self.current_beatmap_perf = None;
        self.gradual_performance_current = None;
    }
}

#[derive(Debug, Default, Serialize)]
pub struct KeyOverlayValues {
    pub k1_pressed: bool,
    pub k1_count: u32,
    pub k2_pressed: bool,
    pub k2_count: u32,
    pub m1_pressed: bool,
    pub m1_count: u32,
    pub m2_pressed: bool,
    pub m2_count: u32,
}

impl KeyOverlayValues {
    pub fn reset(&mut self) {
        self.k1_pressed = false;
        self.k1_count = 0;
        self.k2_pressed = false;
        self.k2_count = 0;
        self.m1_pressed = false;
        self.m1_count = 0;
        self.m2_pressed = false;
        self.m2_count = 0;
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ResultScreenValues {
    pub username: String,
    pub mods: u32,
    pub mode: u8,
    pub max_combo: i16,
    pub score: i32,
    pub hit_300: i16,
    pub hit_100: i16,
    pub hit_50: i16,
    pub hit_geki: i16,
    pub hit_katu: i16,
    pub hit_miss: i16,
    pub accuracy: f64,
}

impl ResultScreenValues {
    pub fn gamemode(&self) -> GameMode {
        GameMode::from(self.mode)
    }

    pub fn update_accuracy(&mut self) {
        let _span = tracy_client::span!("result_screen: calculate accuracy");

        self.accuracy = calculate_accuracy!(self);
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

    /// Relative to beatmap folder audio file path
    /// Example: ``
    pub audio_file: String,
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

    /// MD5 hash of the beatmap
    pub md5: String,

    pub ar: f32,
    pub cs: f32,
    pub hp: f32,
    pub od: f32,

    /// Beatmap Status aka Ranked, Pending, Loved, etc
    pub beatmap_status: BeatmapStatus,

    /// Time in milliseconds of last object of beatmap
    pub last_obj_time: f64,

    /// Time in milliseconds of first object of beatmap
    pub first_obj_time: f64,

    /// BPM of currently selected beatmap
    pub bpm: f64,

    /// Max BPM of currently selected beatmap
    pub max_bpm: f64,

    /// Min BPM of currently selected beatmap
    pub min_bpm: f64,

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
            GameMode::Osu => self.hit_300 + self.hit_100 + self.hit_50 + self.hit_miss,
            GameMode::Taiko => self.hit_300 + self.hit_100 + self.hit_miss,
            GameMode::Catch => {
                self.hit_300 + self.hit_100 + self.hit_50 + self.hit_miss + self.hit_katu
            }
            GameMode::Mania => {
                self.hit_300
                    + self.hit_100
                    + self.hit_50
                    + self.hit_miss
                    + self.hit_katu
                    + self.hit_geki
            }
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
            }
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
            }
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
            }
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
            _ => base_grade,
        }
    }

    pub fn update_accuracy(&mut self) {
        let _span = tracy_client::span!("calculate accuracy");

        let acc: f64 = 'blk: {
            if self.passed_objects == 0 {
                break 'blk 1.;
            }

            calculate_accuracy!(self)
        };

        self.accuracy = acc;
    }

    pub fn calculate_unstable_rate(&self) -> f64 {
        let _span = tracy_client::span!("calculate ur");

        if self.hit_errors.is_empty() {
            return 0.0;
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
    pub prev_state: GameState,
    #[serde(skip)]
    pub prev_menu_mods: u32,
    #[serde(skip)]
    pub prev_menu_mode: i32,
    #[serde(skip)]
    pub delta_sum: usize,

    /// Name of the current skin
    pub skin: String,

    /// Skin folder relative to the osu! folder
    pub skin_folder: String,

    /// Is chat enabled (F9/F8)
    pub chat_enabled: bool,

    /// Playtime in milliseconds
    /// `Playing` => represents your progress into current beatmap
    /// `SongSelect` => represents progress of mp3
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
    /// `ResultScreen` => using result_screen mods
    pub stars_mods: f64,

    /// Stars calculated during gameplay and based on
    /// current gameplay mods and passed objects
    /// calculated gradually
    pub current_stars: f64,

    /// Result Screen info
    pub result_screen: ResultScreenValues,

    /// Gameplay info
    pub gameplay: GameplayValues,

    /// Beatmap info
    pub beatmap: BeatmapValues,

    // KeyOverlay infi
    pub keyoverlay: KeyOverlayValues,

    /// BPM calculated during gameplay
    /// based on your progress into the beatmap and gameplay mods
    pub current_bpm: f64,

    /// Is kiai is active now
    /// based on your progress into the beatmap
    pub kiai_now: bool,

    /// Current PP based on your state
    ///
    /// `Playing` => based on your progress into the beatmap
    ///              and gameplay mods
    /// `SongSelect` => ss_pp for current map using menu_mods
    /// `ResultScreen` => pp calculated for score on the screen
    ///                   (values are taken from result_screen)
    pub current_pp: f64,

    /// Fullcombo PP during gameplay
    /// based on your progress into the beatmap and gameplay mods
    /// basically just removes misses
    pub fc_pp: f64,

    /// SS PP's
    /// based on your progress into the beatmap and mods
    /// `Playing` => using gameplay mods
    /// `SongSelect` => using menu_mods
    /// `ResultScreen` => using result_screen mods
    pub ss_pp: f64,

    /// Mods on `SongSelect` state
    pub menu_mods: u32,

    /// String representation of current selected mods
    /// `Playing` => using gameplay mods
    /// `SongSelect` => using menu_mods
    /// `ResultScreen` => using result_screen mods
    pub mods_str: Vec<&'static str>,

    pub plays: i32,

    /// Position of current playing audio in milliseconds
    /// (to be honest it have nothing to do with precision)
    pub precise_audio_time: i32,
}

impl OutputValues {
    // Reseting values should happen from `OutputValues` functions
    // Separating it in individual functions gonna decrease readability
    // a lot.
    // Also reset a inner values for gradual pp calculator
    pub fn reset_gameplay(&mut self, ivalues: &mut InnerValues) {
        let _span = tracy_client::span!("reset gameplay!");

        self.keyoverlay.reset();

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

        ivalues.reset();
    }

    #[inline]
    pub fn menu_gamemode(&self) -> GameMode {
        let _span = tracy_client::span!("menu_gamemode");
        GameMode::from(self.menu_mode as u8)
    }

    pub fn update_min_max_bpm(&mut self) {
        let _span = tracy_client::span!("update_min_max_bpm");

        if let Some(beatmap) = &self.current_beatmap {
            // Maybe this is not very idiomatic approach
            // but atleast we dont need to iterate twice
            // to calculate min and max values
            let mut max_bpm = f64::MIN;
            let mut min_bpm = f64::MAX;

            for timing_point in beatmap.timing_points.iter() {
                let bpm = 60000.0 / timing_point.beat_len;

                if bpm > max_bpm {
                    max_bpm = bpm
                };
                if bpm < min_bpm {
                    min_bpm = bpm
                };
            }

            self.beatmap.max_bpm = max_bpm;
            self.beatmap.min_bpm = min_bpm;
        }
    }

    pub fn update_current_bpm(&mut self) {
        let _span = tracy_client::span!("get current bpm");

        let bpm = if let Some(beatmap) = &self.current_beatmap {
            match timing_point_at(beatmap, self.playtime as f64) {
                Some(v) => 60000.0 / v.beat_len,
                None => return,
            }
        } else {
            self.current_bpm
        };

        self.current_bpm = bpm;
    }

    pub fn update_kiai(&mut self) {
        let _span = tracy_client::span!("get_kiai");

        self.kiai_now = if let Some(beatmap) = &self.current_beatmap {
            // TODO: get rid of extra allocation?
            let kiai_data = effect_point_at(beatmap, self.playtime as f64);

            if let Some(kiai) = kiai_data {
                kiai.kiai
            } else {
                self.kiai_now
            }
        } else {
            self.kiai_now
        }
    }

    /// Depends on `GameplayValues` and `ResultScreenValues`
    pub fn update_current_pp(&mut self, ivalues: &mut InnerValues) {
        // TODO refactor this function in near future
        // maybe even split pp into struct aka `GameplayValues` -> pp
        // etc
        let _span = tracy_client::span!("get_current_pp");

        if self.state == GameState::ResultScreen {
            if let Some(beatmap) = &self.current_beatmap {
                //.mode(self.result_screen.gamemode()) TODO

                let diff = Difficulty::new()
                    .lazer(false)
                    .mods(self.result_screen.mods)
                    .calculate(beatmap);

                let perf = Performance::new(diff)
                    .n300(self.result_screen.hit_300 as u32)
                    .n100(self.result_screen.hit_100 as u32)
                    .n50(self.result_screen.hit_50 as u32)
                    .n_geki(self.result_screen.hit_geki as u32)
                    .n_katu(self.result_screen.hit_katu as u32)
                    .misses(self.result_screen.hit_miss as u32)
                    .calculate();

                self.current_pp = perf.pp();
            }

            return;
        }

        // TODO yep it definitely should be refactored
        if self.state == GameState::SongSelect {
            self.current_pp = self.ss_pp;
        }

        if let Some(beatmap) = &self.current_beatmap {
            let mut score_state = ScoreState::new();

            score_state.max_combo = self.gameplay.max_combo as u32;
            score_state.n_geki = self.gameplay.hit_geki as u32;
            score_state.n_katu = self.gameplay.hit_katu as u32;
            score_state.n300 = self.gameplay.hit_300 as u32;
            score_state.n100 = self.gameplay.hit_100 as u32;
            score_state.n50 = self.gameplay.hit_50 as u32;
            score_state.misses = self.gameplay.hit_miss as u32;

            // Protecting from non-initialized values
            if self.gameplay.passed_objects == 0 && self.prev_passed_objects == 0 {
                return;
            }

            let gradual = ivalues
                .gradual_performance_current
                .get_or_insert_with(|| {
                    let diff = Difficulty::new()
                        .lazer(false) // Reminder
                        .mods(self.gameplay.mods);

                    let mut grad = GradualPerformance::new(diff, beatmap);

                    // In cases if we start mid-map, advance to the
                    // current position.
                    if self.prev_passed_objects == 0 && self.gameplay.passed_objects != 0 {
                        let res = grad.nth(score_state.clone(), self.gameplay.passed_objects);

                        if res.is_none() {
                            println!("
                                Failed to advance gradual pp forward: passed_objects: {}, grad_remaining_objects: {}",
                                self.gameplay.passed_objects,
                                grad.len()
                            )
                        };

                        self.prev_passed_objects = self.gameplay.passed_objects;
                        self.delta_sum += self.gameplay.passed_objects;
                    };

                    grad
                });

            let passed_objects = self.gameplay.passed_objects;
            let prev_passed_objects = self.prev_passed_objects;
            let delta = passed_objects - prev_passed_objects;

            // delta can't be 0 as processing 0 actually processes 1 object
            if (delta > 0) && (self.delta_sum <= prev_passed_objects) {
                self.delta_sum += delta;
                let attributes_option = gradual.nth(score_state, delta - 1);
                match attributes_option {
                    Some(attributes) => {
                        self.current_pp = attributes.pp();
                        self.current_stars = attributes.stars();
                    }
                    None => {
                        println!(
                            "Failed to calculate current pp/sr, delta_sum: {}, delta_curr: {}",
                            self.delta_sum,
                            delta - 1
                        )
                    }
                }
            }
        }
    }

    /// Depends on `GameplayValues`
    pub fn update_fc_pp(&mut self, ivalues: &mut InnerValues) {
        let _span = tracy_client::span!("update_fc_pp");
        if let Some(beatmap) = &self.current_beatmap {
            if ivalues.current_beatmap_perf.is_some() {
                if let Some(perf_attrs) = ivalues.current_beatmap_perf.clone() {
                    let fc_pp = perf_attrs
                        .performance()
                        .mods(self.gameplay.mods)
                        .n300(self.gameplay.hit_300 as u32)
                        .n100(self.gameplay.hit_100 as u32)
                        .n50(self.gameplay.hit_50 as u32)
                        .n_geki(self.gameplay.hit_geki as u32)
                        .n_katu(self.gameplay.hit_katu as u32)
                        .misses(0)
                        .calculate()
                        .pp();

                    self.fc_pp = fc_pp;
                } else {
                    self.fc_pp = 0.0
                }
            } else {
                let diff = Difficulty::new()
                    .lazer(false)
                    .mods(self.gameplay.mods)
                    .calculate(beatmap);

                let perf_attrs = Performance::new(diff).calculate();

                let ss_pp = perf_attrs.pp();
                self.ss_pp = ss_pp;

                ivalues.current_beatmap_perf = Some(perf_attrs);

                self.fc_pp = ss_pp;
            }
        } else {
            self.fc_pp = 0.0
        }
    }

    /// Adjust bpm based on current state and mods
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
                    self.beatmap.max_bpm *= 1.5;
                    self.beatmap.min_bpm *= 1.5;
                } else if self.gameplay.mods & 256 > 0 {
                    self.gameplay.unstable_rate *= 0.75;
                    self.current_bpm *= 0.75;
                    self.beatmap.bpm *= 0.75;
                    self.beatmap.max_bpm *= 0.75;
                    self.beatmap.min_bpm *= 0.75;
                } else {
                    self.update_min_max_bpm();

                    if let Some(beatmap) = &self.current_beatmap {
                        self.beatmap.bpm = beatmap.bpm();
                    }
                }
            }
            GameState::SongSelect => {
                if self.menu_mods & 64 > 0 {
                    self.beatmap.bpm *= 1.5;
                    self.beatmap.max_bpm *= 1.5;
                    self.beatmap.min_bpm *= 1.5;
                } else if self.menu_mods & 256 > 0 {
                    self.beatmap.bpm *= 0.75;
                    self.beatmap.max_bpm *= 0.75;
                    self.beatmap.min_bpm *= 0.75;
                } else {
                    self.update_min_max_bpm();

                    if let Some(beatmap) = &self.current_beatmap {
                        self.beatmap.bpm = beatmap.bpm();
                    }
                }
            }
            _ => (),
        }
    }

    /// Returns mods depending on current game state
    pub fn get_current_mods(&self) -> u32 {
        match self.state {
            GameState::Playing => self.gameplay.mods,
            GameState::SongSelect => self.menu_mods,
            GameState::ResultScreen => self.result_screen.mods,
            _ => self.menu_mods,
        }
    }

    pub fn update_stars_and_ss_pp(&mut self) {
        let _span = tracy_client::span!("update stars and ss_pp");

        if let Some(beatmap) = &self.current_beatmap {
            let mods = {
                match self.state {
                    GameState::Playing => self.gameplay.mods,
                    GameState::SongSelect => self.menu_mods,
                    GameState::ResultScreen => self.result_screen.mods,
                    _ => self.menu_mods,
                }
            };

            let mode = {
                match self.state {
                    GameState::Playing => self.gameplay.gamemode(),
                    GameState::SongSelect => self.menu_gamemode(),
                    GameState::ResultScreen => self.result_screen.gamemode(),
                    _ => self.menu_gamemode(),
                }
            };

            // Just to be sure
            assert_eq!(beatmap.mode, mode);

            self.stars = Difficulty::new()
                .lazer(false)
                .mods(mods)
                .calculate(beatmap)
                .stars();

            let attr = Performance::new(beatmap).mods(mods).calculate();

            self.stars_mods = attr.stars();
            self.ss_pp = attr.pp();
        }
    }

    pub fn update_readable_mods(&mut self) {
        let _span = tracy_client::span!("get_readable_mods");

        let mods_values = match self.state {
            GameState::Playing => self.gameplay.mods,
            GameState::SongSelect => self.menu_mods,
            GameState::ResultScreen => self.result_screen.mods,
            _ => self.menu_mods,
        };

        self.mods_str.clear();

        MODS.iter().for_each(|(idx, name)| {
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

        self.beatmap.paths.background_path_full = self.osu_path.join("Songs/");

        self.beatmap
            .paths
            .background_path_full
            .push(&self.beatmap.paths.beatmap_folder);

        self.beatmap
            .paths
            .background_path_full
            .push(&self.beatmap.paths.background_file);
    }
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
        assert_eq!(vec!["HD", "HR", "DT"], values.mods_str);

        values.gameplay.mods = 584;
        values.update_readable_mods();
        assert_eq!(vec!["HD", "NC"], values.mods_str);

        values.gameplay.mods = 1107561552;
        values.update_readable_mods();
        assert_eq!(
            vec!["HR", "DT", "FL", "AU", "K7", "Coop", "MR"],
            values.mods_str
        );
    }
}
