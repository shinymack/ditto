# Ditto

<p align="center">
  <img src="crates/ditto-gpui/assets/icon.png" width="160" height="160" alt="Ditto Logo" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-black?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/UI-GPUI%20Kit-blue?style=for-the-badge" alt="GPUI Kit" />
  <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey?style=for-the-badge" alt="Platforms" />
</p>

A lightweight, keyboard-driven, cross-platform clipboard manager built with pure native Rust and GPUI Kit (`longbridge/gpui-kit`).

## Core Constraints

To keep Ditto extremely fast and light on resources, the project adheres to the following constraints:

* **Resource Usage**: Optimized for low memory footprint (~30-50MB RAM).
* **Window Design**: Frameless, transparent, centered, and hidden immediately on blur.
* **Background Daemon**: Hides completely from macOS Dock and Cmd+Tab switcher, operating purely as a background utility.
* **Database (SQLite)**: AES-256-GCM / Argon2id storage. Duplicate entries automatically float to top of history.

## Installation

### Linux & macOS (Terminal)

Run this command in your terminal to install the latest precompiled release binary:

```bash
curl -fsSL https://raw.githubusercontent.com/shinymack/ditto/main/install.sh | sh
```

This installs the binary to `~/.local/bin/ditto`.

### Windows

You can install Ditto on Windows using either method below:

#### Option A: GUI Installer (.exe Setup)
1. Download **`ditto_1.0.0_x64-setup.exe`** from [Releases](https://github.com/shinymack/ditto/releases/latest).
2. Double-click the downloaded setup file to launch the installation wizard.

#### Option B: PowerShell Command Line
Run this command in PowerShell to download and register the executable in `%LOCALAPPDATA%\Microsoft\WindowsApps`:

```powershell
irm https://raw.githubusercontent.com/shinymack/ditto/main/install.ps1 | iex
```

---

## Windows Keybinding & Background Daemon Setup

### 1. How the Daemon Starts
- On Windows, Ditto creates an autostart entry at `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\ditto.bat`.
- The background daemon starts automatically upon logging into Windows.
- You can also start or stop it anytime in PowerShell with `ditto start` or `ditto stop`.

### 2. Setting a Global Hotkey for `ditto toggle`
- **Method A (Native Windows Shortcut Key)**:
  Right-click `ditto.exe` (or Desktop shortcut) -> **Properties** -> Click **Shortcut key** field -> Press key combination (e.g., `Ctrl+Alt+V`) -> Click Apply.
- **Method B (PowerToys Keyboard Manager)**:
  Open PowerToys -> Keyboard Manager -> Remap a shortcut -> Target app: `ditto.exe` -> Args: `toggle`.
- **Method C (AutoHotkey)**:
  Add `^!v::Run "ditto toggle"` to your AutoHotkey script.

---

## CLI Usage

Manage Ditto directly from your terminal or command prompt:

- `ditto start` — Start background daemon
- `ditto stop` — Stop background daemon
- `ditto toggle` — Toggle window visibility
- `ditto status` — Show process state, DB size, and config info
- `ditto update` — Update Ditto to the latest release
- `ditto -v`, `ditto --version` — Show version info

---

## Development Setup

### Prerequisites

Ensure you have the standard Rust/Cargo toolchain installed (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`).

### Commands

* Run Ditto in development mode:

  ```bash
  cargo run -p ditto-gpui -- run
  ```

* Run unit and search latency benchmark tests:

  ```bash
  cargo test --workspace
  ```

* Build optimized release binary:

  ```bash
  cargo build --release -p ditto-gpui
  ```
