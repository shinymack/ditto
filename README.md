# Ditto

<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" height="160" alt="Ditto Logo" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-black?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-FFC131?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React" />
</p>

A lightweight, keyboard-driven clipboard manager for Linux. Built using Tauri v2, Rust, React, TypeScript, Vite, and Tailwind CSS v4.

## Core Constraints

To keep Ditto extremely fast and light on resources, the project adheres to the following constraints:

* **Resource Usage**: Designed to be lightweight, fast, and optimized for low memory usage.
* **Window Design**: Frameless, transparent, centered, and hidden immediately upon window blur.
* **Database (SQLite)**: Automatically floats duplicated clipboard entries to the top of the history.

## Repository Structure

* `/crates/ditto-core`: Core Rust library handling SQLite operations and the clipboard listener.
* `/src-tauri`: Desktop runner host that binds the frontend window and calls `ditto-core`.
* `/app`: Frontend application built with React, TypeScript, and Vite.

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

## Installation

You can install Ditto on Linux without cloning or building the repository by running this command in your terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/shinymack/ditto/main/install.sh | sh
```

This script detects your platform, downloads the latest precompiled release binary, places it in `~/.local/bin/ditto`, and registers it with your desktop environment.

If you prefer to build from source, you can clone this repository and run:

```bash
./install.sh
```
Once installed, you can start, pause, resume, toggle, or clear the history directly from your terminal using:
- `ditto start` - Start daemon in background
- `ditto toggle` - Toggle window visibility
- `ditto pause` - Pause monitoring
- `ditto resume` - Resume monitoring
- `ditto clear` - Clear database history
