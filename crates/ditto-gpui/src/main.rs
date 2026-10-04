pub mod ipc;
pub mod platform;
pub mod search;
pub mod state;
pub mod ui;

use ditto_core::config::Config;
use ditto_core::db::Db;
use gpui_kit::gpui::{
    point, px, size, App, Bounds, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowHandle, WindowKind, WindowOptions,
};
use gpui_kit::*;
use parking_lot::Mutex;
use state::AppState;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use ui::DittoOverlayView;

fn print_help() {
    let art = "\x1b[1;33m  ____  _ _   _\n |  _ \\(_) |_| |_ ___\n | | | | | __| __/ _ \\\n | |_| | | |_| || (_) |\n |____/|_|\\__|\\__\\___/\x1b[0m";
    println!("{}", art);
    println!(
        "  \x1b[1;37mDitto\x1b[0m \x1b[90mv{}\x1b[0m \x1b[90m-\x1b[0m A lightweight, keyboard-driven clipboard manager (Native GPUI Kit).",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("  \x1b[1;33mUSAGE:\x1b[0m");
    println!("    ditto [COMMAND]");
    println!();
    println!("  \x1b[1;33mCOMMANDS:\x1b[0m");
    println!(
        "    \x1b[1;37mstart\x1b[0m       Start the Ditto daemon in the background (detached)"
    );
    println!("    \x1b[1;37mstop\x1b[0m        Stop the running Ditto daemon");
    println!("    \x1b[1;37mrun\x1b[0m         Run the Ditto daemon in the foreground");
    println!("    \x1b[1;37mtoggle\x1b[0m      Toggle window visibility (shows if hidden, hides if visible)");
    println!("    \x1b[1;37mclear\x1b[0m       Clear all clipboard history");
    println!("    \x1b[1;37mpause\x1b[0m       Pause clipboard monitoring");
    println!("    \x1b[1;37mresume\x1b[0m      Resume clipboard monitoring");
    println!("    \x1b[1;37mlist\x1b[0m        List last 50 clipboard items");
    println!("    \x1b[1;37mstatus\x1b[0m      Show daemon process state, database size, and configuration");
    println!("    \x1b[1;37m-v, --version\x1b[0m Show version information");
    println!("    \x1b[1;37m-h, --help\x1b[0m  Show this help message");
    println!("  \x1b[1;33mCONFIG FILE:\x1b[0m");
    println!("    {}", Config::file_path().display());
}

fn get_db_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("ditto");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("ditto.db")
}

