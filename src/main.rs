mod structs;

use crate::structs::{
    GameStatus, 
    StaticAdresses,
    Values,
};

use std::{
    borrow::Cow,
    str::FromStr, 
    collections::HashMap, net::TcpStream, path::PathBuf
};

use clap::Parser;

use async_tungstenite::WebSocketStream;
use crossbeam_channel::bounded;

use async_tungstenite::tungstenite;
use futures_util::sink::SinkExt;
use rosu_pp::{Beatmap, AnyPP, ScoreState};
use smol::{prelude::*, Async};
use tungstenite::Message;

use rosu_memory::{
    memory::{
        process::{Process, ProcessTraits}, 
        signature::Signature, error::ProcessError
    }, 
    websockets::server_thread
};


use eyre::{Report, Result};

#[derive(Parser, Debug)]
pub struct Args {
    /// Path to osu! folder
    #[arg(short, long, env)]
    osu_path: PathBuf,

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

fn read_static_adresses(
    p: &Process,
    adresses: &mut StaticAdresses
) -> Result<()> {
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


    adresses.base = p.read_signature(&base_sign)?;
    adresses.status = p.read_signature(&status_sign)?;
    adresses.menu_mods = p.read_signature(&menu_mods_sign)?;
    adresses.rulesets = p.read_signature(&rulesets_sign)?;
    adresses.playtime = p.read_signature(&playtime_sign)?;

    Ok(())
}

fn process_reading_loop(
    p: &Process,
    args: &Args,
    adresses: &StaticAdresses,
    values: &mut Values
) -> Result<()> {
    let menu_mods_ptr = p.read_i32(adresses.menu_mods + 0x9)?;
    values.menu_mods = p.read_u32(menu_mods_ptr as usize)?;

    let playtime_ptr = p.read_i32(adresses.playtime + 0x5)?;
    values.playtime = p.read_i32(playtime_ptr as usize)?;

    let beatmap_ptr = p.read_i32(adresses.base - 0xC)?;
    let beatmap_addr = p.read_i32(beatmap_ptr as usize)?;

    let status_ptr = p.read_i32(adresses.status - 0x4)?;

    values.status = GameStatus::from(
        p.read_u32(status_ptr as usize)?
    );

    if values.status != GameStatus::MultiplayerLobby {
        let ar_addr = beatmap_addr + 0x2c;
        let cs_addr = ar_addr + 0x04;
        let hp_addr = cs_addr + 0x04;
        let od_addr = hp_addr + 0x04;

        values.ar = p.read_f32(ar_addr as usize)?;
        values.cs = p.read_f32(cs_addr as usize)?;
        values.hp = p.read_f32(hp_addr as usize)?;
        values.od = p.read_f32(od_addr as usize)?;

        let plays_addr = p.read_i32(adresses.base - 0x33)? + 0xC;
        values.plays = p.read_i32(plays_addr as usize)?;

        let artist_addr = p.read_i32((beatmap_addr + 0x18) as usize)?;
        values.artist = p.read_string(artist_addr as usize)?;
    }

    let mut new_map = false;

    if values.status != GameStatus::PreSongSelect
    && values.status != GameStatus::MultiplayerLobby 
    && values.status != GameStatus::MultiplayerResultScreen {
        let path_addr = p.read_i32((beatmap_addr + 0x94) as usize)?;
        let folder_addr = p.read_i32((beatmap_addr + 0x78) as usize)?;

        let beatmap_file = p.read_string(path_addr as usize)?;
        let folder = p.read_string(folder_addr as usize)?;

        if folder != values.folder 
        || beatmap_file != values.beatmap_file {
            let mut full_path = args.osu_path.clone();
            full_path.push("Songs");
            full_path.push(&folder);
            full_path.push(&beatmap_file);

            if full_path.exists() {
                values.current_beatmap = match Beatmap::from_path(full_path) {
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

    let ruleset_addr = p.read_i32(
        (p.read_i32(adresses.rulesets - 0xb)? + 0x4) as usize
    )?;

    if values.status == GameStatus::Playing {
        if values.prev_playtime > values.playtime {
            values.reset_gameplay();
        }

        values.prev_playtime = values.playtime;

        let gameplay_base = 
            p.read_i32((ruleset_addr + 0x68) as usize)? as usize;
        let score_base = p.read_i32(gameplay_base + 0x38)? as usize;


        let hit_errors_base = (
            p.read_i32(score_base + 0x38)?
        ) as usize;

        p.read_i32_array(
            hit_errors_base,
            &mut values.hit_errors
        )?;

        values.unstable_rate = values.calculate_unstable_rate();

        values.mode = p.read_i32(score_base + 0x64)?;

        // store the converted map so it's not converted 
        // everytime it's used for pp calc
        if new_map {
            if let Some(map) = &values.current_beatmap {
                if let Cow::Owned(converted) = map
                    .convert_mode(values.gamemode()) 
                {
                    values.current_beatmap = Some(converted);
                }
            }
        }

        values.hit_300 = p.read_i16(score_base + 0x8a)?;
        values.hit_100 = p.read_i16(score_base + 0x88)?;
        values.hit_50 = p.read_i16(score_base + 0x8c)?;

        values.hit_geki = p.read_i16(score_base + 0x8e)?;
        values.hit_katu = p.read_i16(score_base + 0x90)?;
        values.hit_miss = p.read_i16(score_base + 0x92)?;
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
        if let Some(beatmap) = &values.current_beatmap {
            let mode = values.gamemode();
            let passed_objects = values.passed_objects()?;

            values.passed_objects = passed_objects;

            let pp_current = AnyPP::new(beatmap)
                .mods(values.mods)
                .mode(mode)
                .passed_objects(passed_objects)
                .state(ScoreState {
                    max_combo: values.max_combo as usize,
                    n_geki: values.hit_geki as usize,
                    n_katu: values.hit_katu as usize,
                    n300: values.hit_300 as usize,
                    n100: values.hit_100 as usize,
                    n50: values.hit_50 as usize,
                    n_misses: values.hit_miss as usize,
                })
                .calculate();

            values.current_pp = pp_current.pp();

            let fc_pp = AnyPP::new(beatmap)
                .mods(values.mods)
                .mode(mode)
                .n300(values.hit_300 as usize)
                .n100(values.hit_100 as usize)
                .n50(values.hit_50 as usize)
                .n_geki(values.hit_geki as usize)
                .n_katu(values.hit_katu as usize)
                .n_misses(values.hit_miss as usize)
                .calculate();

            values.fc_pp = fc_pp.pp();
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.osu_path.exists() {
        return Err(Report::msg(
            format!(
                "Provided osu path doesn't exists!\n Path: {}",
                &args.osu_path.to_str().unwrap()
            )
        ));
    }

    let (tx, rx) = bounded::<WebSocketStream<Async<TcpStream>>>(20);

    std::thread::spawn(move || server_thread(tx.clone()));

    let mut client_id = 0;
    let mut clients: HashMap<usize, WebSocketStream<Async<TcpStream>>> = 
        HashMap::new();

    let mut values = Values::default();
    let mut static_adresses = StaticAdresses::default();
    
    // TODO ugly nesting mess
    'init_loop: loop {
        let p = match Process::initialize("osu!.exe") {
            Ok(p) => p,
            Err(e) => {
                println!("{:?}", Report::new(e));
                continue 'init_loop
            },
        };

        println!("Reading static signatures...");
        match read_static_adresses(&p, &mut static_adresses) {
            Ok(_) => {},
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
            while let Ok(client) = rx.try_recv() {
                clients.insert(client_id, client);
                client_id += 1;
            }

            if let Err(e) = process_reading_loop(
                &p,
                &args,
                &static_adresses,
                &mut values
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

            clients.retain(|_client_id, websocket| {
                smol::block_on(async {
                    let next_future = websocket.next();
                    let msg_future = 
                        smol::future::poll_once(next_future);

                    #[allow(clippy::collapsible_match)]
                    let msg = match msg_future.await {
                        Some(v) => {
                            match v {
                                Some(Ok(v)) => Some(v),
                                Some(Err(_)) => return false,
                                None => None,
                            }
                        },
                        None => None,
                    };
                    

                    if let Some(tungstenite::Message::Close(_)) = msg {
                        return false;
                    };

                    let _ = websocket.send(
                        Message::Text(
                            serde_json::to_string(&values)
                                .unwrap() 
                        ) // No way serialization gonna fail so
                          // using unwrap
                    ).await;

                    true
                })
            });
            
            std::thread::sleep(args.interval);
        }
    };
}
