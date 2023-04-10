use std::str::FromStr;

pub enum SignatureByte {
    Byte(u8),
    Any,
}

impl std::str::FromStr for SignatureByte {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "??" => Ok(Self::Any),
            _ => Ok(Self::Byte(u8::from_str_radix(s, 16)?)),
        }
       }
}

pub struct Signature {
    pub bytes: Vec<SignatureByte>,
}

impl std::str::FromStr for Signature {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = Vec::new();

        for c in s.split(" ") {
            let b = SignatureByte::from_str(c)?;
            bytes.push(b);
        }

        Ok(Self { bytes })
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
    
    /*
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
    */
    
    /*
    #[test]
    fn test_find_sig() {
        //              0     1     2     3     4     5     6     7
        let buff = vec![0xFF, 0x30, 0xA3, 0x50, 0x12, 0xAB, 0x2B, 0xCB];

        let sig = Signature::from_str("AB 2B CB").unwrap();
        let s = find_pattern(&buff, &sig).unwrap();
        assert_eq!(s, 5);

        let sig2 = Signature::from_str("AB ?? CB").unwrap();
        let s = find_pattern(&buff, &sig2).unwrap();
        assert_eq!(s, 5);

        let sig3 = Signature::from_str("30 ?? 50").unwrap();
        let s = find_pattern(&buff, &sig3).unwrap();
        assert_eq!(s, 1);

        let sig4 = Signature::from_str("FF ?? ?? 50").unwrap();
        let s = find_pattern(&buff, &sig4).unwrap();
        assert_eq!(s, 0);

        let sig5 = Signature::from_str("12 AB ?? CB").unwrap();
        let s = find_pattern(&buff, &sig5).unwrap();
        assert_eq!(s, 4);

    }
    */
}

