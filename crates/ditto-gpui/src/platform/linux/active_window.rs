use std::process::Command;

/// Trait defining a strategy for resolving the currently active/focused window identifier or class.
pub trait ActiveWindowDetector: Send + Sync {
    fn detect(&self) -> Option<String>;
}

/// Detector using the cross-platform `x_win` crate.
pub struct XWinDetector;

impl ActiveWindowDetector for XWinDetector {
    fn detect(&self) -> Option<String> {
        if let Ok(win) = x_win::get_active_window() {
            if !win.info.name.is_empty() {
                return Some(win.info.name);
            }
        }
        None
    }
}

/// Detector for Hyprland Wayland compositor using `hyprctl activewindow -j`.
pub struct HyprlandDetector;

impl ActiveWindowDetector for HyprlandDetector {
    fn detect(&self) -> Option<String> {
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
            return None;
        }

        let output = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        if let Some(class) = json.get("class").and_then(|c| c.as_str()) {
            if !class.is_empty() {
                return Some(class.to_string());
            }
        }
        if let Some(initial_class) = json.get("initialClass").and_then(|c| c.as_str()) {
            if !initial_class.is_empty() {
                return Some(initial_class.to_string());
            }
        }
        None
    }
}

/// Detector for Sway/wlroots Wayland compositor using `swaymsg -t get_tree`.
pub struct SwayDetector;

impl SwayDetector {
    fn find_focused_node(node: &serde_json::Value) -> Option<String> {
        if node.get("focused").and_then(|f| f.as_bool()) == Some(true) {
            if let Some(app_id) = node.get("app_id").and_then(|a| a.as_str()) {
                if !app_id.is_empty() {
                    return Some(app_id.to_string());
                }
            }
            if let Some(props) = node.get("window_properties") {
                if let Some(class) = props.get("class").and_then(|c| c.as_str()) {
                    return Some(class.to_string());
                }
            }
        }
        if let Some(nodes) = node.get("nodes").and_then(|n| n.as_array()) {
            for child in nodes {
                if let Some(r) = Self::find_focused_node(child) {
                    return Some(r);
                }
            }
        }
        if let Some(floating) = node.get("floating_nodes").and_then(|n| n.as_array()) {
            for child in floating {
                if let Some(r) = Self::find_focused_node(child) {
                    return Some(r);
                }
            }
        }
        None
    }
}

impl ActiveWindowDetector for SwayDetector {
    fn detect(&self) -> Option<String> {
        if std::env::var_os("SWAYSOCK").is_none() {
            return None;
        }

        let output = Command::new("swaymsg")
            .args(["-t", "get_tree"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        Self::find_focused_node(&json)
    }
}

/// Detector for X11 / XWayland sessions using `xprop`.
pub struct X11EwmhDetector;

impl ActiveWindowDetector for X11EwmhDetector {
    fn detect(&self) -> Option<String> {
        let output = Command::new("xprop")
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

        let class_output = Command::new("xprop")
            .args(["-id", win_id, "WM_CLASS"])
            .output()
            .ok()?;

        if !class_output.status.success() {
            return None;
        }

        Some(String::from_utf8_lossy(&class_output.stdout).to_string())
    }
}

/// Composite detector implementing Chain of Responsibility across available backends.
pub struct CompositeActiveWindowDetector {
    pub(crate) detectors: Vec<Box<dyn ActiveWindowDetector>>,
}

impl Default for CompositeActiveWindowDetector {
    fn default() -> Self {
        Self {
            detectors: vec![
                Box::new(XWinDetector),
                Box::new(HyprlandDetector),
                Box::new(SwayDetector),
                Box::new(X11EwmhDetector),
            ],
        }
    }
}

impl CompositeActiveWindowDetector {
    pub fn detect_active_window(&self) -> Option<String> {
        for detector in &self.detectors {
            if let Some(active) = detector.detect() {
                if !active.is_empty() {
                    return Some(active);
                }
            }
        }
        None
    }
}
