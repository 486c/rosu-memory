pub mod memory;
use crate::memory::*;

use core::time::Duration;
use std::thread::sleep;

use cfg_if;
cfg_if::cfg_if! {

    if #[cfg(unix)] {
        pub mod linux;
        use crate::linux::*;
    } else if #[cfg(windows)] {
        pub mod windows;
    } 
}

#[derive(Debug, Default)]
struct StaticAddrs {
    Base: i64,
}

fn main() {
    //TODO get rid of unwrap stuff
    let mut p = Process::find_proc("osu!.exe").unwrap();
    println!("Found the process!!");
    p.read_maps();
    
    /* static */
    let base = p.find_signature("F8 01 74 04 83 65").unwrap();
    let menu_mods = p.find_signature("C8 FF ?? ?? ?? ?? ?? 81 0D ?? ?? ?? ?? 00 08 00 00").unwrap();
    let playtime = p.find_signature("5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04").unwrap();
    let chat_checker = p.find_signature("0A D7 23 3C 00 00 ?? 01").unwrap();
    let skindata = p.find_signature("75 21 8B 1D").unwrap();
    let rulesets = p.find_signature("7D 15 A1 ?? ?? ?? ?? 85 C0").unwrap();
    let chat_area = p.find_signature("33 47 9D FF 5B 7F FF FF").unwrap();

    /* Kinda static?? */
    let beatmap_base = (&base - 0xC);

    println!("Found Base signature: 0x{:x}", &base.offset);

    loop {
        let beatmap = beatmap_base.follow_addr();

        let ar = (&beatmap.follow_addr() + 0x2c).read_f32();
        let cs = (&beatmap.follow_addr() + 0x30).read_f32();
        let hp = (&beatmap.follow_addr() + 0x34).read_f32();
        let od = (&beatmap.follow_addr() + 0x38).read_f32();
        println!("{}", ar);
        println!("{}", cs);
        println!("{}", hp);
        println!("{}", od);
        sleep(Duration::from_secs(5));
    }

}
