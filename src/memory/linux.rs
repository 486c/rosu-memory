use std::fs;
use std::io::IoSliceMut;
use std::path::PathBuf;

use nix::errno::Errno;
use nix::sys::uio::{RemoteIoVec, process_vm_readv};
use nix::unistd::Pid;

use crate::memory::process::{ Process, MemoryRegion, ProcessTraits };
use crate::memory::error::ProcessError;

use super::signature::find_signature;
use super::signature::Signature;

impl ProcessTraits for Process {
    fn initialize(
        proc_name: &str
    ) -> Result<Process, super::error::ProcessError> {
        let process = Process::find_process(proc_name)?;
        process.read_regions()
    }

    fn find_process(proc_name: &str) -> Result<Process, ProcessError> {
        let paths = fs::read_dir("/proc")?;

        for path in paths {
            
            let p = path?.path();

            if !p.is_dir() {
                continue;
            }
            
            let cmd_line = p.join("cmdline");

            if !cmd_line.exists() {
                continue;
            }

            let mut cmd_buff = fs::read_to_string(cmd_line)?;

            cmd_buff.retain(|c| c != '\0');
            cmd_buff = cmd_buff.replace('\\', "/");

            dbg!(&cmd_buff);

            let line = cmd_buff.split(' ').next().unwrap();

            if line.contains(proc_name) {
                let stat = p.join("stat");
                let buff = fs::read_to_string(stat)?;

                // Formatting path
                cmd_buff.remove(0);
                cmd_buff.remove(0);

                let executable_path = PathBuf::from(cmd_buff);
                let executable_dir = executable_path.parent()
                    .map(|v| v.to_path_buf());

                let pid_str = buff.split(' ').next().unwrap();
                
                let pid = pid_str.parse()?;

                return Ok(Self { 
                    pid, 
                    maps: Vec::new(), 
                    executable_dir
                });
            }
        }

        Err(ProcessError::ProcessNotFound)
    }

    fn read_regions(mut self) -> Result<Process, ProcessError> {
        let path = format!("/proc/{}/maps", &self.pid);
        let mut v = Vec::new();
        
        let buff = fs::read_to_string(&path)?;
    
        for line in buff.split('\n')
        {
            if line.is_empty() {
                break;
            }
    
            let mut split = line.split_whitespace();
            let range_raw = split.next().unwrap();
            let mut range_split = range_raw.split('-');

            let from_str = range_split.next().unwrap();
            let to_str = range_split.next().unwrap();

            let from = usize::from_str_radix(
                from_str, 16
            )?;

            let to = usize::from_str_radix(
                to_str, 16
            )?;
    
            v.push(MemoryRegion{ from, size: to - from });
        }
    
        self.maps = v;
        Ok(self)
    }

    fn read_signature(
        &self, 
        sign: &Signature
    ) -> Result<usize, ProcessError> {
        let mut buff = Vec::new();

        for region in &self.maps {
            let remote = RemoteIoVec {
                base: region.from,
                len: region.size
            };

            buff.resize(region.size, 0);

            let slice = IoSliceMut::new(buff.as_mut_slice());

            let res = process_vm_readv(
                Pid::from_raw(self.pid),
                &mut [slice],
                &[remote]
            );
            
            if let Err(e) = res {
                match e {
                    Errno::EPERM | Errno::ESRCH =>
                        return Err(e.into()),
                    _ => continue,
                }
            }

            if let Some(offset) = find_signature(buff.as_slice(), sign) {
                return Ok(remote.base + offset);
            }
        }

        Err(ProcessError::SignatureNotFound(sign.to_string()))
    }

    fn read(
        &self, 
        addr: usize, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError> {
        let remote = RemoteIoVec {
            base: addr,
            len
        };

        let slice = IoSliceMut::new(buff);

        let res = process_vm_readv(
            Pid::from_raw(self.pid),
            &mut [slice],
            &[remote]
        );

        match res {
            Ok(_) => (),
            Err(e) => {
                match e {
                    nix::errno::Errno::EFAULT => 
                        return Err(ProcessError::BadAddress(addr, len)),
                    _ => return Err(e.into()),
                }
            },
        }

        Ok(())
    }
}
