# Novaframe Engine

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Storefront](https://img.shields.io/badge/Storefront-novaframe.co.uk-blueviolet.svg)](https://www.novaframe.co.uk)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-orange.svg)](https://v2.tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-red.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-lightgrey.svg)]()
[![Download Novaframe Dynamic Wallpaper Engine](https://a.fsdn.com/con/app/sf-download-button)](https://sourceforge.net/projects/novaframe-engine/files/latest/download)

Novaframe Engine is the open-source, ultra-lean desktop runtime for [Novaframe](https://www.novaframe.co.uk) live interactive wallpapers and creator marketplace. Engineered with Tauri v2 and Rust, it anchors interactive web-driven canvases directly beneath desktop icons with near-zero idle resource consumption.

Browse and install interactive wallpapers at [novaframe.co.uk](https://www.novaframe.co.uk).

---

## Key Invariants & Architecture

1. **Zero-Waste Power Management (Occlusion & Fullscreen Detection)**
   * **Windows:** Polls `SHQueryUserNotificationState` and enumerates desktop work areas (`desktop_is_covered`) via `Win32_Graphics_Gdi` to automatically pause animation loops whenever a full-screen application or game is active, or all display monitors are obscured.
   * **macOS:** Window-level anchoring via `objc2-app-kit` ensures true desktop underlay behavior (`kCGDesktopWindowLevel - 1`) beneath the menu bar, desktop icons, and active windows.

2. **Isolated Asset & Theme Protocols**
   * Uses custom scheme protocols (`asset:`, `theme:`, `novaframe:`) and strict Content Security Policies (`CSP`) to sandbox user-loaded themes and prevent unauthorized network egress.
   * Supports local theme streaming, WebGL / Three.js canvases, and local state persistence via `@tauri-apps/plugin-store`.

3. **System Integration & Deep-Linking**
   * Single-instance enforcement (`tauri-plugin-single-instance`) routes `novaframe://` URI activations directly to the running process for seamless 1-click web marketplace installation.
   * Native auto-updater integration backed by Minisign cryptographically verified signature chains.
   * Auto-starts on login (`tauri-plugin-autostart`) with zero UI interruption.

---

## Repository Structure

```
├── src/                  # Vanilla frontend app (UI controls, theme viewport)
│   ├── app.js            # Main frontend orchestrator
│   └── app.bundle.js     # esbuild output bundle
├── src-tauri/            # Rust native backend
│   ├── Cargo.toml        # Tauri v2 & OS dependency manifest
│   ├── tauri.conf.json   # Multi-window definitions, capabilities, and update pubkey
│   └── src/
│       ├── main.rs       # App initialization, IPC command handlers, window routing
│       └── deeplink.rs   # novaframe:// URI protocol handling & verification
└── resources/            # Default offline themes & WebGL vendor dependencies
```

---

## Development Setup

### Prerequisites

* **Rust:** Current stable toolchain (`rustup update stable`)
* **Node.js:** v18+ with `npm`
* **macOS Prerequisites:** Xcode Command Line Tools
* **Windows Prerequisites:** Visual Studio C++ Build Tools & WebView2 runtime

### 1. Install Dependencies

```bash
npm install
```

### 2. Run in Development Mode

```bash
# Starts the esbuild watcher and launches the Tauri development window
npm run watch &
npx tauri dev
```

### 3. Production Build

```bash
# Bundle frontend and compile native binaries
npm run build
npx tauri build
```

Artifacts are compiled into `src-tauri/target/release/bundle/`:
* **macOS:** `.dmg` and `.app` bundles (signed and notarized in release CI).
* **Windows:** `.msi` and `.exe` NSIS installers.

---

## License

Distributed under the MIT License. See `LICENSE` for more information.
