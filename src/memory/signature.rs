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

#[derive(Debug)]
pub struct Signature {
    pub bytes: Vec<SignatureByte>,
}

impl FromStr for Signature {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = Vec::new();

        for c in s.split(' ') {
            let b = SignatureByte::from_str(c)?;
            bytes.push(b);
        }

        Ok(Self { bytes })
    }
}

impl From<&str> for Signature {
    fn from(value: &str) -> Self {
        Signature::from_str(value).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::memory::signature::*;

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

        let s = SignatureByte::from_str("??").unwrap();
        assert!(s == 0xAB);
        assert!(s == 0x50);
        assert!(s == 0xFF);
        assert!(s == 0xF3);
        assert!(s == 0xCB);
    }
}

