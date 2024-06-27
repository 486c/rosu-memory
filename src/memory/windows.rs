use std::ffi::c_void;
use std::path::PathBuf;

use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::Memory::MEMORY_BASIC_INFORMATION;
use windows::Win32::System::Memory::VirtualQueryEx;
use windows::Win32::System::Memory::MEM_FREE;
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::ProcessStatus::EnumProcesses;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExA;

use crate::memory::{
    process::{ Process, MemoryRegion, ProcessTraits }, 
    signature::find_signature
};

use super::signature::Signature;
use super::error::ProcessError;

use windows::Win32::{
    System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ
    }, 
    Foundation::{
        CloseHandle, FALSE, HANDLE
    }
};

impl ProcessTraits for Process {
    fn initialize(proc_name: &str) -> Result<Process, ProcessError> {
        let process = Process::find_process(proc_name)?;
        process.read_regions()
    }

    fn find_process(proc_name: &str) -> Result<Process, ProcessError> {
        let mut processes = [0u32; 512];
        let mut returned: u32 = 0;

        let res = unsafe { EnumProcesses(
            processes.as_mut_slice().as_mut_ptr() as _,
            std::mem::size_of::<u32>() as u32 * 512,
            &mut returned
        )};

        res.ok()?;

        let length = returned as usize / std::mem::size_of::<u32>();

        for pid in &processes[0..length] {
            let handle = match unsafe { OpenProcess(
                PROCESS_QUERY_INFORMATION|PROCESS_VM_READ, 
                FALSE, 
                *pid
            )} {
                Ok(h) => h,
                Err(_) => continue,
            };

            let mut string_buff = [0u8; 256];

            let size = unsafe {
              GetModuleFileNameExA(handle, HMODULE(0), string_buff.as_mut_slice())
            };

            let name = std::str::from_utf8(
                &string_buff[0..size as usize]
            )?;

            if name.contains(proc_name) {
                let executable_path = PathBuf::from(name);
                let executable_dir = executable_path.parent()
                    .map(|v| v.to_path_buf());

                return Ok(Process {
                    pid: *pid,
                    handle,
                    maps: Vec::new(),
                    executable_dir
                })
            } else {
                unsafe { CloseHandle(handle) };
            }
        }

        Err(ProcessError::ProcessNotFound)
    }

    fn read_regions(mut self) -> Result<Process, ProcessError> {
        let mut info = MEMORY_BASIC_INFORMATION::default();
        let mut address: usize = 0;

        while unsafe { VirtualQueryEx(
            self.handle,
            Some(address as _),
            &mut info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>()
        )} != 0 {
            address = (info.BaseAddress as usize) + info.RegionSize;

            if info.State != MEM_FREE
            {
                self.maps.push( MemoryRegion {
                    from: info.BaseAddress as usize,
                    size: info.RegionSize,
                })
            }
        };

        Ok(self)
    }

    fn read_signature(
        &self, 
        sign: &Signature
    ) -> Result<i32, ProcessError> {
        let mut buf = Vec::new();
        let mut bytesread: usize = 0;

        for region in self.maps.iter() {
            buf.resize(region.size, 0);

            let res = unsafe { ReadProcessMemory(
                self.handle,
                region.from as *mut c_void,
                buf.as_mut_ptr() as *mut c_void,
                region.size,
                Some(&mut bytesread)
            ) };

            if let Err(error) = res.ok() {
                // Stupid error code that we should
                // ignore during memory regions
                // collection
                if error.code().0 == -2147024597 {
                    continue
                }

                return Err(error.into());
            }

            if let Some(offset) = find_signature(&buf[..bytesread], sign) {
                return Ok((region.from + offset) as i32)
            }
        }

        Err(ProcessError::SignatureNotFound(sign.to_string()))
    }

    fn read(
        &self, 
        addr: i32, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError> {
        let mut n = 0;

        let res = unsafe { ReadProcessMemory(
            self.handle as HANDLE,
            addr as *mut c_void,
            buff.as_mut_ptr() as *mut c_void,
            len,
            Some(&mut n)
        )};

        if res.ok().is_err() && self.handle.is_invalid() {
            return Err(ProcessError::ProcessNotFound);
        }

        res.ok()?;

        Ok(())
    }
}
