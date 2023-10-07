use std::{
    str::FromStr, 
    time::Duration, 
    collections::HashMap, net::TcpStream, path::PathBuf
};

use clap::Parser;

use async_tungstenite::WebSocketStream;
use crossbeam_channel::bounded;

use miniserde::{json, Serialize};
use async_tungstenite::tungstenite;
use futures_util::sink::SinkExt;
use rosu_pp::{Beatmap, AnyPP};
use smol::{prelude::*, Async};
use tungstenite::Message;

use rosu_memory::{
    memory::{
        process::{Process, ProcessTraits}, 
        signature::Signature
    }, 
    websockets::server_thread
};

use eyre::{Report, Result};

#[derive(Parser, Debug)]
pub struct Args {
    // Path to osu folder
    #[arg(short, long)]
    osu_path: PathBuf,
}

#[derive(Debug, Default, Serialize)]
pub struct Values {
    artist: String,
    folder: String,
    beatmap_file: String,

    status: u32,

    ar: f32,
    cs: f32,
    hp: f32,
    od: f32,
    
    // Gameplay info
    hit_300: i16,
    hit_100: i16,
    hit_50: i16,
    hit_geki: i16,
    hit_katu: i16,
    hit_miss: i16,
    combo: i16,
    max_combo: i16,
    mode: i32,

    // Calculated each iteration
    current_pp: f64,

    mods: u32,

    plays: i32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.osu_path.exists() {
        return Err(Report::msg("Provided path doesn't exists!"));
    }

    let (tx, rx) = bounded::<WebSocketStream<Async<TcpStream>>>(20);

    std::thread::spawn(move || server_thread(tx.clone()));

    let mut client_id = 0;
    let mut clients: HashMap<usize, WebSocketStream<Async<TcpStream>>> = 
        HashMap::new();

    let mut values = Values::default();

    let p = Process::initialize("osu!.exe").unwrap();
    
    println!("Reading static signatures...");
    let base_sign = Signature::from_str("F8 01 74 04 83 65")?;
    let status_sign = Signature::from_str("48 83 F8 04 73 1E")?;
    let menu_mods_sign = Signature::from_str("C8 FF ?? ?? ?? ?? ?? 81 0D ?? ?? ?? ?? 00 08 00 00")?;
    let rulesets_sign = Signature::from_str("7D 15 A1 ?? ?? ?? ?? 85 C0")?;

    let base = p.read_signature(&base_sign).unwrap().unwrap();
    let status = p.read_signature(&status_sign).unwrap().unwrap();
    let menu_mods = p.read_signature(&menu_mods_sign).unwrap().unwrap();
    let rulesets = p.read_signature(&rulesets_sign).unwrap().unwrap();

    println!("Starting reading loop");

    let mut cur_beatmap: Option<Beatmap> = None;

