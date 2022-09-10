pub mod memory;

use cfg_if;
cfg_if::cfg_if! {

    if #[cfg(unix)] {
        pub mod linux;
        use crate::linux::*;
    } else if #[cfg(windows)] {
        pub mod windows;
    } 
}


/*
fn read_i32(addr: i64) -> i32 {
    let vec_buf: Vec<u8> = read_at(&addr, 4);

    let buff: [u8; 4] = vec_buf[0..4].try_into().unwrap();

    i32::from_le_bytes(buff)
}

fn read_i64(addr: i64) -> i64 {
    let vec_buf: Vec<u8> = read_at(&addr, 4);

    let buff: [u8; 8] = vec_buf[0..8].try_into().unwrap();

    i64::from_le_bytes(buff)
}

fn read_f32(addr: i64) -> f32 {
    let vec_buf: Vec<u8> = read_at(&addr, 4);

    let buff: [u8; 4] = vec_buf[0..4].try_into().unwrap();

    f32::from_le_bytes(buff)
}
*/

#[derive(Debug, Default)]
struct StaticAddrs {
    Base: i64,
}

fn main() {
    let mut p = Process::find_proc("osu!.exe").unwrap();
    println!("Found the process!!");

    p.read_maps();

    /*
     *
     * find_signature -------> MemAdress 
     *                             |     +-> read_u32() -> u32
     *                             |     +-> read_i32() -> i32
     *            +--------+       |     +-> read_i64() -> i64
     *            | offset | <-----+-----+-> read_f32() -> f32
     *            +--------+             +-> read_f32arr() -> Vec<f32>
     *                                   +-> ...
     *
     */

    //let p = Process::find_proc("asd");

    /*
    let mut addrs = StaticAddrs::default();
    let maps = read_maps(127227);


        /* Reading here */
        
        let sig = Signature::from_str("F8 01 74 04 83 65").unwrap();
        let ret = find_signature(&buf, &sig);

        match ret {
            Ok(size) => {
                let offset: i64 = range.from + (size as i64);
                println!(
                    "found! 0x{:x} - {}", 
                    offset,
                    offset,
                );

                addrs.Base = offset;
                break;
            },
            Err(_error) => continue,
        }
    }

    loop {
        //Check values
        
        let beatmap_addr = read_i32((addrs.Base - 0xC) as i64);
        println!("Beatmap() = 0x{:x}", beatmap_addr);
        
        let ar_addr = read_i32(beatmap_addr as i64) + 0x2c;

        let ar = read_f32(ar_addr as i64);

        println!("{}", ar);

        sleep(Duration::from_secs(5));
    }
    */

}
