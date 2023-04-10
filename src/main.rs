pub mod memory;

use crate::memory::process::{Process, ProcessTraits};

fn main() {
    let p = Process::initialize("osu!.exe").unwrap();

    dbg!(p);

    /*

    println!("Found the process!!");
    p.read_maps();
    
    println!("Finding signatures...");

    /* static */
    let base = p.find_signature("F8 01 74 04 83 65").unwrap();
    let menu_mods = p.find_signature(
        "C8 FF ?? ?? ?? ?? ?? 81 0D ?? ?? ?? ?? 00 08 00 00"
    ).unwrap();
    let playtime = p.find_signature(
        "5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04"
    ).unwrap();
    let chat_checker = p.find_signature("0A D7 23 3C 00 00 ?? 01").unwrap();
    let skindata = p.find_signature("75 21 8B 1D").unwrap();
    let rulesets = p.find_signature("7D 15 A1 ?? ?? ?? ?? 85 C0").unwrap();
    let chat_area = p.find_signature("33 47 9D FF 5B 7F FF FF").unwrap();

    /* Kinda static?? */
    let beatmap_base = &base - 0xC;
    
    println!("Starting web socket server!");
    let event_hub = simple_websockets::launch(24050)
        .expect("Failed to start websocket server on port 24050");
    let mut clients: HashMap<u64, Responder> = HashMap::new();

    let mut json_data = json!({
        "beatmap": {
            "AR": -1,
            "CS": -1,
            "OD": -1,
            "HP": -1,
        }
    });

    loop {
        let beatmap = beatmap_base.follow_addr();

        let ar = (&beatmap.follow_addr() + 0x2c).read_f32();
        let cs = (&beatmap.follow_addr() + 0x30).read_f32();
        let hp = (&beatmap.follow_addr() + 0x34).read_f32();
        let od = (&beatmap.follow_addr() + 0x38).read_f32();

        let folder = (&beatmap.follow_addr() + 0x78)
            .follow_addr()
            .read_string();

        json_data["beatmap"]["AR"] = json!(ar);
        json_data["beatmap"]["CS"] = json!(cs);
        json_data["beatmap"]["OD"] = json!(od);
        json_data["beatmap"]["HP"] = json!(hp);


        match event_hub.next_event() {
            None => {},
            Some(event) => match event {
                Event::Connect(id, responder) => {
                    clients.insert(id, responder);
                },
                Event::Disconnect(id) => {
                    clients.remove(&id);
                },
                Event::Message(_id, _responder) => {},
            }
        }

        for (_key, responder) in &clients {
            responder.send(
                Message::Text(
                    serde_json::to_string(&json_data).unwrap()
                )
            );
        }

        sleep(Duration::from_secs(5));
    }
        */

}
