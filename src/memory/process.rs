use crate::memory::error::ProcessError;

#[derive(Debug)]
pub struct MemoryRegion {
    pub from: i64,
    pub to: i64,
}

#[derive(Debug)]
pub struct Process {
    pub pid: i32,
    pub maps: Vec<MemoryRegion>,
}

pub trait ProcessTraits {
    // Find process by name & read all memory regions
    fn initialize(proc_name: &str) -> Result<Process, ProcessError>;
    fn find_process(proc_name: &str) -> Result<Process, ProcessError>;
    fn read_regions(self) -> Result<Process, ProcessError>;
    //fn read_maps(&mut self);
    //fn read_at(&self, addr: &i32, size: usize) -> Result<Vec<u8>, ProcessErrors>;
    //fn find_signature(&self, s: &str) -> Result<MemAddress, MemoryErrors>;
}
