// Ignoring this clippy warning because one edge case
// with prim_read_array_impl(u8).
// std::slice::from_raw_parts_mut(..) expects
// to use element count (see docs) not bytes and in u8 case it becomes
// something like: 
// buff.len() * std::mem::size_of::<u8>()
// ^                  ^ always 1
// |
// valid length
// valid length * 1 = valid_length
// So clippy thinks we passing bytes count not element count

#![allow(clippy::size_of_in_element_count)]

use std::mem::{size_of, align_of};
use std::path::PathBuf;

use super::error::ProcessError;
use super::signature::Signature;
use paste::paste;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[derive(Debug)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize
}

macro_rules! prim_read_impl {
    ($t: ident) => {
        paste! {
            fn [<read_ $t>](
                &self,
                addr: usize
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
                addr: usize,
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
                    items_ptr as usize + 8,
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
    ) -> Result<usize, ProcessError>;

    fn read(
        &self, 
        addr: usize, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError>;

    fn read_struct<T: Sized>(
        &self,
        addr: usize
    ) -> Result<T, ProcessError> {
        let mut buff = vec![0u8; size_of::<T>()];

        let byte_buff = unsafe {
            std::slice::from_raw_parts_mut(
                buff.as_mut_ptr() as *mut u8,
                buff.len()
            )
        };

        self.read(addr, byte_buff.len(), byte_buff)?;

        let s: T = unsafe { std::ptr::read(buff.as_ptr() as *const _) };

        Ok(s)
    }

    fn read_struct_array<T: Sized>(
        &self,
        addr: usize,
        len: usize
    ) -> Result<Vec<T>, ProcessError> {
        let size = size_of::<T>() + align_of::<T>();
        let mut buff = vec![0u8; size * len];

        let mut byte_buff = unsafe {
            std::slice::from_raw_parts_mut(
                buff.as_mut_ptr() as *mut u8,
                buff.len()
            )
        };
        
        self.read(addr, byte_buff.len(), byte_buff)?;

        let mut arr = Vec::with_capacity(len);

        while byte_buff.len() >= size_of::<T>() {
            let (head, tail) = byte_buff.split_at_mut(size);
            let s: T = unsafe { std::ptr::read(head.as_ptr() as *const T) };
            arr.push(s);
            byte_buff = tail;
        }

        Ok(arr)
    }

    fn read_struct_ptr_array<T: Sized + Clone>(
        &self,
        addr: usize
    ) -> Result<Vec<T>, ProcessError> {
        let mut ptrs = Vec::new();
        self.read_u32_array(addr, &mut ptrs)?;

        let mut arr = Vec::with_capacity(ptrs.len());
        if ptrs.len() == 0 {
            return Ok(arr)
        }
        let size = size_of::<T>();
        let size_with_align = size + align_of::<T>();
        let mut chunk: usize = 1;
        let mut last_ptr = 0; 

        // Reading all values one-by-one is slow and wasteful
        // but List<> elements are stored in chunks
        // so we can find those and read multiple values at once
        for (i, ptr) in ptrs.iter().enumerate() {
            if i == 0 { last_ptr = *ptr; continue }
            // BRUH
            if (ptr.overflowing_sub(last_ptr).0) as usize == size_with_align {
                chunk += 1;
            } else {
                if chunk > 1 {
                    let mut a = self.read_struct_array(last_ptr as usize - size_with_align * (chunk - 1), chunk)?;
                    arr.append(&mut a);
                    chunk = 1;
                } else {
                    let a = self.read_struct(last_ptr as usize)?;
                    arr.push(a);
                }
            }
            last_ptr = *ptr;
        }

        if chunk > 1 {
            let mut a = self.read_struct_array(last_ptr as usize - size_with_align * (chunk - 1), chunk)?;
            assert!(a.len() == chunk);
            arr.append(&mut a);
        } else {
            let a = self.read_struct(last_ptr as usize)?;
            arr.push(a);
        }

        Ok(arr)
    }

    fn read_uleb128(
        &self,
        mut addr: usize
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
        mut addr: usize
    ) -> Result<String, ProcessError> {
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

