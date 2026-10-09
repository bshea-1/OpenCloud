# OpenCloud — Implementation Roadmap

This roadmap outlines the complete development cycle of **OpenCloud** across 6 distinct phases. Each phase contains clear deliverables, technical specifications, and verification checkpoints.

---

## Progress Overview

| Phase | Description | Status |
| :--- | :--- | :--- |
| **Phase 1** | Project Foundation & Architecture Scaffolding | ✅ Completed |
| **Phase 2** | App Scanner & GitHub Release Catalog Engine | ✅ Completed |
| **Phase 3** | Lifecycle Engine (Downloader, Verifier & Installer) | ✅ Completed |
| **Phase 4** | Studio UI Design System & Component Library | ✅ Completed |
| **Phase 5** | Native App Launcher, Recent Projects & System View | ✅ Completed |
| **Phase 6** | Polish, Standalone Scripts & Multi-Platform Packaging | ✅ Completed |

---

## Phase 1: Project Foundation & Architecture Scaffolding
**Objective**: Establish the core desktop client repository structure in 100% pure Rust using `eframe` (v0.36) + `egui` (v0.36) with zero external runtime dependencies, native GPU rendering, and foundational data structures.

- [x] Configure pure-Rust `Cargo.toml` with `eframe`, `egui`, `tokio`, `reqwest`, `serde`
- [x] Configure native window sizing (1200x800, min 960x640), dark titlebar, and viewport
- [x] Define Rust data models (`SuiteApp`, `AppCategory`, `ReleaseAssetInfo`, `InstallState`)
- [x] Set up studio design tokens (dark obsidian palette, typography, layout geometry)
- [x] Verification: Project compiles and passes `cargo check` with zero warnings

---

## Phase 2: App Scanner & GitHub Release Catalog Engine
**Objective**: Build the intelligence layer that audits the local operating system to discover installed apps and queries the GitHub Releases API for the latest binaries.

- [x] Implement Rust local scanner (`scanner.rs`):
  - macOS: Scans `/Applications` and `~/Applications` for `{AppName}.app` and extracts `CFBundleShortVersionString` from `Contents/Info.plist`
  - Windows: Scans `%LOCALAPPDATA%\Programs\OpenCloud\{app}` and Windows Registry
  - Linux: Scans `~/.local/bin` and `/usr/local/bin`
- [x] Implement GitHub Releases API integration (`catalog.rs`):
  - Queries `https://api.github.com/repos/storytold/{repo}/releases/latest` for all 12 apps
  - Parses version tags, asset download URLs for universal macOS DMGs, Windows MSIs/Zips, and Linux AppImages
  - In-memory async update channel via Tokio and Crossbeam
- [x] Implement differential semver comparison (`Installed`, `UpdateAvailable`, `NotInstalled`)
- [x] Verification: Successfully matches local and remote versions for all 12 apps

---

## Phase 3: Lifecycle Engine (Downloader, Verifier & Installer)
**Objective**: Enable single-click automated downloads, verification, and native platform installation.

- [x] Implement streaming download engine with tokio/reqwest emitting real-time progress events
- [x] Implement platform installation orchestrators (`installer.rs`):
  - **macOS**: `hdiutil attach -nobrowse -quiet`, copy `.app` to `/Applications/`, strip quarantine flags (`xattr -dr com.apple.quarantine`), and `hdiutil detach`
  - **Windows**: Silent MSI execution via `msiexec.exe /i ... /qn` or portable zip extraction to `%LOCALAPPDATA%\Programs\OpenCloud`
  - **Linux**: Stream AppImage to `~/.local/bin/`, execute `chmod +x`, and generate `.desktop` file in `~/.local/share/applications/`
- [x] Implement clean application uninstaller (`installer::uninstall_app`)
- [x] Verification: Downloading and installing runs through background channels without blocking the UI thread

---

## Phase 4: Studio UI Design System & Component Library
**Objective**: Build a responsive dark-mode workstation UI with fluid rendering, category filtering, and real-time state indicators in pure Rust.

- [x] Develop Left Navigation Sidebar:
  - Workspaces: *All Applications*, *Installed Apps*, *Updates Available*
  - Category filters: *Photography*, *Vector & Design*, *Video & Motion*, *Audio*, *CAD & Drafting*, *Productivity & Office*
  - Studio views: *Recent Projects*, *System & Storage*
- [x] Build App Card Grid & Detail View:
  - Dual-letter mnemonic badge for each app (`Pc`, `Vc`, `Fc`, `Lc`, `Pd`, `Ec`, `Dc`, `Sc`, `Cd`, `Dk`, `Gc`, `Wc`)
  - Status indicators: `Installed`, `Update Available`, `Not Installed`
  - Real-time download progress bar and percentage
  - Action controls: `Launch`, `Install`, `Update`, context menu (`Uninstall`)
- [x] Top header bar with instantaneous search and update check button
- [x] Verification: UI renders with high performance and responsive interaction

---

## Phase 5: Native App Launcher, Recent Projects & System View
**Objective**: Turn OpenCloud into the central daily launchpad for creative workflows and cross-tool interoperability.

- [x] Implement native process spawner (`launcher.rs`):
  - macOS: `open -a "{AppName}" [filepath]`
  - Windows: `Start-Process "{AppExe}" [filepath]`
  - Linux: Direct fork/exec with file arguments
- [x] Implement Recent Projects aggregator:
  - Scans user directories for recently touched `.psd`, `.svg`, `.raw`, `.pdf`, `.mp4`, `.wav`, `.dxf`, `.docx` files
  - Clicking any file opens it immediately in its corresponding Craft application
- [x] Implement System & Storage diagnostics view
- [x] Verification: Launching an app opens the native app; opening a recent project passes the file argument to the target application

---

## Phase 6: Polish, Standalone Scripts & Multi-Platform Packaging
**Objective**: Productionize the system with standalone terminal scripts, auto-updater supervisor, rich preferences suite, and package builds.

- [x] Create standalone installation scripts in `scripts/`:
  - `scripts/install-macos.sh` (complete unattended macOS installer)
  - `scripts/install-windows.ps1` (complete PowerShell installer)
  - `scripts/install-linux.sh` (complete Linux installer)
- [x] Implement OpenCloud Desktop self-updater (`src/self_updater.rs`)
- [x] Implement Background Auto-Updater engine with per-app toggles and advanced options (`src/autoupdate.rs`)
- [x] Implement comprehensive 7-tab Preferences suite (`src/ui/settings_view.rs`)
- [x] Implement Creative Fonts Manager (`src/fonts.rs`)
- [x] Compile release binaries (`cargo build --release`)
- [x] End-to-end multi-platform verification and unit testing (`cargo test` passes 6/6 tests)
