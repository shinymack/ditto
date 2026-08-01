use std::path::PathBuf;

const ICON_BYTES: &[u8] = include_bytes!("../../icons/icon.png");

fn find_focused_sway_node(node: &serde_json::Value) -> Option<String> {
    if node.get("focused").and_then(|f| f.as_bool()) == Some(true) {
        if let Some(app_id) = node.get("app_id").and_then(|a| a.as_str()) {
            return Some(app_id.to_string());
        }
        if let Some(class) = node.get("window_properties").and_then(|w| w.get("class")).and_then(|c| c.as_str()) {
            return Some(class.to_string());
        }
    }
    if let Some(nodes) = node.get("nodes").and_then(|n| n.as_array()) {
        for child in nodes {
            if let Some(res) = find_focused_sway_node(child) {
                return Some(res);
            }
        }
    }
    if let Some(floating) = node.get("floating_nodes").and_then(|n| n.as_array()) {
        for child in floating {
            if let Some(res) = find_focused_sway_node(child) {
                return Some(res);
            }
        }
    }
    None
}

pub fn get_active_window_class() -> Option<String> {
    if let Ok(win) = x_win::get_active_window() {
        if !win.info.name.is_empty() {
            return Some(win.info.name);
        }
    }
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        if let Ok(output) = std::process::Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(class) = json.get("class").and_then(|c| c.as_str()) {
                        if !class.is_empty() {
                            return Some(class.to_string());
                        }
                    }
                }
            }
        }
    }

    // 2. Sway Wayland Query
    if std::env::var_os("SWAYSOCK").is_some() {
        if let Ok(output) = std::process::Command::new("swaymsg")
            .args(["-t", "get_tree"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(app_id) = find_focused_sway_node(&json) {
                        return Some(app_id);
                    }
                }
            }
        }
    }

    // 3. X11 / XWayland Fallback via xprop
    let output = std::process::Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let win_id = stdout.split('#').last()?.trim();

    if win_id.is_empty() || win_id == "0x0" {
        return None;
    }

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

pub fn setup_autostart() {
    let Ok(exe) = std::env::current_exe() else { return };

    let mut icon_path = PathBuf::from("/tmp/ditto.png");
    if let Some(data_dir) = dirs::data_dir() {
        let icon_dir = data_dir.join("icons/hicolor/512x512/apps");
        let _ = std::fs::create_dir_all(&icon_dir);
        let icon_file = icon_dir.join("ditto.png");
        let _ = std::fs::write(&icon_file, ICON_BYTES);
        icon_path = icon_file;
    }

    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Ditto\nExec={} run\nHidden=false\nX-GNOME-Autostart-enabled=true\nComment=Keyboard-driven clipboard manager\nIcon={}\nTerminal=false\nCategories=Utility;\nStartupWMClass=ditto\n",
        exe.display(),
        icon_path.display()
    );

    if let Some(autostart_dir) = dirs::config_dir().map(|d| d.join("autostart")) {
        let _ = std::fs::create_dir_all(&autostart_dir);
        let _ = std::fs::write(autostart_dir.join("ditto.desktop"), &content);
    }

    if let Some(apps_dir) = dirs::data_dir().map(|d| d.join("applications")) {
        let _ = std::fs::create_dir_all(&apps_dir);
        let _ = std::fs::write(apps_dir.join("ditto.desktop"), &content);
    }
}
pub fn focus_by_pid() {
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
