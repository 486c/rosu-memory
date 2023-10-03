use std::{str::FromStr, time::Duration, collections::HashMap, net::TcpStream};
use async_tungstenite::WebSocketStream;
use crossbeam_channel::bounded;

use rosu_memory::{memory::{process::{Process, ProcessTraits}, signature::Signature}, websockets::server_thread};
use miniserde::{json, Serialize};
use async_tungstenite::tungstenite;
use futures_util::sink::SinkExt;
use smol::{prelude::*, Async};
use tungstenite::Message;

#[derive(Debug, Default, Serialize)]
pub struct Values {
    artist: String,
    beatmap_file: String,

    ar: f32,
    cs: f32,
    hp: f32,
    od: f32,

    plays: i32,
}

fn main() {

    let (tx, rx) = bounded::<WebSocketStream<Async<TcpStream>>>(20);

    std::thread::spawn(move || server_thread(tx.clone()));

    let mut client_id = 0;
    let mut clients: HashMap<usize, WebSocketStream<Async<TcpStream>>> = 
        HashMap::new();

    let mut values = Values::default();

    let p = Process::initialize("osu!.exe").unwrap();
    
    println!("Reading static signatures...");
    let base_sign = Signature::from_str("F8 01 74 04 83 65").unwrap();
    //let status_sign = Signature::from_str("48 83 F8 04 73 1E").unwrap();

    let base = p.read_signature(&base_sign).unwrap().unwrap();
    //let status = p.read_signature(&status_sign).unwrap().unwrap();

    println!("Starting reading loop");

    loop {
        // Receive new WebSocket clients if there any
        while let Ok(client) = rx.try_recv() {
            clients.insert(client_id, client);
            client_id += 1;
        }

        let beatmap_ptr = p.read_i32(base - 0xC).unwrap();
        let beatmap_addr = p.read_i32(beatmap_ptr as usize).unwrap();
        
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

        let path_addr = p.read_i32((beatmap_addr + 0x94) as usize).unwrap();
        values.beatmap_file = p.read_string(path_addr as usize).unwrap();

        // web sockets handler
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
