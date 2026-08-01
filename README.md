# Ditto

<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" height="160" alt="Ditto Logo" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-black?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-FFC131?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React" />
</p>

A lightweight, keyboard-driven, cross-platform clipboard manager built using Tauri v2, Rust, React, TypeScript, Vite, and Tailwind CSS v4.

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

Ensure you have Rust/Cargo installed, and use `bun` for package management:

* **Cargo**: Standard Rust toolchain.
* **Bun**: `bun install` for frontend packages.

### Commands

* Run the development server (auto-rebuilds Tauri backend & React frontend):

  ```bash
  bun run tauri dev
  ```

* Run Rust unit tests:

  ```bash
  cargo test --workspace
  ```

* Build optimized release:

  ```bash
  bun run tauri build
  ```
