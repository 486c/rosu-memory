use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum SignatureByte {
    Byte(u8),
    Any,
}

impl FromStr for SignatureByte {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "??" => Ok(Self::Any),
            _ => Ok(Self::Byte(u8::from_str_radix(s, 16)?)),
        }
    }
}

impl PartialEq<u8> for SignatureByte {
    fn eq(&self, other: &u8) -> bool {
        match self {
            SignatureByte::Any => true,
            SignatureByte::Byte(b) => b == other,
        }
    }
}

impl Display for SignatureByte {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            SignatureByte::Byte(byte) => write!(f, "{byte:X}"),
            SignatureByte::Any => f.write_str("??"),
        }
    }
}

#[derive(Debug)]
pub struct Signature {
    pub bytes: Vec<SignatureByte>,
}

impl FromStr for Signature {
    type Err = std::num::ParseIntError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut bytes = Vec::new();

        for c in value.split(' ') {
            let b = SignatureByte::from_str(c)?;
            bytes.push(b);
        }

        Ok(Self { bytes })
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut bytes = self.bytes.iter();

        if let Some(byte) = bytes.next() {
            Display::fmt(byte, f)?;

            for byte in bytes {
                write!(f, " {byte}")?;
            }
        }

        Ok(())
    }
}

// Find signature inside of [u8] buffer
#[inline]
pub fn find_signature(buff: &[u8], sign: &Signature) -> Option<usize> {
    let mut i = 0;
    let mut found = true;

    while i + sign.bytes.len() <= buff.len() {
        for j in 0..sign.bytes.len() {
            if sign.bytes[j] != buff[i + j] {
                found = false;
                break;
            }
        }

        if found {
            return Some(i);
        }

        found = true;

        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::memory::signature::*;
    use std::str::FromStr;

    #[test]
    fn test_find_sig() {
        //              0     1     2     3     4     5     6     7
        let buff = vec![0xFF, 0x30, 0xA3, 0x50, 0x12, 0xAB, 0x2B, 0xCB];

        let sig = Signature::from_str("AB 2B CB").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 5);

        let sig = Signature::from_str("AB ?? CB").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 5);

        let sig = Signature::from_str("30 ?? 50").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 1);

        let sig = Signature::from_str("FF ?? ?? 50").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 0);

        let sig = Signature::from_str("12 AB ?? CB").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 4);

        let sig = Signature::from_str("50 12 AB").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 3);

        let sig = Signature::from_str("?? 30 ?? ?? ?? ?? ?? CB").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 0);

        let sig = Signature::from_str("FF 30 A3 50 12 ?? ?? CB").unwrap();
        let s = find_signature(&buff, &sig).unwrap();
        assert_eq!(s, 0);
    }

    #[test]
    fn test_signature_parsing() {
        let s = Signature::from_str("FF 30 A3 50").unwrap();
        assert_eq!(s.bytes.len(), 4);

        let s = Signature::from_str("FF 30 A3 50 ?? ?? ?? FF").unwrap();
        assert_eq!(s.bytes.len(), 8);

        let s = Signature::from_str("FF 30 A3 50 ?? ?? ?? FF CB FF FF ?? 10 2B 4A ?? ??").unwrap();
        assert_eq!(s.bytes.len(), 17);
    }

    #[test]
    fn test_signature_byte() {
        let s = SignatureByte::from_str("AB").unwrap();
        assert!(s == 0xAB);
        assert!(s != 0xFF);
        assert!(s != 0x50);
        assert!(s != 0xFF);
        assert!(s != 0xF3);
        assert!(s != 0xCB);

        let s = SignatureByte::from_str("??").unwrap();
        assert!(s == 0xAB);
        assert!(s == 0x50);
        assert!(s == 0xFF);
        assert!(s == 0xF3);
        assert!(s == 0xCB);
    }

    #[test]
    fn test_formatting() {
        let s = Signature::from_str("FF 30 A3 50").unwrap();
        assert_eq!(s.bytes.len(), 4);

        assert_eq!("FF 30 A3 50".to_owned(), s.to_string().to_uppercase());
    }
}
