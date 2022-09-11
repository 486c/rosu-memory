use std::ops::{Sub, Add};

use cfg_if;
cfg_if::cfg_if! {

    if #[cfg(unix)] {
        use crate::linux::{Process, ProcessTraits};
    } else if #[cfg(windows)] {
    } 
}

#[derive(Debug, Clone)]
pub enum MemoryErrors {
    PatternNotFound,
    StrToHexFailed,
}

#[derive(Debug)]
pub (crate) struct MemAddress<'a> {
    pub (crate) process: &'a Process,
    pub (crate) offset: i32,
}

pub trait MemAddressTraits {
    fn follow_addr(&self) -> Self;
    fn read_i32(&self) -> i32;
    fn read_f32(&self) -> f32;
}

impl MemAddressTraits for MemAddress<'_> {
    fn follow_addr(&self) -> Self {
        Self { process: self.process, offset: self.read_i32() }
    }

    fn read_i32(&self) -> i32 {
        let vec_buf: Vec<u8> = self.process.read_at(&self.offset, 4).unwrap();
    
        i32::from_le_bytes(
            vec_buf[0..4].try_into().unwrap()
        )
    }
    
    fn read_f32(&self) -> f32 {
        let vec_buf: Vec<u8> = self.process.read_at(&self.offset, 4).unwrap();
    
        f32::from_le_bytes(
            vec_buf[0..4].try_into().unwrap()
        )
    }
}

impl<'a> Sub<i32> for &MemAddress<'a> {
    type Output = MemAddress<'a>;

    fn sub(self, other: i32) -> Self::Output {
        Self::Output { process: self.process, offset: self.offset - other }
    }
}

impl<'a> Add<i32> for &MemAddress<'a> {
    type Output = MemAddress<'a>;

    fn add(self, other: i32) -> Self::Output {
        Self::Output { process: self.process, offset: self.offset + other }
    }
}

#[derive(Debug)]
pub (crate) struct MemoryRegion {
    pub (crate) from: i64,
    pub (crate) to: i64,
}

use std::str::FromStr;
use std::ops::Index;

pub enum SignatureByte {
    Byte(u8),
    Any,
}

impl PartialEq<u8> for SignatureByte {
    fn eq(&self, other: &u8) -> bool {
        match self {
            SignatureByte::Any => true,
            SignatureByte::Byte(b) => b == other,
        }
    }
}

impl FromStr for SignatureByte {
    type Err = MemoryErrors;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq("??") {
            Ok(Self::Any)
        } else
        {
            let status = u8::from_str_radix(s, 16);

            match status {
                Ok(num) => Ok(Self::Byte(num)),
                Err(_) => Err(MemoryErrors::StrToHexFailed),
            }
        }
    }
}

pub struct Signature {
    bytes: Vec<SignatureByte>,
}

impl Signature {
    fn new(bytes: Vec<SignatureByte>) -> Self {
        Self { bytes }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

impl FromStr for Signature {
    type Err = MemoryErrors;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = Vec::new();

        for c in s.split(" ") {
            let b = SignatureByte::from_str(c)?;
            bytes.push(b);
        }

        Ok(Self::new(bytes))
    }
}

impl Index<usize> for Signature {
    type Output = SignatureByte;

    fn index(&self, i: usize) -> &Self::Output {
        &self.bytes[i]
    }
}

pub fn find_signature(
    buff: &Vec<u8>,
    sig: &Signature
) -> Result<i32, MemoryErrors> {
    let mut found = true;
    let mut offset: i32 = 0;

    for i in 0..buff.len() {
        found = true;
        
        // index out of bounds check 
        if i + sig.len() > buff.len() {
            found = false;
            break;
        }

        for j in 0..sig.len() {
            
            if sig[j] != buff[i + j] {
                found = false;
                break;
            }
        }

        if found {
            offset = i as i32;
            break;
        }
    }

    match found {
        true => Ok(offset),
        false => Err(MemoryErrors::PatternNotFound),
    }
    
}

#[test]
fn test_signature_parsing() {
    let s = Signature::from_str("FF 30 A3 50").unwrap();
    assert_eq!(s.len(), 4);

    let s = Signature::from_str("FF 30 A3 50 ?? ?? ?? FF").unwrap();
    assert_eq!(s.len(), 8);

    let s = Signature::from_str("FF 30 A3 50 ?? ?? ?? FF CB FF FF ?? 10 2B 4A ?? ??").unwrap();
    assert_eq!(s.len(), 17);
}

#[test]
fn test_signature_byte() {
    let s = SignatureByte::from_str("AB").unwrap();
    assert!(s == 0xAB);
    assert!(s != 0xFF);
    
    let s = SignatureByte::from_str("??").unwrap();
    assert!(s == 0xAB);
    assert!(s == 0x50);
    assert!(s == 0xFF);
    assert!(s == 0xF3);
    assert!(s == 0xCB);
}

#[test]
fn test_find_sig() {
    //              0     1     2     3     4     5     6     7
    let buff = vec![0xFF, 0x30, 0xA3, 0x50, 0x12, 0xAB, 0x2B, 0xCB];

    let sig = Signature::from_str("AB 2B CB").unwrap();
    let s = find_signature(&buff, &sig).unwrap();
    assert_eq!(s, 5);
    
    let sig2 = Signature::from_str("AB ?? CB").unwrap();
    let s = find_signature(&buff, &sig2).unwrap();
    assert_eq!(s, 5);

    let sig3 = Signature::from_str("30 ?? 50").unwrap();
    let s = find_signature(&buff, &sig3).unwrap();
    assert_eq!(s, 1);

    let sig4 = Signature::from_str("FF ?? ?? 50").unwrap();
    let s = find_signature(&buff, &sig4).unwrap();
    assert_eq!(s, 0);

    let sig5 = Signature::from_str("12 AB ?? CB").unwrap();
    let s = find_signature(&buff, &sig5).unwrap();
    assert_eq!(s, 4);

}
