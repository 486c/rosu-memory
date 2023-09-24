use std::{str::FromStr, time::Duration};
use rosu_memory::memory::{process::{Process, ProcessTraits}, signature::Signature};

fn main() {
    let p = Process::initialize("osu!.exe").unwrap();

    let base_sign = Signature::from_str("F8 01 74 04 83 65").unwrap();
    let status_sign = Signature::from_str("48 83 F8 04 73 1E").unwrap();

    let base = p.read_signature(&base_sign).unwrap().unwrap();
    let status = p.read_signature(&status_sign).unwrap().unwrap();

    loop {
        let beatmap_addr = p.read_i32((base - 0xC) as usize).unwrap();

        let ar_addr = p.read_i32(beatmap_addr as usize).unwrap() + 0x2c;

        let ar = p.read_f32(ar_addr as usize).unwrap();

        dbg!(&beatmap_addr);
        dbg!(&ar);

        //let ar_addr = read_i32(beatmap_addr as i64) + 0x2c;

        //let ar = read_f32(ar_addr as i64);

        //let base_ptr = p.read_u64(base).unwrap();
        /*
        let status = p.read_u32((status - 0x4) as usize).unwrap();

        dbg!(&status);

        let beatmap_addr = base - 0xC;

        let beatmap_ptr = p.read_u64(beatmap_addr as usize).unwrap();

        dbg!(&beatmap_ptr);
        */

        std::thread::sleep(Duration::from_secs(1));
    }
}
