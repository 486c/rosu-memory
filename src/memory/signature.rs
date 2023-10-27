use crate::memory::error::ParseSignatureError;

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SignatureByte {
    Byte(u8),
    Any,
}

impl FromStr for SignatureByte {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "??" => Ok(Self::Any),
            _ => u8::from_str_radix(s, 16).map(Self::Byte),
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
    bytes: Box<[SignatureByte]>,
}

impl FromStr for Signature {
    type Err = ParseSignatureError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() % 3 != 2 {
            return Err(ParseSignatureError::InvalidLength(value.len()));
        }

        let capacity = (value.len() + 2) / 3;
        let mut bytes = Vec::with_capacity(capacity);

        for c in value.split(' ') {
            bytes.push(c.parse()?);
        }

        // making sure there is no excess capacity so converting
        // from Vec to Box does not re-allocate
        debug_assert_eq!(bytes.len(), bytes.capacity());

        Ok(Self {
            bytes: bytes.into_boxed_slice(),
        })
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

/// Find signature inside of [u8] buffer
#[inline]
pub fn find_signature(buff: &[u8], sign: &Signature) -> Option<usize> {
    buff.windows(sign.bytes.len())
        .enumerate()
        .find_map(|(i, window)| (sign.bytes.as_ref() == window).then_some(i))
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
        assert_eq!(s, 0xAB);
        assert_ne!(s, 0xFF);
        assert_ne!(s, 0x50);
        assert_ne!(s, 0xFF);
        assert_ne!(s, 0xF3);
        assert_ne!(s, 0xCB);

        let s = SignatureByte::from_str("??").unwrap();
        assert_eq!(s, 0xAB);
        assert_eq!(s, 0x50);
        assert_eq!(s, 0xFF);
        assert_eq!(s, 0xF3);
        assert_eq!(s, 0xCB);
    }

    #[test]
    fn test_formatting() {
        let s = Signature::from_str("FF 30 A3 50").unwrap();
        assert_eq!(s.bytes.len(), 4);

        assert_eq!("FF 30 A3 50".to_owned(), s.to_string().to_uppercase());
    }
}
