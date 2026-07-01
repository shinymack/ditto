# Ditto

A lightweight, keyboard-driven clipboard manager for Linux. Built using Tauri v2, Rust, React, TypeScript, Vite, and Tailwind CSS v4.

## Core Constraints

To keep Ditto extremely fast and light on resources, the project adheres to the following constraints:
* **Binary Size**: Release binary must be under **20MB** (compiled with size optimizations).
* **Memory Footprint**: Target RAM usage is under **50MB**.
* **Window Design**: Frameless, transparent, centered, and hidden immediately upon window blur.
* **Database (SQLite)**: Automatically floats duplicated clipboard entries to the top of the history.

## Repository Structure

* `/crates/ditto-core`: Core Rust library handling SQLite operations and the clipboard listener.
* `/src-tauri`: Desktop runner host that binds the frontend window and calls `ditto-core`.
* `/app`: Frontend application built with React, TypeScript, and Vite.
* `.docs/`: Documentation guides and rule definitions.

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
