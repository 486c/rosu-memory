use rosu_memory::memory::process::{Process, ProcessTraits};

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
    } else if #[cfg(target_os = "windows")] {
        use windows::{Win32::{Foundation::{
            CloseHandle, FALSE, GetLastError}}, 
            core::PSTR
        };

        use windows::Win32::System::Threading::{ 
            OpenProcess, 
            PROCESS_QUERY_INFORMATION,
            QueryFullProcessImageNameA,
            PROCESS_NAME_FORMAT
        };
    } 
}

#[cfg(target_os = "windows")]
fn get_process_name(id: u32) -> String {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, id)
            .unwrap();

        let mut size = 128;
        let mut buff: Vec<u8> = Vec::with_capacity(128);
        let name = PSTR::from_raw(buff.as_mut_slice().as_mut_ptr());

        QueryFullProcessImageNameA(
            handle,
            PROCESS_NAME_FORMAT(0),
            name,
            &mut size
        );

        CloseHandle(handle);

        let name = name.to_string().unwrap();

        let path = std::path::Path::new(&name);

        path.file_name().unwrap()
            .to_os_string().into_string().unwrap()
    }
}

#[cfg(target_os = "linux")]
fn get_process_name(id: u32) -> String {
    

    std::fs::read_to_string(
        format!("/proc/{}/cmdline", id)
    ).unwrap()
}

#[test]
fn test_process_finder() {
    let proc_id = std::process::id();
    let name = get_process_name(proc_id);

    let proc = Process::find_process(&name).unwrap();
    assert_eq!(proc_id, proc.pid as u32);

    let proc = proc.read_regions().unwrap();
    assert!(!proc.maps.is_empty())
}
