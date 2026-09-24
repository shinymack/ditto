use std::process::Command;

pub fn get_active_window_class() -> Option<String> {
    if let Ok(win) = x_win::get_active_window() {
        if !win.info.name.is_empty() {
            return Some(win.info.name);
        }
    }
    let output = Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get id of first application process whose frontmost is true",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

pub fn setup_autostart() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.shinymack.ditto</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        exe.display()
    );

    if let Some(home) = dirs::home_dir() {
        let launch_agents = home.join("Library/LaunchAgents");
        let _ = std::fs::create_dir_all(&launch_agents);
        let plist_path = launch_agents.join("com.shinymack.ditto.plist");
        let _ = std::fs::write(plist_path, plist_content);
    }
}

pub fn is_window_mapped() -> bool {
    false
}

pub fn init_window_hints(_x: Option<f32>, _y: Option<f32>) {}

pub fn focus_by_pid() {}

#[cfg(target_os = "macos")]
pub unsafe fn setup_macos_window_level(ns_window_ptr: *mut std::ffi::c_void) {
    if ns_window_ptr.is_null() {
        return;
    }
    extern "C" {
        fn sel_registerName(str: *const u8) -> *mut std::ffi::c_void;
        fn objc_msgSend();
    }
    let sel_set_collection_behavior = sel_registerName(b"setCollectionBehavior:\0".as_ptr());
    let sel_set_level = sel_registerName(b"setLevel:\0".as_ptr());

    let behavior: usize = (1 << 0) | (1 << 8); // CanJoinAllSpaces | FullScreenAuxiliary
    let level: isize = 25; // NSStatusWindowLevel

    let func_behavior: unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, usize) =
        std::mem::transmute(objc_msgSend as *const ());
    func_behavior(ns_window_ptr, sel_set_collection_behavior, behavior);

    let func_level: unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, isize) =
        std::mem::transmute(objc_msgSend as *const ());
    func_level(ns_window_ptr, sel_set_level, level);
}
