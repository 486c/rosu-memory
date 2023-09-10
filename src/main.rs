use std::str::FromStr;
use rosu_memory::memory::{process::{Process, ProcessTraits}, signature::Signature};

fn main() {
    let p = Process::initialize("osu!.exe").unwrap();
    let base = Signature::from_str("F8 01 74 04 83 65").unwrap();

    let addr = p.read_signature(&base).unwrap().unwrap();
    dbg!(&addr);

    let test = p.read_i32(addr).unwrap();
    dbg!(&test);
}
