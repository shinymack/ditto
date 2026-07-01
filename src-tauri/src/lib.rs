use tauri::{Emitter, Manager, WindowEvent};
use std::sync::Mutex;
use std::time::Instant;

struct AppState {
    db: ditto_core::db::Db,
    last_shown: Mutex<Option<Instant>>,
}

#[tauri::command]
fn get_history(
    state: tauri::State<'_, AppState>,
    limit: usize,
    query: Option<String>,
) -> Result<Vec<ditto_core::db::ClipboardItem>, String> {
    state.db.get_history(limit, query.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn select_item(app_handle: tauri::AppHandle, content: String) -> Result<(), String> {
    ditto_core::clipboard::set_text(&content).map_err(|e| e.to_string())?;
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
fn hide_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
fn clear_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.db.clear_history().map_err(|e| e.to_string())
}

fn socket_path() -> std::path::PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("ditto.sock")
}

fn focus_by_pid() {
    let pid = std::process::id();
    if let Ok(output) = std::process::Command::new("wmctrl").args(["-lp"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let win_id = parts[0];
                let win_pid = parts[2];
                if win_pid == pid.to_string() {
                    let _ = std::process::Command::new("wmctrl")
                        .args(["-i", "-a", win_id])
                        .spawn();
                }
            }
        }
    }
}

fn start_socket_server(app_handle: tauri::AppHandle) {
    let sock = socket_path();
    let _ = std::fs::remove_file(&sock);

    use std::os::unix::net::UnixListener;
    let listener = match UnixListener::bind(&sock) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("ditto: failed to bind socket: {}", e);
            return;
        }
    };

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if stream.is_err() {
                continue;
            }
            if let Some(window) = app_handle.get_webview_window("main") {
                let visible = window.is_visible().unwrap_or(false);
                if visible {
                    let _ = window.hide();
                } else {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if let Ok(mut last_shown) = state.last_shown.lock() {
                            *last_shown = Some(Instant::now());
                        }
                    }
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();

                    focus_by_pid();

                    let w = window.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(80));
                        let _ = w.set_focus();
                        focus_by_pid();
                    });
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                let app_handle = window.app_handle();
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Ok(last_shown) = state.last_shown.lock() {
                        if let Some(instant) = *last_shown {
                            if instant.elapsed().as_millis() < 400 {
                                return;
                            }
                        }
                    }
                }
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .expect("failed to get app local data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app local data dir");
            let db_path = app_data_dir.join("ditto.db");
            let db = ditto_core::db::Db::init(db_path.to_str().unwrap()).expect("failed to init db");

            app.manage(AppState { 
                db: db.clone(),
                last_shown: Mutex::new(None),
            });

            let (tx, rx) = std::sync::mpsc::channel();
            ditto_core::clipboard::start_monitor(tx, 500, None);

            let app_handle = app.handle().clone();
            let db_clone = db.clone();
            std::thread::spawn(move || {
                for text in rx {
                    if let Err(e) = db_clone.insert_or_update(&text) {
                        eprintln!("Failed to save clip: {}", e);
                    }
                    let _ = app_handle.emit("clipboard-updated", ());
                }
            });

            // Start Unix socket server for `ditto toggle`
            start_socket_server(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            select_item,
            clear_history,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
