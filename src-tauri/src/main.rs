#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn print_help() {
    let art = "\x1b[1;33m  ____  _ _   _\n |  _ \\(_) |_| |_ ___\n | | | | | __| __/ _ \\\n | |_| | | |_| || (_) |\n |____/|_|\\__|\\__\\___/\x1b[0m";
    println!("{}", art);
    println!("  \x1b[1;37mDitto\x1b[0m \x1b[90mv{}\x1b[0m \x1b[90m-\x1b[0m A lightweight, keyboard-driven clipboard manager.", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  \x1b[1;33mUSAGE:\x1b[0m");
    println!("    ditto [COMMAND]");
    println!();
    println!("  \x1b[1;33mCOMMANDS:\x1b[0m");
    println!("    \x1b[1;37mstart\x1b[0m       Start the Ditto daemon in the background (detached)");
    println!("    \x1b[1;37mstop\x1b[0m        Stop the running Ditto daemon");
    println!("    \x1b[1;37mrun\x1b[0m         Run the Ditto daemon in the foreground");
    println!("    \x1b[1;37mtoggle\x1b[0m      Toggle window visibility (shows if hidden, hides if visible)");
    println!("    \x1b[1;37mclear\x1b[0m       Clear all clipboard history");
    println!("    \x1b[1;37mpause\x1b[0m       Pause clipboard monitoring");
    println!("    \x1b[1;37mresume\x1b[0m      Resume clipboard monitoring");
    println!("    \x1b[1;37mlist\x1b[0m        List last 50 clipboard items");
    println!("    \x1b[1;37mstatus\x1b[0m      Show daemon process state, database size, and configuration");
    println!("    \x1b[1;37mupdate\x1b[0m      Update Ditto to the latest release and restart daemon");
    println!("    \x1b[1;37m-v, --version\x1b[0m Show version information");
    println!("    \x1b[1;37m-h, --help\x1b[0m  Show this help message");
    println!("  \x1b[1;33mCONFIG FILE:\x1b[0m");
    println!("    {}", ditto_core::config::Config::file_path().display());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next();

    match cmd.as_deref() {
        Some("-h" | "--help") => {
            print_help();
        }
        Some("-v" | "--version" | "version") => {
            println!("ditto v{}", env!("CARGO_PKG_VERSION"));
        }
        None => {
            if ditto_lib::ipc::send_command("toggle").is_err() {
                ditto_lib::platform::setup_autostart();
                ditto_lib::run();
            }
        }
        Some("toggle") => {
            if ditto_lib::ipc::send_command("toggle").is_err() {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe)
                        .arg("run")
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    let _ = ditto_lib::ipc::send_command("toggle");
                }
            }
        }
        Some("clear") => {
            if ditto_lib::ipc::send_command("clear").is_ok() {
                println!("ditto: cleared history successfully");
            } else {
                eprintln!("ditto: error: daemon is not running");
                std::process::exit(1);
            }
        }
        Some("pause") => {
            if ditto_lib::ipc::send_command("pause").is_ok() {
                println!("ditto: clipboard monitoring paused");
            } else {
                eprintln!("ditto: error: daemon is not running");
                std::process::exit(1);
            }
        }
        Some("resume") => {
            if ditto_lib::ipc::send_command("resume").is_ok() {
                println!("ditto: clipboard monitoring resumed");
            } else {
                eprintln!("ditto: error: daemon is not running");
                std::process::exit(1);
            }
        }
        Some("list") => {
            let db_path = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("com.shinymack.ditto")
                .join("ditto.db");
            if !db_path.exists() {
                println!("ditto: no clipboard history found");
                return;
            }
            match ditto_core::db::Db::init(db_path.to_str().unwrap()) {
                Ok(db) => match db.get_history(50, None) {
                    Ok(items) => {
                        for (i, item) in items.iter().enumerate() {
                            let preview = item.content.replace('\n', " ");
                            let truncated = if preview.len() > 60 {
                                format!("{}...", &preview[..60])
                            } else {
                                preview
                            };
                            println!("{:2}. [{}] {}", i + 1, item.created_at, truncated);
                        }
                    }
                    Err(e) => eprintln!("ditto: error reading history: {}", e),
                },
                Err(e) => eprintln!("ditto: error opening database: {}", e),
            }
        }
        Some("start") => {
            if ditto_lib::ipc::is_daemon_running() {
                println!("ditto: daemon is already running");
                return;
            }
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .arg("run")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                println!("ditto: daemon started in the background");
            }
        }
        Some("stop") => {
            if ditto_lib::ipc::send_command("stop").is_ok() {
                println!("ditto: daemon stopped");
            } else {
                println!("ditto: daemon is not running");
            }
        }
        Some("update") => {
            println!("ditto: checking for updates...");
            let was_running = ditto_lib::ipc::is_daemon_running();
            if was_running {
                println!("ditto: stopping running daemon...");
                let _ = ditto_lib::ipc::send_command("stop");
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            println!("ditto: running installer script...");
            let install_status = std::process::Command::new("sh")
                .arg("-c")
                .arg("curl -fsSL https://raw.githubusercontent.com/shinymack/ditto/main/install.sh | sh")
                .status();

            match install_status {
                Ok(status) if status.success() => {
                    println!("ditto: updated successfully!");
                    if was_running {
                        println!("ditto: restarting daemon...");
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe)
                                .arg("run")
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        }
                    }
                }
                _ => {
                    eprintln!("ditto: error: update failed");
                    std::process::exit(1);
                }
            }
        }
        Some("status") => {
            let running = ditto_lib::ipc::is_daemon_running();
            let db_path = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("com.shinymack.ditto")
                .join("ditto.db");

            let (db_size, item_count) = if db_path.exists() {
                let size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
                let count = ditto_core::db::Db::init(db_path.to_str().unwrap())
                    .and_then(|db| db.get_history(10000, None))
                    .map(|items| items.len())
                    .unwrap_or(0);
                (size, count)
            } else {
                (0, 0)
            };

            let formatted_size = if db_size < 1024 {
                format!("{} B", db_size)
            } else if db_size < 1024 * 1024 {
                format!("{:.2} KB", db_size as f64 / 1024.0)
            } else {
                format!("{:.2} MB", db_size as f64 / (1024.0 * 1024.0))
            };

            let status_str = if running {
                "\x1b[32m● Running\x1b[0m"
            } else {
                "\x1b[31m○ Stopped\x1b[0m"
            };

            println!("\x1b[1;33mDitto Daemon Status:\x1b[0m");
            println!("  Daemon Process: {}", status_str);
            println!("  App Version:    v{}", env!("CARGO_PKG_VERSION"));
            println!("  Database Path:  {}", db_path.display());
            println!("  Database Size:  {}", formatted_size);
            println!("  History Count:  {} items", item_count);
            println!("  Config File:    {}", ditto_core::config::Config::file_path().display());
        }
        Some("run") => {
            if ditto_lib::ipc::is_daemon_running() {
                eprintln!("ditto: error: daemon is already running");
                std::process::exit(1);
            }
            ditto_lib::platform::setup_autostart();
            ditto_lib::run();
        }
        Some(unknown) => {
            eprintln!("ditto: error: unknown command '{}'", unknown);
            println!();
            print_help();
            std::process::exit(1);
        }
    }
}
