use std::fs;

use crate::memory::process::{ Process, MemoryRegion, ProcessTraits };
use crate::memory::error::ProcessError;

impl ProcessTraits for Process {
    fn initialize(proc_name: &str) -> Result<Process, super::error::ProcessError> {
        let process = Process::find_process(proc_name)?;
        process.read_regions()
    }

    fn find_process(proc_name: &str) -> Result<Process, ProcessError> {
        let mut found: bool = false;

        let paths = fs::read_dir("/proc")?;

        let mut pid: i32 = -1;
        for path in paths {
            
            let p = path?.path();

            if !p.is_dir() {
                continue;
            }

            let cmd_line = p.join("cmdline");

            if !cmd_line.exists() {
                continue;
            }

            let buff = fs::read_to_string(&cmd_line)?;
            let line = buff.split(' ').next().unwrap();

            if line.contains(&proc_name) {
                let stat = p.join("stat");
                let buff = fs::read_to_string(&stat)?;

                let pid_str = buff.split(' ').next().unwrap();
                
                pid = pid_str.parse()?;
                found = true;
                break;
            }
        }

        match found {
            true => Ok(Self { pid, maps: Vec::new() }),
            false => {
                println!("Can't find process!");
                Err(ProcessError::ProcessNotFound)
            }
        }
    }

    fn read_regions(mut self) -> Result<Process, ProcessError> {
        let path = format!("/proc/{}/maps", &self.pid);
        let mut v = Vec::new();
        
        let buff = fs::read_to_string(&path)?;
    
        for line in buff.split('\n')
        {
            if line.len() == 0 {
                break;
            }
    
            let mut split = line.split_whitespace();
            let range_raw = split.next().unwrap();
            let mut range_split = range_raw.split('-');
    
            let from = i64::from_str_radix(
                range_split.next().unwrap(), 16
            )?;

            let to = i64::from_str_radix(
                range_split.next().unwrap(), 16
            )?;
    
            v.push(MemoryRegion{ from, to });
        }
    
        self.maps = v;
        Ok(self)
    }
}
