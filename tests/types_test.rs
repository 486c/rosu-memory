use std::io::Read;

use rosu_memory::memory::process::ProcessTraits;
use rosu_memory::memory::error::*;

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
    let mut file = std::fs::File::open("./tests/files/test_uleb").unwrap();

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
    let mut file = std::fs::File::open("./tests/files/test_uleb").unwrap();

    let mut buff = Vec::new();

    file.read_to_end(&mut buff).unwrap();

    let p = FakeProccess {
        buff
    };

    let s = p.read_string(0).unwrap();

    assert_eq!(s, "test".to_owned())
}

#[test]
fn test_u8() {
    let buff: Vec<u8> = vec![0x01, 0x0A, 0xFF];
    let p = FakeProccess {
        buff
    };

    let mut tmp = [0u8; 1];
    p.read(0, 1, &mut tmp).unwrap();
    assert!(tmp[0] == 0x01);

    let mut tmp = [0u8; 1];
    p.read(1, 1, &mut tmp).unwrap();
    assert!(tmp[0] == 0x0A);

    let mut tmp = [0u8; 1];
    p.read(2, 1, &mut tmp).unwrap();
    assert!(tmp[0] == 0xFF);
}

#[test]
fn test_u32() {
    let buff: Vec<u8> = 32u32.to_le_bytes().to_vec();
    let mut p = FakeProccess {
        buff
    };

    let tmp = p.read_u32(0).unwrap();
    assert_eq!(tmp, 32 as u32);

    p.buff = 245u32.to_le_bytes().to_vec();
    let tmp = p.read_u32(0).unwrap();
    assert_eq!(tmp, 245 as u32);

    p.buff = 888u32.to_le_bytes().to_vec();
    let tmp = p.read_u32(0).unwrap();
    assert_eq!(tmp, 888 as u32);

    p.buff = 3728123u32.to_le_bytes().to_vec();
    let tmp = p.read_u32(0).unwrap();
    assert_eq!(tmp, 3728123 as u32);
}
