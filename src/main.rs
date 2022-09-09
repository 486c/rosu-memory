use std::fs;
use i64;
use nix::sys::uio;
use std::io::IoSliceMut;
use nix::unistd::Pid;
use std::thread::sleep;

use std::time::Duration;

mod memory;
use crate::memory::find_signature;

use std::str::FromStr;
use crate::memory::Signature;

struct MemoryRegion {
    from: i64,
    to: i64,
}

fn read_maps(pid: i32) -> Vec<MemoryRegion> {
    let path = format!("/proc/{}/maps", &pid);

    let mut v = Vec::new();
    let mut buff = String::new();

    buff = fs::read_to_string(&path).unwrap();

    for line in buff.split('\n')
    {
        if line.len() == 0 {
            break;
        }

        let mut split = line.split_whitespace();
        let range_raw = split.next().unwrap();
        let mut range_split = range_raw.split('-');

        let from = i64::from_str_radix(range_split.next().unwrap(), 16)
            .unwrap();
        let to = i64::from_str_radix(range_split.next().unwrap(), 16)
            .unwrap();

        v.push(MemoryRegion{from, to});
    }

    v
}

fn read_at<'a>(addr: &'a i64, size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size];

    let remote_iov = uio::RemoteIoVec{
        base: (*addr) as usize,
        len: size,
    };

    let ret = uio::process_vm_readv(
        Pid::from_raw(127227), 
        &mut[IoSliceMut::new(&mut buf)],
        &[remote_iov]
    );

    buf
}

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

#[derive(Debug, Default)]
struct StaticAddrs {
    Base: i64,
}

fn main() {
    let mut addrs = StaticAddrs::default();
    let maps = read_maps(127227);

    for range in maps {
        let mut buf = vec![0u8; (range.to - range.from) as usize];

        let remote_iov = uio::RemoteIoVec{
            base: (range.from) as usize,
            len: (range.to - range.from) as usize,
        };

        let ret = uio::process_vm_readv(
            Pid::from_raw(127227), 
            &mut[IoSliceMut::new(&mut buf)],
            &[remote_iov]
        );
    
        match ret {
            Ok(size) => size,
            Err(_error) => {
                continue;
            },
        };

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

    while true {
        //Check values
        
        let beatmap_addr = read_i32((addrs.Base - 0xC) as i64);
        println!("Beatmap() = 0x{:x}", beatmap_addr);
        
        let ar_addr = read_i32(beatmap_addr as i64) + 0x2c;

        let ar = read_f32(ar_addr as i64);

        println!("{}", ar);

        sleep(Duration::from_secs(5));
    }

}
