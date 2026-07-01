// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;
use std::os::unix::net::UnixStream;

const ICON_BYTES: &[u8] = include_bytes!("../icons/icon.png");

fn runtime_dir() -> std::path::PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
}

fn socket_path() -> std::path::PathBuf {
    runtime_dir().join("ditto.sock")
}

fn setup_autostart() {
    let Ok(exe) = std::env::current_exe() else { return };
    
    // Install the embedded icon to standard user-local icon theme directory
    let mut icon_path = std::path::PathBuf::from("/tmp/ditto.png");
    if let Some(data_dir) = dirs::data_dir() {
        let icon_dir = data_dir.join("icons/hicolor/512x512/apps");
        let _ = std::fs::create_dir_all(&icon_dir);
        let icon_file = icon_dir.join("ditto.png");
        let _ = std::fs::write(&icon_file, ICON_BYTES);
        icon_path = icon_file;
    }

    // Use absolute path of the installed local icon to bypass icon theme caching on X11
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Ditto\nExec={}\nHidden=false\nX-GNOME-Autostart-enabled=true\nComment=Keyboard-driven clipboard manager\nIcon={}\nTerminal=false\nCategories=Utility;\nStartupWMClass=ditto\n",
        exe.display(),
        icon_path.display()
    );

    // 1. Write to autostart
    if let Some(autostart_dir) = dirs::config_dir().map(|d| d.join("autostart")) {
        let _ = std::fs::create_dir_all(&autostart_dir);
        let _ = std::fs::write(autostart_dir.join("ditto.desktop"), &content);
    }

    // 2. Write to applications directory for window manager WM_CLASS resolution
    if let Some(apps_dir) = dirs::data_dir().map(|d| d.join("applications")) {
        let _ = std::fs::create_dir_all(&apps_dir);
        let _ = std::fs::write(apps_dir.join("ditto.desktop"), &content);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("toggle") {
        if let Ok(mut stream) = UnixStream::connect(socket_path()) {
            let _ = stream.write_all(b"toggle");
        } else {
            // Daemon not running, spin it up!
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                // Wait for daemon to initialize and bind socket
                std::thread::sleep(std::time::Duration::from_millis(350));
                if let Ok(mut stream) = UnixStream::connect(socket_path()) {
                    let _ = stream.write_all(b"toggle");
                }
            }
        }
        return;
    }

    // Start of the daemon
    setup_autostart();
    ditto_lib::run();
}
