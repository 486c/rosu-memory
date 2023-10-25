use std::fs::File;
use std::io::Read;
use rand::prelude::*;
use paste::paste;

use rosu_memory::memory::process::ProcessTraits;
use rosu_memory::memory::error::*;

macro_rules! prim_read_test {
    ($t: ident) => {
        paste! {
            #[test]
            fn [<test_ $t>]() {
                let mut rng = rand::thread_rng();
                let num: $t = rng.gen();

                let buff: Vec<u8> = num.to_le_bytes().to_vec();
                let p = FakeProccess {
                    buff
                };

                let tmp = p.[<read_ $t>](0).unwrap();
                assert_eq!(tmp, num);
            }
        }
    }
}

pub struct FakeProccess {
    buff: Vec<u8>,
}

impl ProcessTraits for FakeProccess {
    fn initialize(_proc_name: &str) -> Result<Self, ProcessError> {
        todo!()
    }

    fn find_process(_proc_name: &str) -> Result<Self, ProcessError> {
        todo!()
    }

    fn read_regions(self) -> Result<Self, ProcessError> {
        todo!()
    }

    fn read_signature(
        &self, 
        _sign: &rosu_memory::memory::signature::Signature
    ) -> Result<usize, ProcessError> {
        todo!()
    }

    fn read(
        &self, 
        addr: usize, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError> {
        // Addr - starting index
        //self.buff.set_position(addr as u64);
        //self.buff.read(buff);

        let mut slice = &self.buff[addr..addr+len];
        let _ = slice.read(buff);

        Ok(())
    }
}

#[test]
fn test_uleb() {
    let mut file = File::open("./tests/files/test_uleb")
        .unwrap();

    let mut buff = Vec::new();

    file.read_to_end(&mut buff).unwrap();

    let p = FakeProccess {
        buff
    };
    
    // Skipping that one 0x0b byte
    assert_eq!(p.read_uleb128(1).unwrap(), 4);
}

#[test]
fn test_string() {
    // TODO rewrite this test by generating random string

    let mut buff: Vec<u8> = vec![0x0, 0x0, 0x0, 0x0]; // Random 4 bytes

    let s = 0x08u32.to_le_bytes();
    buff.extend_from_slice(&s);

    let test_string_bytes: Vec<u8> = vec![
        83,
        0,
        101,
        0,
        114,
        0,
        101,
        0,
        110,
        0,
        105,
        0,
        116,
        0,
        121,
        0,
    ];

    buff.extend_from_slice(&test_string_bytes);

    let p = FakeProccess {
        buff
    };

    let len = p.read_u32(0x4).unwrap();
    assert_eq!(len, 8);

    let s = p.read_string(0).unwrap();
    assert_eq!(s, "Serenity");
}

prim_read_test!(i8);
prim_read_test!(i16);
prim_read_test!(i32);
prim_read_test!(i64);
prim_read_test!(i128);

prim_read_test!(u8);
prim_read_test!(u16);
prim_read_test!(u32);
prim_read_test!(u64);
prim_read_test!(u128);

prim_read_test!(f32);
prim_read_test!(f64);
