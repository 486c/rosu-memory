use serde::Serialize;

use crate::structs::{GameState, OutputValues, BeatmapStatus};

#[derive(Debug, Serialize)]
pub struct GosuMenu {
    state: GameState,

    #[serde(rename = "SkinFolder")]
    skin_folder: String,

    #[serde(rename = "gameMode")]
    gamemode: i32,

    #[serde(rename = "isChatEnabled")]
    chat_enabled: bool,

    #[serde(rename = "bm")]
    beatmap: GosuBeatmap,

    mods: GosuMods,

    pp: GosuMenuPp,
}

#[derive(Debug, Serialize)]
pub struct GosuMenuPp {
    #[serde(rename = "100")]
    pp_ss: f64,
}

#[derive(Debug, Serialize)]
pub struct GosuBeatmapTime {
    first_obj: f64,
    current: f64,
    full: f64,
    mp3: f64
}

#[derive(Debug, Serialize)]
pub struct GosuBeatmapMetadata {
    artist: String,
    title: String,
    mapper: String,
    difficulty: String,
}

#[derive(Debug, Serialize)]
pub struct GosuBeatmapStatsBpm {
    min: i32,
    max: i32,
}

#[derive(Debug, Serialize)]
pub struct GosuBeatmapStats {
    #[serde(rename = "AR")]
    ar: f32,
    #[serde(rename = "CS")]
    cs: f32,
    #[serde(rename = "OD")]
    od: f32,
    #[serde(rename = "HP")]
    hp: f32,
    #[serde(rename = "SR")]
    sr: f64,

    #[serde(rename = "BPM")]
    bpm: GosuBeatmapStatsBpm,

    #[serde(rename = "fullSR")]
    full_sr: f64,
}

#[derive(Debug, Serialize)]
pub struct GosuMods {
    num: u32,
    str: String,
}

#[derive(Debug, Serialize)]
pub struct GosuBeatmapPath {
    full: String,
    folder: String,
    file: String,
    bg: String,
    audio: String,
}

#[derive(Debug, Serialize)]
pub struct GosuBeatmap {
    id: i32,
    set: i32,
    md5: String,

    time: GosuBeatmapTime,

    #[serde(rename = "rankedStatus")]
    status: BeatmapStatus,

    metadata: GosuBeatmapMetadata,

    stats: GosuBeatmapStats,

    path: GosuBeatmapPath,

}

#[derive(Debug, Serialize)]
pub struct GosuGameplayCombo {
    current: i16,
    max: i16,
}

#[derive(Debug, Serialize)]
pub struct GosuGameplayHp {
    normal: f64,
    smooth: f64,
}

#[derive(Debug, Serialize)]
pub struct GosuGameplayHitsGrade {
    current: String,
    #[serde(rename = "maxThisPlay")]
    max: String
}

#[derive(Debug, Serialize)]
pub struct GosuGameplayPp {
    current: f64,
    fc: f64,
    max: f64,
}

#[derive(Debug, Serialize)]
pub struct GosuGameplayHits {
    #[serde(rename = "300")]
    hit_300: i16,
    #[serde(rename = "200")]
    hit_200: i16,
    #[serde(rename = "100")]
    hit_100: i16,
    #[serde(rename = "50")]
    hit_50: i16,
    #[serde(rename = "geki")]
    hit_geki: i16,
    #[serde(rename = "katu")]
    hit_katu: i16,
    #[serde(rename = "0")]
    hit_miss: i16,

    grade: GosuGameplayHitsGrade,

    #[serde(rename = "sliderBreaks")]
    slider_breaks: i16,

    #[serde(rename = "unstableRate")]
    unstable_rate: f64,


    // TODO hitErrorArray
}

#[derive(Debug, Serialize)]
pub struct GosuGameplay {
    #[serde(rename = "gameMode")]
    gamemode: u8,

    name: String,
    score: i32,
    accuracy: f64,
    combo: GosuGameplayCombo,
    hp: GosuGameplayHp,
    hits: GosuGameplayHits,

    pp: GosuGameplayPp,
}

#[derive(Debug, Serialize)]
pub struct GosuValues {
    menu: GosuMenu,
    gameplay: GosuGameplay,
}

