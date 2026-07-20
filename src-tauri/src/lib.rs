use tauri::{Emitter, Manager, WindowEvent};
use std::sync::Mutex;
use std::time::Instant;

struct AppState {
    db: ditto_core::db::Db,
    last_shown: Mutex<Option<Instant>>,
    is_paused: std::sync::atomic::AtomicBool,
    config: Mutex<ditto_core::config::Config>,
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

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Result<ditto_core::config::Config, String> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
fn save_config(state: tauri::State<'_, AppState>, config: ditto_core::config::Config) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
fn is_paused(state: tauri::State<'_, AppState>) -> bool {
    state.is_paused.load(std::sync::atomic::Ordering::SeqCst)
}

#[tauri::command]
fn set_paused(state: tauri::State<'_, AppState>, app_handle: tauri::AppHandle, paused: bool) {
    state.is_paused.store(paused, std::sync::atomic::Ordering::SeqCst);
    let _ = app_handle.emit("pause-updated", paused);
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

fn get_active_window_class() -> Option<String> {
    // 1. Get the active window ID using xprop -root _NET_ACTIVE_WINDOW
    let output = std::process::Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse output like: _NET_ACTIVE_WINDOW(WINDOW): window id # 0x3800458
    let win_id = stdout
        .split('#')
        .last()?
        .trim();
    
    if win_id.is_empty() || win_id == "0x0" {
        return None;
    }
    
    // 2. Get the WM_CLASS for this window ID
    let class_output = std::process::Command::new("xprop")
        .args(["-id", win_id, "WM_CLASS"])
        .output()
        .ok()?;
        
    if !class_output.status.success() {
        return None;
    }
    
    let class_stdout = String::from_utf8_lossy(&class_output.stdout);
    Some(class_stdout.to_string())
}

fn start_socket_server(app_handle: tauri::AppHandle) {
    let sock = socket_path();
    let _ = std::fs::remove_file(&sock);

    use std::os::unix::net::UnixListener;
    use std::io::Read;

    let listener = match UnixListener::bind(&sock) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("ditto: failed to bind socket: {}", e);
            return;
        }
    };

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };

            let mut buf = [0u8; 32];
            let n = match stream.read(&mut buf) {
                Ok(n) => n,
                Err(_) => continue,
            };

            let msg = String::from_utf8_lossy(&buf[..n]);
            let cmd = msg.trim();

            if cmd == "toggle" {
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
            } else if cmd == "clear" {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    let _ = state.db.clear_history();
                    let _ = app_handle.emit("clipboard-updated", ());
                }
            } else if cmd == "pause" {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.is_paused.store(true, std::sync::atomic::Ordering::SeqCst);
                    let _ = app_handle.emit("pause-updated", true);
                }
            } else if cmd == "resume" {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.is_paused.store(false, std::sync::atomic::Ordering::SeqCst);
                    let _ = app_handle.emit("pause-updated", false);
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
            if window.label() == "main" {
                if let WindowEvent::Focused(false) = event {
                    let app_handle = window.app_handle();
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if state.config.lock().unwrap().persistent_window {
                            return;
                        }
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

            let config = ditto_core::config::Config::load();
            app.manage(AppState { 
                db: db.clone(),
                last_shown: Mutex::new(None),
                is_paused: std::sync::atomic::AtomicBool::new(false),
                config: Mutex::new(config),
            });

            let (tx, rx) = std::sync::mpsc::channel();
            ditto_core::clipboard::start_monitor(tx, None);

            let app_handle = app.handle().clone();
            let db_clone = db.clone();
            let app_handle_state = app.handle().clone();
            std::thread::spawn(move || {
                for text in rx {
                    // Check if clipboard monitoring is paused and get config
                    let active_config = if let Some(state) = app_handle_state.try_state::<AppState>() {
                        if state.is_paused.load(std::sync::atomic::Ordering::SeqCst) {
                            continue;
                        }
                        state.config.lock().unwrap().clone()
                    } else {
                        ditto_core::config::Config::load()
                    };

                    // Check if the currently active window matches any ignored application keyword
                    let mut ignore = false;
                    if let Some(active_window) = get_active_window_class() {
                        let active_window_lower = active_window.to_lowercase();
                        for app_name in &active_config.ignored_apps {
                            if active_window_lower.contains(&app_name.to_lowercase()) {
                                ignore = true;
                                break;
                            }
                        }
                    }

                    if !ignore {
                        if let Err(e) = db_clone.insert_or_update(&text, active_config.max_items) {
                            eprintln!("Failed to save clip: {}", e);
                        }
                        let _ = app_handle.emit("clipboard-updated", ());
                    }
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
            hide_window,
            get_config,
            save_config,
            is_paused,
            set_paused
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
