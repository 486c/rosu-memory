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
    fn initialize(proc_name: &str) -> Result<Self, ProcessError> {
        todo!()
    }

    fn find_process(proc_name: &str) -> Result<Self, ProcessError> {
        todo!()
    }

    fn read_regions(self) -> Result<Self, ProcessError> {
        todo!()
    }

    fn read_signature(
        &self, 
        sign: &rosu_memory::memory::signature::Signature
    ) -> Result<Option<usize>, ProcessError> {
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
    let mut file = File::open("./tests/files/test_uleb")
        .unwrap();

    let mut buff = Vec::new();

    file.read_to_end(&mut buff).unwrap();

    let p = FakeProccess {
        buff
    };

    let s = p.read_string(0).unwrap();

    assert_eq!(s, "test".to_owned())
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
