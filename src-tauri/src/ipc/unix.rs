use crate::ipc::handle_ipc_command;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

pub fn socket_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("ditto.sock")
}

pub fn send_command(cmd: &str) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
    stream.write_all(cmd.as_bytes()).map_err(|e| e.to_string())
}

pub fn is_daemon_running() -> bool {
    UnixStream::connect(socket_path()).is_ok()
}

pub fn start_ipc_server(app_handle: tauri::AppHandle) {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("ditto: failed to bind socket at {}: {}", path.display(), e);
            return;
        }
    };

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buf = [0u8; 128];
                if let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        continue;
                    }
                    let msg = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    handle_ipc_command(&app_handle, &msg);
                }
            }
        }
    });
}

