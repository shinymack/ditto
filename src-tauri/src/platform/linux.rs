use std::path::PathBuf;

const ICON_BYTES: &[u8] = include_bytes!("../../icons/icon.png");

pub fn get_active_window_class() -> Option<String> {
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
