use std::ffi::c_void;

use windows::Win32::System::Memory::MEMORY_BASIC_INFORMATION;
use windows::Win32::System::Memory::VirtualQueryEx;
use windows::Win32::System::Memory::MEM_FREE;
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::ProcessStatus::EnumProcesses;
use windows::Win32::System::ProcessStatus::GetProcessImageFileNameA;
use windows::Win32::Foundation::GetLastError;

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

macro_rules! check_win32_error {
    () => {{
        let result = unsafe { GetLastError() };

        windows::core::Error::from(result)?;
    }}
}

//TODO put unsafe only where it belongs
//or wrap safe helper functions :)
impl ProcessTraits for Process {
    fn initialize(proc_name: &str) -> Result<Process, ProcessError> {
        let process = Process::find_process(proc_name)?;
        process.read_regions()
    }

    fn find_process(proc_name: &str) -> Result<Process, ProcessError> {
        unsafe {
            let mut processes: Vec<u32> = vec![0; 512];
            let mut returned: u32 = 0;

            let res = EnumProcesses(
                processes.as_mut_slice().as_mut_ptr() as _,
                std::mem::size_of::<u32>() as u32 * 512,
                &mut returned
            );

            if let Err(error) = res.ok() {
                return Err(error.into());
            }
            
            let length = returned as usize / std::mem::size_of::<u32>();

            for pid in &processes[0..length] {
                let handle = match OpenProcess(
                    PROCESS_QUERY_INFORMATION|PROCESS_VM_READ, 
                    FALSE, 
                    pid.clone()
                ) {
                    Ok(h) => h,
                    Err(_) => continue,
                };

                let mut string_buff = vec![0u8; 256];

                let size = GetProcessImageFileNameA(
                    handle, 
                    string_buff.as_mut_slice()
                );

                let name = std::str::from_utf8_unchecked(
                    &string_buff[0..size as usize]
                ).to_owned();

                if name.contains(proc_name) {
                    return Ok(Process {
                        pid: pid.clone(),
                        handle,
                        maps: Vec::new()
                    })
                } else {
                    CloseHandle(handle);
                }
            }

            Err(ProcessError::ProcessNotFound)
        }
    }

    fn read_regions(mut self) -> Result<Process, ProcessError> {
        unsafe {
            let mut info = MEMORY_BASIC_INFORMATION::default();
            let mut address: usize = 0;

            while VirtualQueryEx(
                self.handle,
                Some(address as _),
                &mut info,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>()
            ) != 0 {
                address = (info.BaseAddress as usize) + info.RegionSize;


                if info.State != MEM_FREE
                {
                    self.maps.push( MemoryRegion {
                        from: info.BaseAddress as usize,
                        size: info.RegionSize
                    })
                }
            };
        }
        Ok(self)
    }

    fn read_signature(
        &self, 
        sign: &Signature
    ) -> Result<usize, ProcessError> {
        unsafe {
            for chunk in self.maps.chunks(32) {
                let mut buffs: Vec<Vec<u8>> = chunk.iter()
                    .map(|region| vec![0; region.size])
                    .collect();

                // TODO use zip?
                for (index, region) in chunk.iter().enumerate() {
                    let mut bytesread: usize = 0;

                    let res = ReadProcessMemory(
                        self.handle as HANDLE,
                        region.from as *mut c_void,
                        buffs[index].as_mut_ptr() as *mut c_void,
                        region.size,
                        Some(&mut bytesread)
                    );

                    //TODO do the same as L117:linux.rs
                    if let Err(error) = res.ok() {
                        dbg!(error);
                        continue;
                    }

                    let res = find_signature(&buffs[index], sign);
                    if let Some(offset) = res {
                        return Ok(region.from + offset)
                    }
                }
            }

            Err(ProcessError::SignatureNotFound(sign.to_string()))
        }
    }

    fn read(
        &self, 
        addr: usize, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError> {
        // TODO check if buff size is corresponding to len

        let mut n = 0;

        let res = unsafe { ReadProcessMemory(
            self.handle as HANDLE,
            addr as *mut c_void,
            buff.as_mut_ptr() as *mut c_void,
            len,
            Some(&mut n)
        )};

        res.ok()?;

        Ok(())
    }
}
