use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

struct AppState {
    db: ditto_core::db::Db,
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
fn clear_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.db.clear_history().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyV) {
                            if let Some(window) = app.get_webview_window("main") {
                                if let Ok(visible) = window.is_visible() {
                                    if visible {
                                        let _ = window.hide();
                                    } else {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    }
                                }
                            }
                        }
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
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

            app.manage(AppState { db: db.clone() });

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

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
            let _ = app.global_shortcut().register(shortcut);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            select_item,
            clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
