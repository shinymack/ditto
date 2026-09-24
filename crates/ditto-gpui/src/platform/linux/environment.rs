use std::env;

/// The display server session type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Wayland,
    X11,
    Unknown,
}

/// The desktop environment or Wayland compositor in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    Gnome,
    Kde,
    Sway,
    Hyprland,
    Cosmic,
    Xfce,
    Generic,
}

/// Represents the detected Linux runtime desktop environment.
#[derive(Debug, Clone)]
pub struct LinuxEnvironment {
    pub session_type: SessionType,
    pub desktop: DesktopEnvironment,
}

impl LinuxEnvironment {
    /// Detects the current Linux display server and desktop environment from standard environment variables.
    pub fn detect() -> Self {
        let session_type = if env::var_os("WAYLAND_DISPLAY").is_some()
            || env::var("XDG_SESSION_TYPE")
                .map(|s| s == "wayland")
                .unwrap_or(false)
        {
            SessionType::Wayland
        } else if env::var_os("DISPLAY").is_some()
            || env::var("XDG_SESSION_TYPE")
                .map(|s| s == "x11")
                .unwrap_or(false)
        {
            SessionType::X11
        } else {
            SessionType::Unknown
        };

        let desktop = if env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            DesktopEnvironment::Hyprland
        } else if env::var_os("SWAYSOCK").is_some() {
            DesktopEnvironment::Sway
        } else {
            let xdg_current = env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .to_lowercase();
            let xdg_session = env::var("XDG_SESSION_DESKTOP")
                .unwrap_or_default()
                .to_lowercase();

            if xdg_current.contains("gnome") || xdg_session.contains("gnome") {
                DesktopEnvironment::Gnome
            } else if xdg_current.contains("kde")
                || xdg_current.contains("plasma")
                || xdg_session.contains("plasma")
            {
                DesktopEnvironment::Kde
            } else if xdg_current.contains("cosmic") || xdg_session.contains("cosmic") {
                DesktopEnvironment::Cosmic
            } else if xdg_current.contains("xfce") || xdg_session.contains("xfce") {
                DesktopEnvironment::Xfce
            } else {
                DesktopEnvironment::Generic
            }
        };

        Self {
            session_type,
            desktop,
        }
    }

    pub fn is_wayland(&self) -> bool {
        self.session_type == SessionType::Wayland
    }

    pub fn is_x11(&self) -> bool {
        self.session_type == SessionType::X11
    }
}
