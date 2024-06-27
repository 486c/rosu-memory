use std::path::PathBuf;

use super::error::ProcessError;
use super::signature::Signature;
use paste::paste;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[derive(Debug)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize, 
}

macro_rules! prim_read_impl {
    ($t: ident) => {
        paste! {
            fn [<read_ $t>](
                &self,
                addr: i32
            ) -> Result<$t, ProcessError> {
                let mut bytes = [0u8; std::mem::size_of::<$t>()];
                self.read(addr, std::mem::size_of::<$t>(), &mut bytes)?;


                Ok($t::from_le_bytes(bytes))
            }
        }
    }
}

macro_rules! prim_read_array_impl {
    ($t: ident) => {
        paste! {
            fn [<read_ $t _array>](
                &self,
                addr: i32,
                buff: &mut Vec<$t>
            ) -> Result<(), ProcessError> {
                let items_ptr = self.read_i32(addr + 4)?;
                let size = self.read_i32(addr + 12)? as usize;

                buff.resize(size, 0 as $t);

                let byte_buff = unsafe { std::slice::from_raw_parts_mut(
                    buff.as_mut_ptr() as *mut u8,
                    buff.len() * std::mem::size_of::<$t>()
                ) };


                self.read(
                    items_ptr + 8,
                    size * std::mem::size_of::<$t>(),
                    byte_buff
                )?;

                Ok(())
            }
        }
    }
}

pub struct Process {
    #[cfg(target_os = "linux")]
    pub pid: i32,

    #[cfg(target_os = "windows")]
    pub pid: u32,

    #[cfg(target_os = "windows")]
    pub handle: HANDLE,

    pub maps: Vec<MemoryRegion>,
    pub executable_dir: Option<PathBuf>,
}

pub trait ProcessTraits where Self: Sized {
    fn initialize(proc_name: &str) -> Result<Self, ProcessError>;
    fn find_process(proc_name: &str) -> Result<Self, ProcessError>;
    fn read_regions(self) -> Result<Self, ProcessError>;

    fn read_signature(
        &self, 
        sign: &Signature
    ) -> Result<i32, ProcessError>;

    fn read(
        &self, 
        addr: i32, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError>;

    fn read_uleb128(
        &self,
        mut addr: i32,
    ) -> Result<u64, ProcessError> {
        let mut value: u64 = 0;
        let mut bytes_read = 0;

        loop {
            let byte = self.read_u8(addr)?;
            addr += 1;

            let byte_value = (byte & 0b0111_1111) as u64;
            value |= byte_value << (7 * bytes_read);

            bytes_read += 1;

            if (byte &!0b0111_1111) == 0 {
                break;
            }
        }

        Ok(value)
    }

    fn read_string(
        &self,
        addr: i32,
    ) -> Result<String, ProcessError> {
        let mut addr = self.read_i32(addr)?;
        // C# string structure: 4B obj header, 4B str len, str itself
        let len = self.read_u32(addr + 0x4)? as usize;
        addr += 0x8;

        let mut buff = vec![0u16; len];

        let byte_buff = unsafe {
            std::slice::from_raw_parts_mut(
                buff.as_mut_ptr() as *mut u8,
                buff.len() * 2
            )
        };

        self.read(addr, byte_buff.len(), byte_buff)?;

        Ok(String::from_utf16_lossy(&buff))
    }
    
    prim_read_impl!(i8);
    prim_read_impl!(i16);
    prim_read_impl!(i32);
    prim_read_impl!(i64);
    prim_read_impl!(i128);

    prim_read_impl!(u8);
    prim_read_impl!(u16);
    prim_read_impl!(u32);
    prim_read_impl!(u64);
    prim_read_impl!(u128);

    prim_read_impl!(f32);
    prim_read_impl!(f64);

    prim_read_array_impl!(i8);
    prim_read_array_impl!(i16);
    prim_read_array_impl!(i32);
    prim_read_array_impl!(i64);
    prim_read_array_impl!(i128);

    prim_read_array_impl!(u8);
    prim_read_array_impl!(u16);
    prim_read_array_impl!(u32);
    prim_read_array_impl!(u64);
    prim_read_array_impl!(u128);

    prim_read_array_impl!(f32);
    prim_read_array_impl!(f64);
}

