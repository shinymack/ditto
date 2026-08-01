use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

const PIPE_NAME: &str = r"\\.\pipe\ditto-ipc-socket";

pub fn socket_path() -> PathBuf {
    PathBuf::from(PIPE_NAME)
}

pub fn is_daemon_running() -> bool {
    OpenOptions::new().read(true).write(true).open(PIPE_NAME).is_ok()
}

pub fn send_command(cmd: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE_NAME)
        .map_err(|e| format!("Failed to connect to Ditto daemon pipe: {}", e))?;
    file.write_all(cmd.as_bytes()).map_err(|e| e.to_string())
}

#[cfg(windows)]
pub fn start_ipc_server(app_handle: tauri::AppHandle) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        ReadFile, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
        PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
    };

    std::thread::spawn(move || {
        let name: Vec<u16> = std::ffi::OsStr::new(PIPE_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        loop {
            let handle = unsafe {
                CreateNamedPipeW(
                    name.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    1,
                    512,
                    512,
                    0,
                    std::ptr::null(),
                )
            };

            if handle == INVALID_HANDLE_VALUE {
                std::thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }

            let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) };
            if connected != 0 || unsafe { windows_sys::Win32::Foundation::GetLastError() } == 535 {
                let mut buf = [0u8; 128];
                let mut read_bytes: u32 = 0;
                let success = unsafe {
                    ReadFile(
                        handle,
                        buf.as_mut_ptr() as _,
                        buf.len() as u32,
                        &mut read_bytes,
                        std::ptr::null_mut(),
                    )
                };

                if success != 0 && read_bytes > 0 {
                    let msg = String::from_utf8_lossy(&buf[..read_bytes as usize])
                        .trim()
                        .to_string();
                    crate::ipc::handle_ipc_command(&app_handle, &msg);
                }

                unsafe {
                    DisconnectNamedPipe(handle);
                }
            }

            unsafe {
                CloseHandle(handle);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn start_ipc_server(_app_handle: tauri::AppHandle) {}
