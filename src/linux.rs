use nix::sys::uio::{RemoteIoVec, process_vm_readv};
use nix::unistd::Pid;

use std::io::IoSliceMut;
use std::fs;
use std::str::FromStr;

use crate::memory::{MemoryRegion, MemAddress, MemoryErrors};
use crate::memory::{Signature, find_signature};

#[derive(Debug)]
pub enum ProcessErrors {
    ProcessNotFound,
    ReadFailed,
}

#[derive(Debug)]
pub struct Process {
    pub (crate) pid: i32,
    pub (crate) maps: Vec<MemoryRegion>,
}

pub (crate) trait FindProc {
    fn find_proc(s: &str) -> Result<Self, ProcessErrors> where Self: Sized;
}

pub (crate) trait ProcessTraits {
    fn read_maps(&mut self);
    fn read_at(&self, addr: &i32, size: usize) -> Result<Vec<u8>, ProcessErrors>;
    fn find_signature(&self, s: &str) -> Result<MemAddress, MemoryErrors>;
}

impl FindProc for Process {
    fn find_proc(s: &str) -> Result<Self, ProcessErrors> {

        let mut found: bool = false;

        let paths = fs::read_dir("/proc")
            .expect("Failed to read /proc/ dir.");

        let mut pid: i32 = -1;
        for path in paths {
            
            let p = path.unwrap().path();

            if !p.is_dir() {
                continue;
            }

            let cmd_line = p.join("cmdline");

            if !cmd_line.exists() {
                continue;
            }

            let buff = fs::read_to_string(&cmd_line).unwrap();
            let line = buff.split(' ').next().unwrap();

            if line.contains(&s) {
                let stat = p.join("stat");
                let buff = fs::read_to_string(&stat).unwrap();

                let pid_str = buff.split(' ').next().unwrap();
                
                pid = i32::from_str(pid_str).unwrap();
                found = true;
                break;
            }
        }

        match found {
            true => Ok(Self { pid, maps: Vec::new() }),
            false => {
                println!("Can't find process!");
                Err(ProcessErrors::ProcessNotFound)
            }
        }

    }
}

impl ProcessTraits for Process {
    fn read_maps(&mut self) {
        let path = format!("/proc/{}/maps", &self.pid);
    
        let mut v = Vec::new();
        
        //TODO remove expect
        let buff = fs::read_to_string(&path)
            .expect("Can't read maps!");
    
        for line in buff.split('\n')
        {
            if line.len() == 0 {
                break;
            }
    
            let mut split = line.split_whitespace();
            let range_raw = split.next().unwrap();
            let mut range_split = range_raw.split('-');
    
            let from = i64::from_str_radix(range_split.next().unwrap(), 16)
                .unwrap();
            let to = i64::from_str_radix(range_split.next().unwrap(), 16)
                .unwrap();
    
            v.push(MemoryRegion{ from, to });
        }
    
        self.maps = v;
    }

    fn read_at(
        &self,
        addr: &i32,
        size: usize
    ) -> Result<Vec<u8>, ProcessErrors> {
        let mut buf = vec![0u8; size];
    
        let remote_iov = RemoteIoVec{
            base: (*addr) as usize,
            len: size,
        };
    
        let ret = process_vm_readv(
            Pid::from_raw(self.pid), 
            &mut[IoSliceMut::new(&mut buf)],
            &[remote_iov]
        );

        match ret {
            Ok(_) => Ok(buf),
            Err(_) => Err(ProcessErrors::ReadFailed),
        }
    }

    fn find_signature(&self, s: &str) -> Result<MemAddress, MemoryErrors> {
        let sig = Signature::from_str(s)?;
        let mut buff: Vec<u8>;
        let mut found = false;

        let mut offset: i32 = 0;

        for map in &self.maps {
            let ret = self.read_at(&(map.from as i32), (map.to-map.from) as usize);

            match ret {
                Ok(buf) => buff = buf,
                Err(_) => continue,
            };

            let ret = find_signature(&buff, &sig);
            match ret {
                Ok(off) => {
                    offset = off + map.from as i32;
                    found = true;
                    break;
                },
                Err(_) => continue,
            }
        }
        
        match found {
            true => Ok(MemAddress{ offset, process: &self }),
            false => Err(MemoryErrors::PatternNotFound),
        }
    }

}

#[cfg(test)]
mod tests {
    use crate::linux::Process;
    use crate::linux::FindProc;
    use std::process;
    
    #[test]
    fn find_proc() {
        let p = Process::find_proc("memory_tests").unwrap();

        assert_eq!(p.pid as u32, process::id());
    }
}
