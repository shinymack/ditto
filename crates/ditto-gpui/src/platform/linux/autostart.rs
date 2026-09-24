const ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon.png");

/// Installs XDG desktop autostart entry, application launcher, and icon.
pub fn setup_autostart() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };

    let desktop_entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Ditto\n\
         Comment=Lightweight clipboard manager\n\
         Exec={} run\n\
         Icon=ditto\n\
         Terminal=false\n\
         Categories=Utility;\n\
         StartupNotify=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.display()
    );

    if let Some(dir) = dirs::config_dir().map(|d| d.join("autostart")) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("ditto.desktop"), &desktop_entry);
    }
    if let Some(dir) = dirs::data_dir().map(|d| d.join("applications")) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("ditto.desktop"), &desktop_entry);
    }
    if let Some(dir) = dirs::data_dir().map(|d| d.join("icons/hicolor/256x256/apps")) {
        let _ = std::fs::create_dir_all(&dir);
        let icon_path = dir.join("ditto.png");
        if !icon_path.exists() {
            let _ = std::fs::write(icon_path, ICON_BYTES);
        }
    }
}
