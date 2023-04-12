pub mod memory;

use std::str::FromStr;

use crate::memory::{process::{Process, ProcessTraits}, signature::Signature};

fn main() {
    let p = Process::initialize("osu!.exe").unwrap();
    let base = Signature::from_str("F8 01 74 04 83 65").unwrap();
    p.read_signature(&base).unwrap();
}