fn open_overlay_window(
    cx: &mut App,
    state: AppState,
    window_slot: Arc<Mutex<Option<WindowHandle<DittoOverlayView>>>>,
) {
    let width = px(800.0);
    let height = px(500.0);

    let window_bounds = {
        let cfg = state.config.read();
        if let (Some(x), Some(y)) = (cfg.window_x, cfg.window_y) {
            if x > 10.0 && y > 10.0 {
                WindowBounds::Windowed(Bounds::new(point(px(x), px(y)), size(width, height)))
            } else if let Some(display) = cx.primary_display() {
                let db = display.bounds();
                let x = db.origin.x + (db.size.width - width) / 2.0;
                let y = db.origin.y + (db.size.height - height) / 3.0;
                WindowBounds::Windowed(Bounds::new(point(x, y), size(width, height)))
            } else {
                WindowBounds::Windowed(Bounds::centered(None, size(width, height), cx))
            }
        } else if let Some(display) = cx.primary_display() {
            let db = display.bounds();
            let x = db.origin.x + (db.size.width - width) / 2.0;
            let y = db.origin.y + (db.size.height - height) / 3.0;
            WindowBounds::Windowed(Bounds::new(point(x, y), size(width, height)))
        } else {
            WindowBounds::Windowed(Bounds::centered(None, size(width, height), cx))
        }
    };

    let options = WindowOptions {
        window_bounds: Some(window_bounds),
        titlebar: Some(gpui_kit::gpui::TitlebarOptions {
            title: Some("Ditto".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        window_decorations: Some(WindowDecorations::Client),
        kind: WindowKind::Normal,
        window_background: WindowBackgroundAppearance::Transparent,
        focus: true,
        show: true,
        is_resizable: false,
        is_movable: true,
        app_id: Some("ditto".into()),
        ..Default::default()
    };
    let (saved_x, saved_y) = {
        let cfg = state.config.read();
        (cfg.window_x, cfg.window_y)
    };
    let state_clone = state.clone();
    let slot_clone = window_slot.clone();

    match cx.open_window(options, move |window, cx| {
        let view = cx.new(|cx| DittoOverlayView::new(state_clone, window, cx));
        window.refresh();
        view
    }) {
        Ok(handle) => {
            *slot_clone.lock() = Some(handle.clone());
            let _ = handle.update(cx, |_, window, cx| {
                window.refresh();
                cx.notify();
            });
            std::thread::spawn(move || {
                platform::init_window_hints(saved_x, saved_y);
                platform::focus_by_pid();
            });
        }
        Err(e) => {
            eprintln!("ditto: failed to open window: {:?}", e);
        }
    }
}

fn run_daemon() {
    if ipc::is_daemon_running() {
        let _ = ipc::send_command("toggle");
        return;
    }

    platform::setup_autostart();

    let db_path = get_db_path();
    let db = Db::init(db_path.to_str().unwrap()).expect("Failed to initialize database");
    let config = Config::load();
    let state = AppState::new(db, config);

    // 1. Background Clipboard Watcher Thread
    let (tx, rx) = std::sync::mpsc::channel();
    ditto_core::clipboard::start_monitor(tx, None);

    let state_clip = state.clone();
    std::thread::spawn(move || {
        for text in rx {
            if state_clip.is_paused() {
                continue;
            }

            let ignored_apps = state_clip.config.read().ignored_apps.clone();
            let mut ignore = false;
            if let Some(active_window) = platform::get_active_window_class() {
                let active_window_lower = active_window.to_lowercase();
                for app_name in &ignored_apps {
                    if active_window_lower.contains(&app_name.to_lowercase()) {
                        ignore = true;
                        break;
                    }
                }
            }

            if !ignore {
                let _ = state_clip.insert_clip(&text);
            }
        }
    });

    // 2. Window Slot & Visibility State
    let window_slot: Arc<Mutex<Option<WindowHandle<DittoOverlayView>>>> =
        Arc::new(Mutex::new(None));
    let is_visible: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let slot_clone = window_slot.clone();
    let is_vis_clone = is_visible.clone();
    let state_clone = state.clone();

    // 3. Thread-Safe IPC Channel
    let (ipc_tx, ipc_rx) = smol::channel::unbounded::<String>();

    ipc::start_ipc_server(move |msg| {
        if msg == "stop" {
            let path = ipc::socket_path();
            let _ = std::fs::remove_file(&path);
            std::process::exit(0);
        }
        let _ = ipc_tx.send_blocking(msg.to_string());
    });

    // 4. Launch GPUI Application with Explicit QuitMode (Daemon Persistence)
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
        gpui_kit::component::Theme::set_scrollbar_mode(gpui_kit::base::ScrollbarMode::Always, cx);
        cx.set_quit_mode(gpui_kit::gpui::QuitMode::Explicit);
        // Track window close globally (non-Linux only — on Linux we unmap instead)
        let slot_for_closed = slot_clone.clone();
        let is_vis_for_closed = is_vis_clone.clone();
        cx.on_window_closed(move |_, _| {
            *slot_for_closed.lock() = None;
            is_vis_for_closed.store(false, Ordering::SeqCst);
        })
        .detach();

        // Process incoming IPC commands on main thread
        let slot = slot_clone.clone();
        let state = state_clone.clone();
        let is_vis = is_vis_clone.clone();

        cx.spawn(async move |cx| {
            while let Ok(msg) = ipc_rx.recv().await {
                let slot = slot.clone();
                let state = state.clone();
                let is_vis = is_vis.clone();
                cx.update(move |cx| match msg.as_str() {
                    "toggle" => {
                        let lock = slot.lock();
                        #[cfg(target_os = "linux")]
                        let currently_visible = platform::is_window_mapped();
                        #[cfg(not(target_os = "linux"))]
                        let currently_visible = is_vis.load(Ordering::SeqCst);

                        if currently_visible {
                            // Window is visible — hide it
                            if let Some(h) = lock.as_ref() {
                                let _ = h.update(cx, |view, window, cx| {
                                    view.hide_overlay(window, cx);
                                });
                            }
                            is_vis.store(false, Ordering::SeqCst);
                        } else if lock.is_some() {
                            // Window handle exists but is hidden (unmapped) — show it
                            drop(lock);
                            let cfg = state.config.read();
                            let x = cfg.window_x.unwrap_or(400.0) as i32;
                            let y = cfg.window_y.unwrap_or(200.0) as i32;
                            drop(cfg);

                            #[cfg(target_os = "linux")]
                            platform::show_window(x, y);

                            let lock = slot.lock();
                            if let Some(h) = lock.as_ref() {
                                let _ = h.update(cx, |view, window, cx| {
                                    view.on_show(window, cx);
                                    window.activate_window();
                                    window.refresh();
                                    cx.notify();
                                });
                            }
                            is_vis.store(true, Ordering::SeqCst);
                            platform::focus_by_pid();

                            // Retry focus after 80ms (mirrors Tauri behavior)
                            std::thread::spawn(|| {
                                std::thread::sleep(std::time::Duration::from_millis(80));
                                platform::focus_by_pid();
                            });
                        } else {
                            // No window handle at all — create fresh window
                            drop(lock);
                            open_overlay_window(cx, state, slot.clone());
                            is_vis.store(true, Ordering::SeqCst);
                        }
                    }
                    "clear" => {
                        let _ = state.clear_history();
                    }
                    "pause" => {
                        state.set_paused(true);
                    }
                    "resume" => {
                        state.set_paused(false);
                    }
                    _ => {}
                });
            }
        })
        .detach();
    });
}

fn spawn_daemon(exe: &std::path::Path) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("run")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn()?;
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next();

    match cmd.as_deref() {
        None | Some("-h" | "--help") => {
            print_help();
        }
        Some("-v" | "--version" | "version") => {
            println!("ditto v{}", env!("CARGO_PKG_VERSION"));
        }
        Some("toggle") => {
            if ipc::is_daemon_running() {
                if let Err(e) = ipc::send_command("toggle") {
                    eprintln!("ditto: failed to toggle: {}", e);
                }
            } else {
                let Ok(exe) = std::env::current_exe() else {
                    eprintln!("ditto: could not determine binary path");
                    return;
                };

                let _ = spawn_daemon(&exe);

                for _ in 0..30 {
                    std::thread::sleep(Duration::from_millis(50));
                    if ipc::is_daemon_running() {
                        let _ = ipc::send_command("toggle");
                        return;
                    }
                }
            }
        }
        Some("start") => {
            if ipc::is_daemon_running() {
                println!("ditto: daemon is already running");
                return;
            }
            let Ok(exe) = std::env::current_exe() else {
                eprintln!("ditto: could not determine binary path");
                return;
            };

            match spawn_daemon(&exe) {
                Ok(_) => {
                    println!("ditto: daemon started in background");
                }
                Err(e) => {
                    eprintln!("ditto: failed to spawn daemon: {}", e);
                }
            }
        }
        Some("stop") => {
            if !ipc::is_daemon_running() {
                println!("ditto: daemon is not running");
                return;
            }
            if let Err(e) = ipc::send_command("stop") {
                eprintln!("ditto: failed to stop daemon: {}", e);
            } else {
                println!("ditto: daemon stopped");
            }
        }
        Some("run") => {
            run_daemon();
        }
        Some("clear") => {
            if ipc::is_daemon_running() {
                let _ = ipc::send_command("clear");
            } else {
                let db_path = get_db_path();
                if let Ok(db) = Db::init(db_path.to_str().unwrap()) {
                    let _ = db.clear_history();
                }
            }
            println!("ditto: clipboard history cleared");
        }
        Some("pause") => {
            if ipc::is_daemon_running() {
                let _ = ipc::send_command("pause");
                println!("ditto: clipboard monitoring paused");
            } else {
                eprintln!("ditto: daemon is not running");
            }
        }
        Some("resume") => {
            if ipc::is_daemon_running() {
                let _ = ipc::send_command("resume");
                println!("ditto: clipboard monitoring resumed");
            } else {
                eprintln!("ditto: daemon is not running");
            }
        }
        Some("list") => {
            let db_path = get_db_path();
            let db = Db::init(db_path.to_str().unwrap()).expect("Failed to initialize database");
            match db.get_history(50, None) {
                Ok(items) => {
                    println!("\x1b[1;37mShowing last {} items:\x1b[0m\n", items.len());
                    for (i, item) in items.iter().enumerate() {
                        let is_img = item.content.starts_with("data:image/png;base64,");
                        let preview = if is_img {
                            "\x1b[36m[Image Clip]\x1b[0m"
                        } else {
                            item.content.trim()
                        };
                        println!(
                            "\x1b[33m{:>2}.\x1b[0m \x1b[90m[{}]\x1b[0m {}",
                            i + 1,
                            item.created_at,
                            preview
                        );
                    }
                }
                Err(e) => eprintln!("Failed to read history: {}", e),
            }
        }
        Some("status") => {
            let running = ipc::is_daemon_running();
            println!("\x1b[1;37mDitto Status:\x1b[0m");
            println!(
                "  Daemon:       {}",
                if running {
                    "\x1b[32m● Running\x1b[0m"
                } else {
                    "\x1b[31m○ Stopped\x1b[0m"
                }
            );
            let db_path = get_db_path();
            if let Ok(meta) = std::fs::metadata(&db_path) {
                println!(
                    "  Database Size: {:.2} MB",
                    meta.len() as f64 / 1024.0 / 1024.0
                );
            }
            if let Ok(db) = Db::init(db_path.to_str().unwrap()) {
                if let Ok(count) = db.count_items() {
                    println!("  Total Items:   {}", count);
                }
            }
            println!("  Config Path:   {}", Config::file_path().display());
        }
        Some(unknown) => {
            eprintln!("ditto: unknown command '{}'", unknown);
            eprintln!("Try 'ditto --help' for more information.");
            std::process::exit(1);
        }
    }
}
