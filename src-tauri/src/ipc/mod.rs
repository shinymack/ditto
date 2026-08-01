use tauri::{Emitter, Manager};

#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::*;

pub fn handle_ipc_command(app_handle: &tauri::AppHandle, msg: &str) {
    match msg {
        "toggle" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                let visible = window.is_visible().unwrap_or(false);
                if visible {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        }
        "clear" => {
            if let Some(state) = app_handle.try_state::<crate::AppState>() {
                let _ = state.db.clear_history();
                let _ = app_handle.emit("history-updated", ());
            }
        }
        "pause" => {
            if let Some(state) = app_handle.try_state::<crate::AppState>() {
                state.is_paused.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = app_handle.emit("pause-status-changed", true);
            }
        }
        "resume" => {
            if let Some(state) = app_handle.try_state::<crate::AppState>() {
                state.is_paused.store(false, std::sync::atomic::Ordering::SeqCst);
                let _ = app_handle.emit("pause-status-changed", false);
            }
        }
        "stop" => {
            let path = socket_path();
            let _ = std::fs::remove_file(&path);
            std::process::exit(0);
        }
        _ => {}
    }
}
