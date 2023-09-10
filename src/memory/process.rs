use super::error::ProcessError;
use super::signature::Signature;

#[derive(Debug)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize
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
    fn initialize(proc_name: &str) -> Result<Self, ProcessError>;
    fn find_process(proc_name: &str) -> Result<Self, ProcessError>;
    fn read_regions(self) -> Result<Self, ProcessError>;

    fn read_signature(
        &self, 
        sign: &Signature
    ) -> Result<Option<usize>, ProcessError>;

    fn read(
        self, 
        addr: usize, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError>;

    fn read_i32(
        self,
        addr: usize,
    ) -> Result<i32, ProcessError> {
        let mut bytes = [0u8; 4];
        self.read(addr, bytes.len(), &mut bytes)?;

        Ok(i32::from_le_bytes(bytes))
    }
}
