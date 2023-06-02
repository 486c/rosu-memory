pub mod process;
pub mod signature;

mod error;

use cfg_if;

use self::signature::{Signature, SignatureByte};

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
    } else if #[cfg(target_os = "windows")] {
        mod windows;
        use self::windows::*;
    } 
}

// Find signature inside of [u8] buffer
#[inline]
fn find_signature(buff: &[u8], sign: &Signature) -> Option<usize> {
    let mut i = 0;
    let mut found = true;

    while i + sign.bytes.len() <= buff.len() {
        for j in 0..sign.bytes.len() {
            if sign.bytes[j] != buff[i + j]
            && sign.bytes[j] != SignatureByte::Any {
                found = false;
                break;
            }
        }

        if found {
            return Some(i);
        } else {
            found = true;
        }

        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::memory::{signature::Signature, find_signature};

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

    }
}

