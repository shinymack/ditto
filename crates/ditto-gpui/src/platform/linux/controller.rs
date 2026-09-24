use super::x11::{native_x11_activate, native_x11_moveresize};
use std::process::Command;

/// Trait defining a strategy for controlling, positioning, and focusing the Ditto window on Linux.
pub trait WindowController: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn activate(&self, window_id: u32) -> bool;
    fn move_resize(&self, window_id: u32, x: i32, y: i32, width: u32, height: u32) -> bool;
}

/// Strategy for Hyprland Wayland compositor using `hyprctl`.
pub struct HyprlandController;

impl WindowController for HyprlandController {
    fn name(&self) -> &'static str {
        "Hyprland"
    }

    fn is_available(&self) -> bool {
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    fn activate(&self, _window_id: u32) -> bool {
        Command::new("hyprctl")
            .args(["dispatch", "focuswindow", "class:ditto"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn move_resize(&self, _window_id: u32, x: i32, y: i32, width: u32, height: u32) -> bool {
        // Under Hyprland, windows can be positioned via hyprctl dispatch movewindowpixel or rules
        let move_arg = format!("{},{}", x, y);
        let resize_arg = format!("{},{}", width, height);
        let _ = Command::new("hyprctl")
            .args([
                "dispatch",
                "movewindowpixel",
                "exact",
                &move_arg,
                "class:ditto",
            ])
            .status();
        let _ = Command::new("hyprctl")
            .args([
                "dispatch",
                "resizewindowpixel",
                "exact",
                &resize_arg,
                "class:ditto",
            ])
            .status();
        true
    }
}

/// Strategy for Sway / wlroots Wayland compositor using `swaymsg`.
pub struct SwayController;

impl WindowController for SwayController {
    fn name(&self) -> &'static str {
        "Sway"
    }

    fn is_available(&self) -> bool {
        std::env::var_os("SWAYSOCK").is_some()
    }

    fn activate(&self, _window_id: u32) -> bool {
        let _ = Command::new("swaymsg")
            .args(["[app_id=\"ditto\"] focus"])
            .status();
        let _ = Command::new("swaymsg")
            .args(["[class=\"ditto\"] focus"])
            .status();
        true
    }

    fn move_resize(&self, _window_id: u32, x: i32, y: i32, width: u32, height: u32) -> bool {
        let move_arg = format!("position {} {}", x, y);
        let resize_arg = format!("resize set {} {}", width, height);
        let _ = Command::new("swaymsg")
            .args(["[app_id=\"ditto\"]", &move_arg])
            .status();
        let _ = Command::new("swaymsg")
            .args(["[app_id=\"ditto\"]", &resize_arg])
            .status();
        true
    }
}

/// Strategy using the `wmctrl` CLI if available on the system.
pub struct WmctrlController;

impl WmctrlController {
    fn has_wmctrl() -> bool {
        Command::new("wmctrl")
            .arg("-m")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl WindowController for WmctrlController {
    fn name(&self) -> &'static str {
        "wmctrl"
    }

    fn is_available(&self) -> bool {
        Self::has_wmctrl()
    }

    fn activate(&self, window_id: u32) -> bool {
        let hex_id = format!("0x{:08x}", window_id);
        Command::new("wmctrl")
            .args(["-i", "-R", &hex_id])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn move_resize(&self, window_id: u32, x: i32, y: i32, width: u32, height: u32) -> bool {
        let hex_id = format!("0x{:08x}", window_id);
        let mvarg = format!("0,{},{},{},{}", x, y, width, height);
        Command::new("wmctrl")
            .args(["-i", "-r", &hex_id, "-e", &mvarg])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Native pure-Rust X11 controller using direct EWMH protocol via `x11rb`.
/// Requires zero external CLI tools and works across X11 & XWayland.
pub struct NativeX11Controller;

impl WindowController for NativeX11Controller {
    fn name(&self) -> &'static str {
        "NativeX11"
    }

    fn is_available(&self) -> bool {
        std::env::var_os("DISPLAY").is_some()
    }

    fn activate(&self, window_id: u32) -> bool {
        native_x11_activate(window_id)
    }

    fn move_resize(&self, window_id: u32, x: i32, y: i32, width: u32, height: u32) -> bool {
        native_x11_moveresize(window_id, x, y, width, height)
    }
}

/// Composite controller providing prioritized fallback across window control strategies.
pub struct CompositeWindowController {
    pub(crate) controllers: Vec<Box<dyn WindowController>>,
}

impl Default for CompositeWindowController {
    fn default() -> Self {
        Self {
            controllers: vec![
                Box::new(HyprlandController),
                Box::new(SwayController),
                Box::new(WmctrlController),
                Box::new(NativeX11Controller),
            ],
        }
    }
}

impl CompositeWindowController {
    /// Activates the window using the first available working strategy.
    pub fn activate_window(&self, window_id: u32) -> bool {
        for controller in &self.controllers {
            if controller.is_available() {
                if controller.activate(window_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Moves and resizes the window using the first available working strategy.
    pub fn move_resize_window(
        &self,
        window_id: u32,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> bool {
        for controller in &self.controllers {
            if controller.is_available() {
                if controller.move_resize(window_id, x, y, width, height) {
                    return true;
                }
            }
        }
        false
    }
}
