use tauri::{Emitter, Manager, WindowEvent};
use std::sync::Mutex;
use std::time::Instant;

pub mod ipc;
pub mod platform;

pub struct AppState {
    pub db: ditto_core::db::Db,
    pub last_shown: Mutex<Option<Instant>>,
    pub is_paused: std::sync::atomic::AtomicBool,
    pub config: Mutex<ditto_core::config::Config>,
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
    if let Err(e) = ditto_core::clipboard::set_text(&content) {
        eprintln!("Failed to write to clipboard: {}", e);
        return Err(e.to_string());
    }

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
    if let Err(e) = config.save() {
        return Err(e.to_string());
    }
    let mut current = state.config.lock().unwrap();
    *current = config;
    Ok(())
}

#[tauri::command]
fn is_paused(state: tauri::State<'_, AppState>) -> bool {
    state.is_paused.load(std::sync::atomic::Ordering::SeqCst)
}

#[tauri::command]
fn set_paused(state: tauri::State<'_, AppState>, app_handle: tauri::AppHandle, paused: bool) {
    state.is_paused.store(paused, std::sync::atomic::Ordering::SeqCst);
    let _ = app_handle.emit("pause-status-changed", paused);
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
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_visible_on_all_workspaces(true);
                let _ = window.show();
                let _ = window.set_focus();
            }
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
            if let Some(state) = app.try_state::<AppState>() {
                if let Ok(mut last_shown) = state.last_shown.lock() {
                    *last_shown = Some(Instant::now());
                }
            }

            let (tx, rx) = std::sync::mpsc::channel();
            ditto_core::clipboard::start_monitor(tx, None);

            let app_handle = app.handle().clone();
            let db_clone = db.clone();
            let app_handle_state = app.handle().clone();
            std::thread::spawn(move || {
                for text in rx {
                    let active_config = if let Some(state) = app_handle_state.try_state::<AppState>() {
                        if state.is_paused.load(std::sync::atomic::Ordering::SeqCst) {
                            continue;
                        }
                        state.config.lock().unwrap().clone()
                    } else {
                        ditto_core::config::Config::load()
                    };

                    let mut ignore = false;
                    if let Some(active_window) = platform::get_active_window_class() {
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

            ipc::start_ipc_server(app.handle().clone());

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
