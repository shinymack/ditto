pub mod active_window;
pub mod autostart;
pub mod controller;
pub mod environment;
pub mod x11;

pub use active_window::{ActiveWindowDetector, CompositeActiveWindowDetector};
pub use autostart::setup_autostart;
pub use controller::{CompositeWindowController, WindowController};
pub use environment::{DesktopEnvironment, LinuxEnvironment, SessionType};
pub use x11::is_window_mapped;

use std::sync::LazyLock;

static DETECTOR: LazyLock<CompositeActiveWindowDetector> =
    LazyLock::new(CompositeActiveWindowDetector::default);
static CONTROLLER: LazyLock<CompositeWindowController> =
    LazyLock::new(CompositeWindowController::default);

/// Detects the currently active window name or class across X11, GNOME, KDE, Sway, and Hyprland.
pub fn get_active_window_class() -> Option<String> {
    DETECTOR.detect_active_window()
}

/// Initializes window hints and positions window at initial coordinates immediately.
pub fn init_window_hints(x: Option<f32>, y: Option<f32>) {
    let mut win_id = x11::get_ditto_x11_window_id();
    if win_id.is_none() {
        for _ in 0..15 {
            std::thread::sleep(std::time::Duration::from_millis(2));
            win_id = x11::get_ditto_x11_window_id();
            if win_id.is_some() {
                break;
            }
        }
    }

    let Some(win_id) = win_id else {
        return;
    };

    let pos = match (x, y) {
        (Some(x), Some(y)) if x > 10.0 && y > 10.0 => Some((x as i32, y as i32)),
        _ => None,
    };

    let _ = x11::apply_window_hints(win_id, pos);

    if let Some((x, y)) = pos {
        CONTROLLER.move_resize_window(win_id, x, y, 800, 500);
    }
}

/// Hides the Ditto window by unmapping it in X11, keeping the WGPU surface alive.
/// Returns the true root-relative coordinates (x, y) of the window before unmapping.
pub fn hide_window() -> Option<(i32, i32)> {
    x11::unmap_and_get_coords()
}

/// Shows the Ditto window: activates it and positions it at (x, y).
pub fn show_window(x: i32, y: i32) {
    let Some(win_id) = x11::get_ditto_x11_window_id() else {
        return;
    };

    CONTROLLER.activate_window(win_id);
    CONTROLLER.move_resize_window(win_id, x, y, 800, 500);
}

/// Requests window manager focus for the Ditto window.
pub fn focus_by_pid() {
    if let Some(win_id) = x11::get_ditto_x11_window_id() {
        CONTROLLER.activate_window(win_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_environment_detection() {
        let env = LinuxEnvironment::detect();
        assert!(matches!(
            env.session_type,
            SessionType::Wayland | SessionType::X11 | SessionType::Unknown
        ));
        assert!(matches!(
            env.desktop,
            DesktopEnvironment::Gnome
                | DesktopEnvironment::Kde
                | DesktopEnvironment::Sway
                | DesktopEnvironment::Hyprland
                | DesktopEnvironment::Cosmic
                | DesktopEnvironment::Xfce
                | DesktopEnvironment::Generic
        ));
    }

    #[test]
    fn test_controller_cascade_availability() {
        let controller = CompositeWindowController::default();
        assert_eq!(controller.controllers.len(), 4);
        assert_eq!(controller.controllers[0].name(), "Hyprland");
        assert_eq!(controller.controllers[1].name(), "Sway");
        assert_eq!(controller.controllers[2].name(), "wmctrl");
        assert_eq!(controller.controllers[3].name(), "NativeX11");
    }

    #[test]
    fn test_detector_cascade() {
        let detector = CompositeActiveWindowDetector::default();
        assert_eq!(detector.detectors.len(), 4);
    }
}
