mod structs;
mod network;

use structs::{InnerValues, OutputValues, Clients};
use tracy_client::*;

use crate::network::{server_thread, handle_clients};

use crate::structs::{
    BeatmapStatus,
    GameStatus,
    StaticAddresses,
    Values,
};

use std::sync::{Arc, Mutex};
use std::{
    borrow::Cow,
    path::PathBuf
};

use clap::Parser;
use rosu_pp::Beatmap;

use rosu_memory::
    memory::{
        process::{Process, ProcessTraits}, 
        error::ProcessError
    };

use eyre::{Report, Result};

#[derive(Parser, Debug)]
pub struct Args {
    /// Path to osu! folder
    #[arg(short, long, env)]
    osu_path: Option<PathBuf>,

    /// Interval between updates in ms
    #[clap(default_value = "300")]
    #[arg(short, long, value_parser=parse_interval)]
    interval: std::time::Duration,
}

fn parse_interval(
    arg: &str
) -> Result<std::time::Duration, std::num::ParseIntError> {
    let ms = arg.parse()?;
    Ok(std::time::Duration::from_millis(ms))
}

fn process_reading_loop(
    p: &Process,
    state: &mut Values
) -> Result<()> {
    let _span = span!("reading loop");

    let values = state.values.clone();
    let mut values = values.lock().unwrap();

    let menu_mods_ptr = p.read_i32(
        state.ivalues.addresses.menu_mods + 0x9
    )?;

    values.menu_mods = p.read_u32(menu_mods_ptr as usize)?;

    let playtime_ptr = p.read_i32(state.addresses.playtime + 0x5)?;
    values.playtime = p.read_i32(playtime_ptr as usize)?;

    let beatmap_ptr = p.read_i32(state.addresses.base - 0xC)?;
    let beatmap_addr = p.read_i32(beatmap_ptr as usize)?;

    let status_ptr = p.read_i32(state.addresses.status - 0x4)?;

    let skin_ptr = p.read_i32(state.addresses.skin + 0x4)?;
    let skin_data = p.read_i32(skin_ptr as usize)?;
    values.skin = p.read_string(skin_data as usize + 0x44)?;

    values.status = GameStatus::from(
        p.read_u32(status_ptr as usize)?
    );

    if beatmap_addr == 0 {
      return Ok(())
    }

    if values.status != GameStatus::MultiplayerLobby {
        let ar_addr = beatmap_addr + 0x2c;
        let cs_addr = ar_addr + 0x04;
        let hp_addr = cs_addr + 0x04;
        let od_addr = hp_addr + 0x04;

        values.ar = p.read_f32(ar_addr as usize)?;
        values.cs = p.read_f32(cs_addr as usize)?;
        values.hp = p.read_f32(hp_addr as usize)?;
        values.od = p.read_f32(od_addr as usize)?;

        let plays_addr = p.read_i32(state.addresses.base - 0x33)? + 0xC;
        values.plays = p.read_i32(plays_addr as usize)?;

        values.artist = p.read_string((beatmap_addr + 0x18) as usize)?;
    }

    values.beatmap_status = BeatmapStatus::from(
        p.read_i16(beatmap_addr as usize + 0x130)?
    );

    let mut new_map = false;

    if values.status != GameStatus::PreSongSelect
    && values.status != GameStatus::MultiplayerLobby 
    && values.status != GameStatus::MultiplayerResultScreen {
        let beatmap_file = p.read_string((beatmap_addr + 0x94) as usize)?;
        let folder = p.read_string((beatmap_addr + 0x78) as usize)?;
        let menu_mode_addr = p.read_i32(state.addresses.base - 0x33)?;
        values.menu_mode = p.read_i32(menu_mode_addr as usize)?;


        if folder != values.folder 
        || beatmap_file != values.beatmap_file {
            let mut full_path = values.osu_path.clone();
            full_path.push("Songs");
            full_path.push(&folder);
            full_path.push(&beatmap_file);

            if full_path.exists() {
                values.current_beatmap = match Beatmap::from_path(
                    full_path
                ) {
                    Ok(beatmap) => {
                        new_map = true;
                        Some(beatmap)
                    },
                    Err(_) => {
                        println!("Failed to parse beatmap");
                        None
                    },
                }
            }
        }
        values.beatmap_file = beatmap_file;
        values.folder = folder;
    }

    if let Some(beatmap) = &values.current_beatmap {
        values.bpm = beatmap.bpm();
    }

    // store the converted map so it's not converted 
    // everytime it's used for pp calc
    if new_map {
        if let Some(map) = &values.current_beatmap {
            if let Cow::Owned(converted) = map
                .convert_mode(values.menu_gamemode()) 
            {
                values.current_beatmap = Some(converted);
            }
        }
    }

    let ruleset_addr = p.read_i32(
        (p.read_i32(state.addresses.rulesets - 0xb)? + 0x4) as usize
    )?;

    if values.status == GameStatus::Playing {
        let _span = span!("Gameplay data");
        if values.prev_playtime > values.playtime {
            values.reset_gameplay();
            state.ivalues.reset();
        }

        values.prev_playtime = values.playtime;

        let gameplay_base = 
            p.read_i32((ruleset_addr + 0x68) as usize)? as usize;
        let score_base = p.read_i32(gameplay_base + 0x38)? as usize;

        let hp_base: usize = p.read_i32(gameplay_base + 0x40)? as usize;

        // Random value but seems to work pretty well
        if values.playtime > 150 {
            values.current_hp = p.read_f64(hp_base + 0x1C)?;
            values.current_hp_smooth = p.read_f64(hp_base + 0x14)?;
        }

        let hit_errors_base = (
            p.read_i32(score_base + 0x38)?
        ) as usize;

        p.read_i32_array(
            hit_errors_base,
            &mut values.hit_errors
        )?;

        values.unstable_rate = values.calculate_unstable_rate();

        values.mode = p.read_i32(score_base + 0x64)?;

        values.hit_300 = p.read_i16(score_base + 0x8a)?;
        values.hit_100 = p.read_i16(score_base + 0x88)?;
        values.hit_50 = p.read_i16(score_base + 0x8c)?;

        values.username = p.read_string(score_base + 0x28)?;

        values.hit_geki = p.read_i16(score_base + 0x8e)?;
        values.hit_katu = p.read_i16(score_base + 0x90)?;
        values.hit_miss = p.read_i16(score_base + 0x92)?;

        let passed_objects = values.passed_objects()?;
        values.passed_objects = passed_objects;

        values.accuracy = values.get_accuracy();

        values.score = p.read_i32(score_base + 0x78)?;

        values.combo = p.read_i16(score_base + 0x94)?;
        values.max_combo = p.read_i16(score_base + 0x68)?;

        if values.prev_combo > values.combo {
            values.prev_combo = 0;
        }

        if values.combo < values.prev_combo
        && values.hit_miss == values.prev_hit_miss {
            values.slider_breaks += 1;
        }

        values.prev_hit_miss = values.hit_miss;

        let mods_xor_base = (
            p.read_i32(score_base + 0x1C)?
        ) as usize;

        let mods_raw = p.read_u64(mods_xor_base + 0x8)?;

        let mods_xor1 = mods_raw & 0xFFFFFFFF;
        let mods_xor2 = mods_raw >> 32;

        values.mods = (mods_xor1 ^ mods_xor2) as u32;

        // Calculate pp
        values.current_pp = values.get_current_pp(&mut state.ivalues);
        values.fc_pp = values.get_fc_pp(&mut state.ivalues);

        values.prev_passed_objects = passed_objects;
        
        values.grade = values.get_current_grade();
        values.current_bpm = values.get_current_bpm();
        values.kiai_now = values.get_kiai();

        // Placing at the very end cuz we should
        // keep up with current_bpm & unstable rate
        // updates
        values.adjust_bpm();
    }

    Ok(())
}

