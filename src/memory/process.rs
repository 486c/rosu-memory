
use super::error::ProcessError;
use super::signature::Signature;

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct MemoryRegion {
    pub from: usize,
    pub to: usize,
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize
}

#[cfg(target_os = "linux")]
impl MemoryRegion {
    pub fn size(&self) -> usize {
        self.to - self.from
    }
}

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct Process {
    pub pid: u32, // TODO Use u32? or even usize
    pub handle: HANDLE,
    pub maps: Vec<MemoryRegion>,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct Process {
    pub pid: i32, // TODO Use u32? or even usize
    pub maps: Vec<MemoryRegion>,
}

pub trait ProcessTraits where Self: Sized {
    // Find process by name & read all memory regions
    fn initialize(proc_name: &str) -> Result<Self, ProcessError>;
    fn find_process(proc_name: &str) -> Result<Self, ProcessError>;
    fn read_regions(self) -> Result<Self, ProcessError>;
    fn read_signature(&self, sign: &Signature) -> Result<Option<usize>, ProcessError>;
    //fn read_maps(&mut self);
    //fn read_at(&self, addr: &i32, size: usize) -> Result<Vec<u8>, ProcessErrors>;
    //fn find_signature(&self, s: &str) -> Result<MemAddress, MemoryErrors>;
}