impl From<&OutputValues> for GosuValues {
    fn from(value: &OutputValues) -> Self {
        GosuValues {
            menu: GosuMenu {
                beatmap: GosuBeatmap {
                    id: value.beatmap.map_id,
                    set: value.beatmap.mapset_id,
                    md5: value.beatmap.md5.clone(),
                    status: value.beatmap.beatmap_status,
                    metadata: GosuBeatmapMetadata {
                        artist: value.beatmap.artist.clone(),
                        title: value.beatmap.artist.clone(),
                        mapper: value.beatmap.artist.clone(),
                        difficulty: value.beatmap.artist.clone(),
                    },
                    stats: GosuBeatmapStats {
                        ar: value.beatmap.ar,
                        cs: value.beatmap.cs,
                        od: value.beatmap.od,
                        hp: value.beatmap.hp,
                        sr: value.current_stars,
                        bpm: GosuBeatmapStatsBpm {
                            min: value.beatmap.min_bpm as i32,
                            max: value.beatmap.max_bpm as i32,
                        },
                        full_sr: value.stars_mods,
                    },
                    time: GosuBeatmapTime {
                        first_obj: value.beatmap.first_obj_time,
                        current: value.playtime as f64,
                        full: value.beatmap.last_obj_time,
                        mp3: value.beatmap.last_obj_time,
                    },
                    path: GosuBeatmapPath {
                        full: value.beatmap.paths.background_path_full
                            .clone()
                            .into_os_string().into_string()
                            .unwrap_or_default(),
                        folder: value.beatmap.paths.beatmap_folder.clone(),
                        file: value.beatmap.paths.beatmap_file.clone(),
                        bg: value.beatmap.paths.background_file.clone(),
                        audio: value.beatmap.paths.audio_file.clone(),
                    },
                },
                mods: GosuMods {
                    num: value.get_current_mods(),
                    str: value.mods_str.iter()
                        .fold(String::new(), |mut acc, x| {
                            acc.push_str(x);

                            acc
                        }),
                },
                state: value.state,
                skin_folder: value.skin_folder.clone(),
                gamemode: value.menu_mode,
                chat_enabled: value.chat_enabled,
                pp: GosuMenuPp {
                    pp_ss: value.ss_pp,
                },
            },
            gameplay: GosuGameplay {
                gamemode: value.gameplay.gamemode() as u8,
                name: value.gameplay.username.clone(),
                score: value.gameplay.score,
                accuracy: value.gameplay.accuracy,
                combo: GosuGameplayCombo {
                    current: value.gameplay.combo,
                    max: value.gameplay.max_combo,
                },
                hp: GosuGameplayHp {
                    normal: value.gameplay.current_hp,
                    smooth: value.gameplay.current_hp_smooth,
                },
                hits: GosuGameplayHits {
                    hit_300: value.gameplay.hit_300,
                    hit_200: value.gameplay.hit_katu,
                    hit_100: value.gameplay.hit_100,
                    hit_50: value.gameplay.hit_50,
                    hit_geki: value.gameplay.hit_geki,
                    hit_katu: value.gameplay.hit_katu,
                    hit_miss: value.gameplay.hit_miss,
                    grade: GosuGameplayHitsGrade {
                        current: value.gameplay.get_current_grade().to_string(),
                        max: value.gameplay.get_current_grade().to_string(),
                    },
                    slider_breaks: value.gameplay.slider_breaks,
                    unstable_rate: value.gameplay.unstable_rate,
                },
                pp: GosuGameplayPp {
                    current: value.current_pp,
                    fc: value.fc_pp,
                    max: value.fc_pp,
                },
            },
        }
    }
}

/*


{
    "menu": {
        "pp": {
            "100": 432,
            "99": 365,
            "98": 327,
            "97": 305,
            "96": 292,
            "95": 284,
            "strains": [ //Difficulty strain of the map, could be used to display strain graph
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                48.16962718963623,
                48.16962718963623,
                45.56485843658447,
                70.59610366821289,
                70.64111709594727,
                78.77282524108887, 
                174.9065968440129,
                171.57178192138673,
                154.53320966448103,
                133.339404296875,
                141.12918949127197,
                195.2433319091797,
                0
            ]
        }
    },
    "gameplay": {
        "gameMode": 0,
        "name": "Vaxei 2",
        "score": 120917,
        "accuracy": 86.73469345332401,
        "combo": {
            "current": 86,
            "max": 86
        },
        "hp": {
            "normal": 197.5853537607667,
            "smooth": 195.14011574928378
        },
        "hits": {
            "300": 45,
            "200": 6,
            "geki": 8,
            "100": 11,
            "katu": 6,
            "50": 0,
            "0": 0,
            "sliderBreaks": 0,
            "grade": {
                  "current": "B",
                  "maxThisPlay": "A"
            },
            "unstableRate": 131.88863093302524,
            "hitErrorArray": [ //Unstable rate array
                2,
                35,
                -2,
                4,
                24,
                38,
                1,
                -20,
                12,
                20,
                27,
                27,
                18,
                8,
                17,
                0,
                -2,
                16,
                28,
                40,
                23,
                -5,
                4,
                27,
                25,
                28,
                23,
                5,
                10,
                6,
                -6,
                10,
                4,
                7,
                -13,
                22,
                14,
                11,
                -7,
                6,
                -8,
                -8,
                -2,
                -2,
                -7,
                10,
                17,
                0,
                1,
                21,
                6,
                -2,
                7,
                18,
                7,
                14
            ]
        },
        "pp": {
            "current": 51,
            "fc": 385,
            "maxThisPlay": 385 //Possible pp this play, counts misses
        },
        "leaderboard": {
            "hasLeaderboard": true,
            "ourplayer": {
                "name": "Vaxei 2",
                "score": 120917,
                "combo": 86,
                "maxCombo": 86,
                "mods": "HR",
                "h300": 45,
                "h100": 11,
                "h50": 0,
                "h0": 0,
                "team": 0, //0 - solo, 1 OR 2 is BLUE/RED
                "position": 51,
                "isPassing": 1
            },
            "slots": [{ //gameplay leaderboard slots. Score order
                    "name": "Exarch",
                    "score": 54862276,
                    "combo": 0, //only visible in multiplayer or ourplayer in solo
                    "maxCombo": 1811,
                    "mods": "HDHR",
                    "h300": 1115,
                    "h100": 25,
                    "h50": 0,
                    "h0": 0,
                    "team": 0,
                    "position": 1,
                    "isPassing": 1
                },
                {
                    "name": "_Criller",
                    "score": 52751571,
                    "combo": 0,
                    "maxCombo": 1814,
                    "mods": "HD",
                    "h300": 1140,
                    "h100": 0,
                    "h50": 0,
                    "h0": 0,
                    "team": 0,
                    "position": 2,
                    "isPassing": 1
                },
                {
                    "name": "Vaxei 2",
                    "score": 120917,
                    "combo": 86,
                    "maxCombo": 86,
                    "mods": "HR",
                    "h300": 45,
                    "h100": 11,
                    "h50": 0,
                    "h0": 0,
                    "team": 0,
                    "position": 51,
                    "isPassing": 1
                }
            ]
        }
    }
}
*/