fn main() -> Result<()> {
    let _client = tracy_client::Client::start();

    let args = Args::parse();
    let output_values = Arc::new(Mutex::new(OutputValues::default()));
    let inner_values = InnerValues::default();

    let mut state = Values {
        addresses: StaticAddresses::default(),
        clients: Clients::default(),
        ivalues: inner_values,
        values: output_values,
    };
    
    // Spawning Hyper server
    let server_clients = state.clients.clone();
    std::thread::spawn(move || server_thread(server_clients));

    'init_loop: loop {
        let mut values = state.values.lock().unwrap();

        let p = match Process::initialize("osu!.exe") {
            Ok(p) => p,
            Err(e) => {
                println!("{:?}", Report::new(e));
                continue 'init_loop
            },
        };

        // OSU_PATH cli argument if provided should
        // overwrite auto detected path
        // else use auto detected path
        match args.osu_path {
            Some(ref v) => {
                println!("Using provided osu! folder path");
                values.osu_path = v.clone();
            },
            None => {
                println!("Using auto-detected osu! folder path");
                if let Some(ref dir) = p.executable_dir {
                    values.osu_path = dir.clone();
                } else {
                    return Err(Report::msg(
                        "Can't auto-detect osu! folder path \
                         nor any was provided through command \
                         line argument"
                    ));
                }
            },
        }
        
        // Checking if path exists
        if !values.osu_path.exists() {
            println!(
                "Provided osu path doesn't exists!\n Path: {}",
                &values.osu_path.to_str().unwrap()
            );

            continue 'init_loop
        };

        drop(values);

        println!("Reading static signatures...");
        match StaticAddresses::new(&p) {
            Ok(v) => state.addresses = v,
            Err(e) => {
                match e.downcast_ref::<ProcessError>() {
                    Some(&ProcessError::ProcessNotFound) => 
                        continue 'init_loop,
                    #[cfg(target_os = "windows")]
                    Some(&ProcessError::OsError{ .. }) => 
                        continue 'init_loop,
                    Some(_) | None => {
                        println!("{:?}", e);
                        continue 'init_loop
                    },
                }
            },
        };

        println!("Starting reading loop");
        'main_loop: loop {
            if let Err(e) = process_reading_loop(
                &p,
                &mut state
            ) {
                match e.downcast_ref::<ProcessError>() {
                    Some(&ProcessError::ProcessNotFound) => 
                        continue 'init_loop,
                    #[cfg(target_os = "windows")]
                    Some(&ProcessError::OsError{ .. }) => 
                        continue 'init_loop,
                    Some(_) | None => {
                        println!("{:?}", e);
                        continue 'main_loop
                    },
                }
            }

            smol::block_on(async {
                handle_clients(state.values.clone(), state.clients.clone()).await;
            });

            std::thread::sleep(args.interval);
        }
    };
}