    loop {
        // Receive new WebSocket clients if there any
        while let Ok(client) = rx.try_recv() {
            clients.insert(client_id, client);
            client_id += 1;
        }
        
        let menu_mods_ptr = p.read_i32(menu_mods + 0x9).unwrap();
        values.mods = p.read_u32(menu_mods_ptr as usize).unwrap();

        let beatmap_ptr = p.read_i32(base - 0xC).unwrap();
        let beatmap_addr = p.read_i32(beatmap_ptr as usize).unwrap();

        let status_ptr = p.read_i32(status - 0x4).unwrap();

        values.status = p.read_u32(status_ptr as usize).unwrap();
        
        let ar_addr = beatmap_addr + 0x2c;
        let cs_addr = ar_addr + 0x04;
        let hp_addr = cs_addr + 0x04;
        let od_addr = hp_addr + 0x04;

        values.ar = p.read_f32(ar_addr as usize).unwrap();
        values.cs = p.read_f32(cs_addr as usize).unwrap();
        values.hp = p.read_f32(hp_addr as usize).unwrap();
        values.od = p.read_f32(od_addr as usize).unwrap();
        
        let plays_addr = p.read_i32(base - 0x33).unwrap() + 0xC;
        values.plays = p.read_i32(plays_addr as usize).unwrap();

        let artist_addr = p.read_i32((beatmap_addr + 0x18) as usize).unwrap();
        values.artist = p.read_string(artist_addr as usize).unwrap();

        // TODO Read after status != 0
        let path_addr = p.read_i32((beatmap_addr + 0x94) as usize).unwrap();
        values.beatmap_file = p.read_string(path_addr as usize).unwrap();

        // TODO Read after status != 0
        if values.status != 0 {
            let folder_addr = p.read_i32((beatmap_addr + 0x78) as usize).unwrap();
            let folder = p.read_string(folder_addr as usize).unwrap();
            if folder != values.folder {
                let full_path = args.osu_path
                    .join("Songs")
                    .join(&folder)
                    .join(&values.beatmap_file);

                if full_path.exists() {
                    cur_beatmap = match Beatmap::from_path(full_path) {
                        Ok(beatmap) => Some(beatmap),
                        Err(_) => {
                            println!("Failed to parse beatmap");
                            None
                        },
                    }
                }
            }
            values.folder = folder;
        }

        let ruleset_addr = p.read_i32(
            (p.read_i32(rulesets - 0xb).unwrap() + 0x4) as usize
        ).unwrap();
        
        // TODO
        //if ruleset_addr == 0 {
        //}

        // TODO do not read gameplay info on status 7 and 0 and 5
        if values.status != 7 && values.status != 0 {
            let gameplay_base = p.read_i32((ruleset_addr + 0x68) as usize).unwrap() as usize;
            let score_base = p.read_i32(gameplay_base + 0x38).unwrap() as usize;

            values.mode = p.read_i32(score_base + 0x64).unwrap();

            values.hit_300 = p.read_i16(score_base + 0x8a).unwrap();
            values.hit_100 = p.read_i16(score_base + 0x88).unwrap();
            values.hit_50 = p.read_i16(score_base + 0x8c).unwrap();

            values.hit_geki = p.read_i16(score_base + 0x8e).unwrap();
            values.hit_katu = p.read_i16(score_base + 0x90).unwrap();
            values.hit_miss = p.read_i16(score_base + 0x92).unwrap();
            values.combo = p.read_i16(score_base + 0x94).unwrap();
            values.max_combo = p.read_i16(score_base + 0x68).unwrap();

            // Calculate pp
            if let Some(beatmap) = &cur_beatmap {
                // TODO use mods from gameplay
                let pp_result = AnyPP::new(beatmap)
                    .mods(values.mods)
                    .combo(values.max_combo as usize)
                    .n300(values.hit_300 as usize)
                    .n100(values.hit_100 as usize)
                    .n50(values.hit_50 as usize)
                    .n_misses(values.hit_miss as usize)
                    .n_geki(values.hit_geki as usize)
                    .n_katu(values.hit_katu as usize)
                    .calculate();

                values.current_pp = pp_result.pp();
            }
        }

        // web sockets loop 
        clients.retain(|_client_id, websocket| {
            smol::block_on(async {
                let next_future = websocket.next();
                let msg_future = smol::future::poll_once(next_future);

                #[allow(clippy::collapsible_match)]
                let msg = match msg_future.await {
                    Some(v) => {
                        match v {
                            Some(m) => {
                                match m {
                                    Ok(v) => Some(v),
                                    Err(_) => return false,
                                }
                            },
                            None => None,
                        }
                    },
                    None => None,
                };
                

                if let Some(tungstenite::Message::Close(_)) = msg {
                    return false;
                };

                websocket.send(
                    Message::Text(json::to_string(&values))
                ).await.unwrap();

                true
            })
        });

        std::thread::sleep(Duration::from_secs(1));
    }
}
