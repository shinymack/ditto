use std::path::PathBuf;

#[cfg(windows)]
pub fn get_active_window_class() -> Option<String> {
    if let Ok(win) = x_win::get_active_window() {
        if !win.info.name.is_empty() {
            return Some(win.info.name);
        }
    }
    use windows_sys::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }

        let process_handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if process_handle.is_null() {
            return None;
        }

        let mut buffer = [0u16; 512];
        let size = GetModuleFileNameExW(
            process_handle,
            std::ptr::null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        );
        CloseHandle(process_handle);

        if size > 0 {
            let path_str = String::from_utf16_lossy(&buffer[..size as usize]);
            let file_name = PathBuf::from(path_str)
                .file_name()?
                .to_string_lossy()
                .to_string();
            Some(file_name)
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_active_window_class() -> Option<String> {
    None
}

pub fn setup_autostart() {
    let Ok(exe) = std::env::current_exe() else { return };

    if let Some(config_dir) = dirs::config_dir() {
        let startup_dir =
            config_dir.join(r"Microsoft\Windows\Start Menu\Programs\Startup");
        let _ = std::fs::create_dir_all(&startup_dir);
        let bat_path = startup_dir.join("ditto.bat");
        let bat_content = format!("@echo off\r\nstart \"\" \"{}\" run\r\n", exe.display());
        let _ = std::fs::write(bat_path, bat_content);
    }
}
pub fn focus_by_pid() {}
