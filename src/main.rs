use std::str::FromStr;
use memory_tests::memory::{process::{Process, ProcessTraits}, signature::Signature};

fn main() {
    let p = Process::initialize("osu!.exe").unwrap();
    let base = Signature::from_str("F8 01 74 04 83 65").unwrap();
    dbg!(p.read_signature(&base).unwrap());
}
