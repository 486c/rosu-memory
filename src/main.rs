use std::{str::FromStr, time::Duration, collections::HashMap, net::TcpStream};
use async_tungstenite::WebSocketStream;
use crossbeam_channel::bounded;

use rosu_memory::{memory::{process::{Process, ProcessTraits}, signature::Signature}, websockets::server_thread};
use miniserde::{json, Serialize};
use async_tungstenite::tungstenite;
use futures::sink::SinkExt;
use smol::{prelude::*, Async};
use tungstenite::Message;

#[derive(Debug, Default, Serialize)]
pub struct Values {
    ar: f32
}

fn main() {

    let (tx, rx) = bounded::<WebSocketStream<Async<TcpStream>>>(20);

    std::thread::spawn(move || server_thread(tx.clone()));

    let mut client_id = 0;
    let mut clients: HashMap<usize, WebSocketStream<Async<TcpStream>>> = 
        HashMap::new();

    let mut values = Values::default();

    let p = Process::initialize("osu!.exe").unwrap();

    let base_sign = Signature::from_str("F8 01 74 04 83 65").unwrap();
    let status_sign = Signature::from_str("48 83 F8 04 73 1E").unwrap();

    let base = p.read_signature(&base_sign).unwrap().unwrap();
    let status = p.read_signature(&status_sign).unwrap().unwrap();

    loop {
        // Receive new WebSocket clients if there any
        while let Ok(client) = rx.try_recv() {
            clients.insert(client_id, client);
            client_id += 1;
        }

        let beatmap_addr = p.read_i32((base - 0xC) as usize).unwrap();
        let ar_addr = p.read_i32(beatmap_addr as usize).unwrap() + 0x2c;
        values.ar = p.read_f32(ar_addr as usize).unwrap();

        clients.retain(|client_id, websocket| {
            smol::block_on(async {
                let next_future = websocket.next();
                let msg_future = smol::future::poll_once(next_future);

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

                if let Some(msg) = msg {
                    match msg {
                        tungstenite::Message::Close(_) => {
                            println!("Client {} disconnected", client_id);
                            return false;
                        },
                        _ => (),
                    };
                }

                websocket.send(
                    Message::Text(json::to_string(&values))
                ).await.unwrap();

                true
            })
        });

        std::thread::sleep(Duration::from_secs(1));
    }
}
