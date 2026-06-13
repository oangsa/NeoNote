# NeoNote - Implementation Plan

> A native Windows GUI shell built in Rust that wraps Neovide with a launcher, tab system, theme engine, and settings panel. No WebView. No terminal required.

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Architecture](#2-architecture)
3. [Module Layout](#3-module-layout)
4. [Tech Stack](#4-tech-stack)
5. [Phase 0 - Repo Setup](#5-phase-0--repo-setup)
6. [Phase 1 - Neovide Embedding](#6-phase-1--neovide-embedding)
7. [Phase 2 - Custom Chrome and Status Bar](#7-phase-2--custom-chrome-and-status-bar)
8. [Phase 3 - Tab System](#8-phase-3--tab-system)
9. [Phase 4 - Theme Engine Core](#9-phase-4--theme-engine-core)
10. [Phase 5 - Theme Panel UI](#10-phase-5--theme-panel-ui)
11. [Phase 6 - Settings Panel](#11-phase-6--settings-panel)
12. [Phase 7 - Launcher and Sidebar](#12-phase-7--launcher-and-sidebar)
13. [Phase 8 - Polish and Installer](#13-phase-8--polish-and-installer)
14. [Data and Persistence](#14-data-and-persistence)
15. [Error Handling Contracts](#15-error-handling-contracts)
16. [Built-in Themes](#16-built-in-themes)
17. [Milestone Summary](#17-milestone-summary)

---

## 1. Project Overview

### What It Is

NeoNote is a native Windows GUI shell built in Rust that:

- Spawns Neovide as a child process and embeds its window using Win32 `SetParent`
- Wraps it with a custom title bar, tab bar, sidebar, status bar, and launcher screen
- Provides a first-class theme engine that styles both the wrapper UI and passes colorschemes into NeoVim via RPC
- Requires zero terminal interaction from the end user

### What It Is Not

- Not a terminal emulator
- Not a WebView-based app - no Electron, no Tauri, no Chromium
- Not a NeoVim reimplementation - Neovide and NeoVim do all editor work

### Goals

- Full access to the user's `init.lua`, plugins, and NeoVim config
- Neovide cursor trail effects (railgun, torpedo, pixiedust, etc.) preserved after embedding
- One-click theme switching affecting both wrapper chrome and editor colorscheme
- Ships as a single `.msi` installer with Neovide bundled

---

## 2. Architecture

```text
+----------------------------------------------------------+
|                      NeoNote Process                     |
|                                                          |
|  +------------+  +--------------+  +------------------+  |
|  |  Launcher  |  |   Sidebar    |  |   Theme Engine   |  |
|  +------------+  +--------------+  +------------------+  |
|                                                          |
|  +----------------------------------------------------+  |
|  |  egui host window (frameless eframe)               |  |
|  |                                                    |  |
|  |  +----------------------------------------------+  |  |
|  |  |  Neovide HWND (reparented via SetParent)     |  |  |
|  |  |  wgpu renders here - GPU context preserved   |  |  |
|  |  +----------------------------------------------+  |  |
|  +----------------------------------------------------+  |
|                          |                               |
+--------------------------+-------------------------------+
                           | msgpack-RPC over TCP (127.0.0.1)
                           v
                     NeoVim (inside Neovide)
                  mode, cursor, buffer events
```

### Process Model

- NeoNote is the parent process and owns all window chrome
- Neovide is a child process per tab - its HWND is reparented into NeoNote's panel
- NeoVim runs inside Neovide - NeoNote does not spawn NeoVim directly
- NeoNote connects to NeoVim via msgpack-RPC over a TCP loopback socket to read mode and cursor for the status bar

### Tab Memory Model

Each tab owns one full Neovide process plus a wgpu GPU context. This is intentionally heavy. The tradeoff is accepted because:

- It gives each tab true isolation - a crash in one tab does not affect others
- The user's NeoVim config, plugins, and LSP run independently per tab
- NeoNote pre-warms one idle Neovide process in the background to make new tab opening feel instant

If memory becomes a concern in a future version, the alternative is a single Neovide instance with NeoVim buffer switching (`:tabnew`, `:bnext`). That approach loses process isolation and is deferred.

### HWND Embedding Risk and Fallback

Reparenting a `wgpu`-rendered window (Neovide) via Win32 `SetParent` can fail on some GPU driver and Windows version combinations because the swap chain is tied to the original HWND surface. The embedding strategy therefore has two tiers:

**Tier 1 - SetParent (preferred)**
Reparent Neovide's HWND directly into NeoNote's panel. Neovide's GPU context continues rendering into the same HWND surface. This works on most modern Windows 10 and Windows 11 configurations.

**Tier 2 - Side-by-side positioning (fallback)**
If rendering breaks or goes black after SetParent, NeoNote falls back to positioning Neovide's window to always overlap NeoNote's panel area at the same screen coordinates. NeoNote's chrome is drawn on top using a transparent overlay window with `WS_EX_LAYERED`. The user experience is visually identical. SetParent is not used.

The fallback is detected automatically: after SetParent, NeoNote sends a test paint and checks if Neovide's window responds within 500ms. If not, it switches to Tier 2 silently.

---

## 3. Module Layout

```text
neonote/
+-- Cargo.toml
+-- build.rs                    <- embed app manifest for DPI awareness
+-- assets/
|   +-- themes/
|       +-- built-in/           <- shipped theme JSON files
+-- docs/
|   +-- neonote-implementation-plan.md
|   +-- skills/
|       +-- skill-hwnd-embedding.md
|       +-- skill-egui-layout.md
|       +-- skill-nvim-rpc.md
|       +-- skill-theme-engine.md
|       +-- skill-neovide-process.md
|       +-- skill-windows-effects.md
|       +-- skill-persistence.md
+-- src/
    +-- main.rs                 <- eframe entry point, app bootstrap
    +-- app.rs                  <- NeoNoteApp struct, top-level update loop
    +-- embed/
    |   +-- mod.rs              <- NeovideInstance: spawn, HWND discovery, resize
    |   +-- hwnd.rs             <- Win32 SetParent, ShowWindow, MoveWindow helpers
    |   +-- fallback.rs         <- Tier 2 overlay positioning logic
    +-- rpc/
    |   +-- mod.rs              <- RpcClient: connect, send, receive
    |   +-- handler.rs          <- nvim-rs Handler impl, event dispatch
    |   +-- api.rs              <- typed wrappers: get_mode, get_cursor, set_colorscheme
    +-- tabs/
    |   +-- mod.rs              <- TabBar widget, tab state vec
    |   +-- tab.rs              <- Tab struct: owns NeovideInstance + RpcClient
    +-- theme/
    |   +-- mod.rs              <- ThemeStore: load, active, apply
    |   +-- schema.rs           <- Theme struct, serde deserialization
    |   +-- visuals.rs          <- map Theme colors to egui::Visuals
    |   +-- apply.rs            <- send colorscheme + cursor globals via RPC
    +-- ui/
    |   +-- titlebar.rs         <- custom frameless title bar
    |   +-- statusbar.rs        <- status bar, mode indicator
    |   +-- sidebar.rs          <- collapsible sidebar, file explorer
    |   +-- launcher.rs         <- startup launcher screen
    |   +-- theme_panel.rs      <- theme browser and custom builder
    |   +-- settings_panel.rs   <- settings panel sections
    |   +-- toast.rs            <- non-blocking toast notifications
    +-- persistence/
    |   +-- mod.rs              <- AppData root: paths, load/save orchestration
    |   +-- config.rs           <- AppConfig struct
    |   +-- session.rs          <- SessionState struct
    |   +-- recent_files.rs     <- RecentFile list, max-20 trimming
    +-- platform/
        +-- mod.rs
        +-- effects.rs          <- DWM Mica/Acrylic, layered opacity
        +-- fonts.rs            <- DirectWrite font enumeration
```

---

## 4. Tech Stack

| Layer | Crate | Purpose |
|---|---|---|
| GUI framework | `egui` + `eframe` 0.31 | All wrapper UI |
| Win32 embedding | `windows` 0.58 | SetParent, ShowWindow, MoveWindow, EnumWindows |
| NeoVim RPC | `nvim-rs` 0.6 + `tokio` | Mode and cursor tracking via msgpack-RPC |
| msgpack types | `rmpv` 1.0 | Value types for RPC messages |
| Serialization | `serde` + `serde_json` 1 | Theme JSON, config, session |
| File dialogs | `rfd` 0.14 | Native Windows open/save pickers |
| Font enumeration | `windows` DirectWrite feature | List installed monospace fonts |
| Window effects | `windows` DWM feature | Mica, Acrylic, opacity |
| Async runtime | `tokio` 1 | RPC communication, process monitoring |

### Cargo.toml

```toml
[package]
name    = "neonote"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe     = "0.31"
egui       = "0.31"
windows    = { version = "0.58", features = [
  "Win32_UI_WindowsAndMessaging",
  "Win32_System_Threading",
  "Win32_Foundation",
  "Win32_Graphics_Dwm",
  "Win32_Graphics_DirectWrite",
] }
nvim-rs    = { version = "0.6", features = ["use_tokio"] }
tokio      = { version = "1", features = ["full"] }
rmpv       = "1.0"
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
rfd        = "0.14"

[build-dependencies]
winres = "0.1"
```

---

## 5. Phase 0 - Repo Setup

**Goal:** Establish the repo structure, module skeletons, and all assets before any logic is written.

**Status:** Done

### Tasks

- [x] Initialize the root Rust binary crate
- [x] Create the full `src/` module tree with module files and stub structs
- [x] Create `docs/`, `docs/skills/`, and `assets/themes/built-in/`
- [x] Copy `neonote-implementation-plan.md` into `docs/`
- [x] Add all built-in theme JSON files to `assets/themes/built-in/`
- [x] Write `build.rs` to embed the DPI-aware app manifest via `winres`
- [x] Add `AGENTS.md` at repo root
- [x] Confirm Cargo produces a buildable empty window app shell
- [x] Set up `.gitignore` for `target/`, `*.msi`
- [x] Create local `main` and `dev` branches; protect `main` in the remote host settings

### Commit

```text
chore(setup): init NeoNote repo with module layout and built-in theme assets
```

---

## 6. Phase 1 - Neovide Embedding

**Goal:** Spawn Neovide and embed its window reliably inside NeoNote. This is the highest-risk phase. Do not proceed to Phase 2 until all acceptance criteria pass.

### Neovide Path Resolution

Resolve in this order:

1. Bundled `neovide.exe` alongside `neonote.exe`
2. `NEOVIDE_PATH` environment variable
3. `neovide` on system PATH
4. If none found: show a blocking setup dialog with instructions - do not crash

### Spawn Flags

```text
neovide.exe --no-fork --neovim-bin nvim --listen 127.0.0.1:{port}
```

Where `{port}` is a randomly selected free TCP port that NeoNote picks before spawning. This is how NeoNote gets a stable RPC address for Phase 2.

### HWND Discovery

After spawning, poll every 50ms up to 5 seconds using `EnumWindows` filtered by the child process PID. If no HWND is found after 5 seconds, show a recoverable error toast and clean up the process.

### Embedding Sequence (Tier 1)

```text
spawn neovide.exe --no-fork --listen 127.0.0.1:{port}
  -> poll EnumWindows until HWND found (max 5s)
  -> SetParent(neovide_hwnd, panel_hwnd)
  -> SetWindowLong(neovide_hwnd, GWL_STYLE, WS_CHILD | WS_VISIBLE)
  -> SetWindowLong(neovide_hwnd, GWL_EXSTYLE, 0)   <- strip WS_EX_APPWINDOW
  -> MoveWindow(neovide_hwnd, 0, 0, panel_w, panel_h, TRUE)
  -> wait 500ms -> check if Neovide window is responding
  -> if black/unresponsive: fall back to Tier 2
```

### Fallback Sequence (Tier 2)

```text
do NOT call SetParent
  -> position Neovide window at panel's screen coordinates
  -> create NeoNote overlay window (WS_EX_LAYERED | WS_EX_TRANSPARENT for editor area)
  -> on every NeoNote move/resize: reposition Neovide window to match
  -> draw chrome (titlebar, statusbar) on the overlay above Neovide
```

### Tasks

- [ ] Implement `NeovideInstance`: spawn, port selection, path resolution, process handle
- [ ] Implement `hwnd.rs`: `EnumWindows` + PID filter, `SetParent`, `MoveWindow`, `ShowWindow`
- [ ] Implement Tier 1 embedding with 500ms render health check
- [ ] Implement `fallback.rs`: overlay window, coordinate tracking, resize sync
- [ ] Handle Neovide process exit: close toast with "Reopen" button, clean up HWND state
- [ ] Resize Neovide to fill panel on every egui layout change
- [ ] Suppress Neovide's own title bar from appearing in the Windows taskbar

### Acceptance Criteria

- Neovide renders inside NeoNote's window with no black areas
- Cursor trail animations (e.g. railgun) are visible inside the embedded panel
- Resizing NeoNote resizes the embedded Neovide panel correctly
- No terminal window or second taskbar entry appears
- Neovide crash or exit produces a recoverable toast - NeoNote does not crash
- Tier 2 fallback activates automatically if Tier 1 rendering fails

---

## 7. Phase 2 - Custom Chrome and Status Bar

**Goal:** Replace the default Windows title bar with custom egui chrome and add a status bar that reads live data from NeoVim via RPC.

### RPC Connection

NeoNote connects to NeoVim using the TCP address it passed to Neovide at spawn time (`127.0.0.1:{port}`). This is a standard NeoVim `--listen` socket. Connection is established asynchronously after Phase 1 embedding confirms Neovide is alive.

NeoNote does not call `nvim_ui_attach` - it is not a UI replacement. It connects only to read events (mode changes, cursor moves) and to send commands (`:w`, `:colorscheme`, etc.).

### Title Bar

- [ ] Enable frameless mode in `eframe` (`NativeOptions::decorated = false`)
- [ ] Draw custom title bar in egui: app name left, filename center, modified indicator (`*`), window controls right
- [ ] Implement drag-to-move via `WM_NCHITTEST` override or egui drag region
- [ ] Minimize, maximize, close buttons call `ShowWindow` / `PostMessage(WM_CLOSE)` via `windows-rs`
- [ ] Double-click title bar toggles maximize

### Menu Bar

File menu:
- [ ] New File -> sends `:enew` via RPC
- [ ] Open File -> `rfd` file picker -> sends `:e {path}` via RPC
- [ ] Open Folder -> `rfd` folder picker -> sends `:cd {path}` via RPC
- [ ] Save -> sends `:w` via RPC
- [ ] Save As -> `rfd` save picker -> sends `:saveas {path}` via RPC
- [ ] Recent Files -> submenu populated from `recent_files.json`
- [ ] Exit -> confirm if any tab has unsaved changes, then close

View menu:
- [ ] Toggle Sidebar (`Ctrl+B`)
- [ ] Toggle Status Bar
- [ ] Zen Mode (`F11`) - hide all chrome, only Neovide panel visible

### Status Bar

- [ ] Subscribe to NeoVim `ModeChanged` autocmd via RPC to track current mode
- [ ] Subscribe to `CursorMoved` autocmd to track line and column
- [ ] Display segments: `[MODE]  filename  encoding  line endings  Ln {n}  Col {n}`
- [ ] Mode label background color driven by `theme.vim_modes.{mode}` - changes instantly on mode switch
- [ ] If RPC is disconnected, show `[--]` for mode and dim the status bar

### VIM Mode Color Mapping

| Mode | Label | Color key |
|---|---|---|
| Normal | NORMAL | `vim_modes.normal` |
| Insert | INSERT | `vim_modes.insert` |
| Visual | VISUAL | `vim_modes.visual` |
| Command-line | COMMAND | `vim_modes.command` |
| Replace | REPLACE | `vim_modes.replace` |

---

## 8. Phase 3 - Tab System

**Goal:** Support multiple simultaneous editor sessions via a tab bar. Each tab owns one Neovide process.

### Tab Struct

```rust
pub struct Tab {
    pub id:       Uuid,
    pub title:    String,
    pub modified: bool,
    pub nvim_instance: NeovideInstance,
    pub rpc:      RpcClient,
    pub hwnd:     HWND,
}
```

### Tab Switching

Switching tabs calls `ShowWindow(old_hwnd, SW_HIDE)` then `ShowWindow(new_hwnd, SW_SHOW)` and moves the visible HWND to fill the panel. The RPC connection switches to the new tab's client.

### Pre-warming

NeoNote keeps one idle Neovide process spawned in the background at all times. When the user opens a new tab, the pre-warmed process is promoted to that tab and a new pre-warm begins. This makes new tab opening feel instant.

Pre-warm is paused when the machine has less than 200MB of available RAM to avoid pressure on low-memory systems. Check via `GlobalMemoryStatusEx`.

### Tasks

- [ ] Define `Tab` struct and `TabBar` widget
- [ ] Render tab strip below the title bar in egui
- [ ] Implement HWND show/hide switching
- [ ] Implement pre-warm process management
- [ ] `+` button: promote pre-warmed instance or spawn fresh if pre-warm is not ready
- [ ] Close button: check `vim.bo.modified` via RPC
  - If clean: send `:q`, wait for process exit, remove tab
  - If dirty: show confirmation dialog (Save / Discard / Cancel)
  - If process does not exit within 3 seconds: force kill
- [ ] Right-click context menu: Close, Close Others, Close All, Copy Path
- [ ] Drag to reorder tabs
- [ ] Modified indicator `*` in tab label, driven by RPC `BufModifiedSet` autocmd
- [ ] If all tabs are closed: show launcher screen

---

## 9. Phase 4 - Theme Engine Core

**Goal:** Define the theme schema, load themes from disk, apply them to the egui wrapper, and push colorscheme and cursor settings to NeoVim via RPC.

### Theme JSON Schema

```json
{
  "name": "Catppuccin Mocha",
  "variant": "dark",
  "author": "catppuccin",
  "colors": {
    "background":       "#1e1e2e",
    "background_alt":   "#181825",
    "surface":          "#313244",
    "border":           "#45475a",
    "text":             "#cdd6f4",
    "text_muted":       "#6c7086",
    "accent_primary":   "#cba6f7",
    "accent_secondary": "#89b4fa",
    "success":          "#a6e3a1",
    "warning":          "#f9e2af",
    "error":            "#f38ba8",
    "cursor":           "#f5c2e7"
  },
  "vim_modes": {
    "normal":  "#89b4fa",
    "insert":  "#a6e3a1",
    "visual":  "#cba6f7",
    "command": "#f9e2af",
    "replace": "#f38ba8"
  },
  "neovim_colorscheme": "catppuccin",
  "neovide": {
    "cursor_animation_length": 0.13,
    "cursor_trail_size":       0.8,
    "cursor_vfx_mode":         "railgun"
  }
}
```

### Cursor VFX Options

| Value | Effect |
|---|---|
| `"railgun"` | Particles shoot along trail |
| `"torpedo"` | Torpedo animation |
| `"pixiedust"` | Sparkle particles |
| `"sonicboom"` | Shockwave ring on stop |
| `"ripple"` | Water ripple |
| `"wireframe"` | Geometric wireframe trail |
| `""` | Clean cursor, no effect |

### Tasks

- [ ] Define `Theme` struct with `serde::Deserialize`
- [ ] Implement hex color parser: `"#1e1e2e"` -> `egui::Color32`
- [ ] Build `ThemeStore`: scan `assets/themes/built-in/` and `%APPDATA%\NeoNote\themes\user\` on startup, cache all parsed themes
- [ ] Implement `apply_to_egui_visuals(theme) -> egui::Visuals`
- [ ] On theme activate: update `egui::Visuals`, then send via RPC:
  - `:colorscheme {neovim_colorscheme}` (graceful no-op if plugin not installed)
  - `vim.g.neovide_cursor_animation_length = {value}`
  - `vim.g.neovide_cursor_trail_size = {value}`
  - `vim.g.neovide_cursor_vfx_mode = "{value}"`
- [ ] Persist active theme name to `config.json`
- [ ] Restore active theme on startup before first paint

---

## 10. Phase 5 - Theme Panel UI

**Goal:** Build a browsable theme panel with live preview, import, and a custom theme builder.

### Layout

```text
+-------------------------------+
|  Themes                    X  |
+-------------------------------+
|  [ Dark ]  [ Light ]  [ All ] |
+-------------------------------+
|  +----------+  +----------+   |
|  | Tokyo    |  | Catppuc  |   |
|  | Night    |  | cin Mocha|   |
|  | [swatches]  [swatches] |   |
|  | [ Apply ]|  |[Apply v] |   |
|  +----------+  +----------+   |
+-------------------------------+
|  [ + Import Theme ]           |
|  [ + Create Custom Theme ]    |
+-------------------------------+
```

### Tasks

- [ ] Open panel from Theme menu or `Ctrl+Shift+T`
- [ ] Filter tabs: Dark / Light / All
- [ ] Theme cards: name, author, 6-color swatch strip, Apply button
- [ ] Hover card: live preview on wrapper UI only - NeoVim colorscheme not sent yet
- [ ] Mouse out without applying: revert wrapper to committed theme
- [ ] Apply button: commit theme - send colorscheme and cursor globals via RPC
- [ ] Active theme card shows checkmark
- [ ] Import button: `rfd` file picker for `.json` -> validate schema -> copy to `%APPDATA%\NeoNote\themes\user\` -> reload ThemeStore -> show success/error toast
- [ ] Custom theme builder: color pickers for each token, live preview, name field, Save button -> write to `themes/user/`

---

## 11. Phase 6 - Settings Panel

**Goal:** Build a settings panel covering all app configuration. All changes persist immediately - no Save button.

### General

- [ ] Default open folder (path picker, stored in `config.json`)
- [ ] Auto-restore last session on launch (toggle)
- [ ] Show launcher on startup (toggle)

### Editor

- [ ] Font family: searchable dropdown populated by DirectWrite font enumeration, filtered to monospace families
- [ ] Font size: slider + number input (8-32px) - sends `vim.o.guifont = "{family}:h{size}"` via RPC on change
- [ ] Line height: slider (1.0-2.0) - sends `vim.g.neovide_scale_factor` via RPC
- [ ] Tab size: segmented control 2 / 4 / 8 - sends `vim.o.tabstop` via RPC
- [ ] Word wrap: toggle - sends `vim.o.wrap` via RPC

### Cursor and Animation

- [ ] Enable cursor trail: master toggle - sends `vim.g.neovide_cursor_vfx_mode = ""` when off
- [ ] Cursor effect: dropdown (railgun / torpedo / pixiedust / sonicboom / ripple / wireframe / none)
- [ ] Animation length: slider (0.05-0.5s)
- [ ] Trail size: slider (0.1-1.0)
- [ ] Cursor blink: toggle - sends `vim.o.guicursor` modification via RPC
- [ ] Note: these settings override the active theme's cursor config when changed manually

### Window

- [ ] Startup size: Windowed / Maximized / Fullscreen
- [ ] Window opacity: slider (70-100%) - calls `SetLayeredWindowAttributes` immediately
- [ ] Blur behind: toggle - calls `DwmSetWindowAttribute` (Acrylic on Win10, Mica on Win11)
- [ ] Remember window geometry: toggle

### Keybindings

- [ ] Table of wrapper-level shortcuts with editable key column
- [ ] Click row to capture new key combination
- [ ] Note: NeoVim keybinds are managed in `init.lua`, not here

### About

- [ ] App version (from `Cargo.toml` via `env!("CARGO_PKG_VERSION")`)
- [ ] Neovide version (from `neovide --version` stdout)
- [ ] NeoVim version (from RPC `nvim_get_api_info`)
- [ ] Link to GitHub repository

---

## 12. Phase 7 - Launcher and Sidebar

**Goal:** Add a startup launcher screen and a collapsible sidebar.

### Launcher Screen

Shown at startup when no file argument is passed and `show_launcher_on_startup` is true.

- [ ] App name centered at top
- [ ] New File, Open File, Open Folder buttons
- [ ] Recent files list: max 20 entries, sorted by last opened, shows filename + truncated path + timestamp
  - Click to open file
  - Right-click: Remove from list
- [ ] Theme preview strip at bottom: one color swatch row per built-in theme, click to activate
- [ ] Animated transition to editor view when a file is opened (fade + slide)
- [ ] If a Neovide pre-warm is ready, it is shown in the background behind the launcher so the editor appears instantly on open

### Sidebar

- [ ] Toggle with `Ctrl+B` or View menu
- [ ] Three tabs: File Explorer, Recent Files, Bookmarks
- [ ] File Explorer: directory tree of current open folder, expand/collapse, click to open, right-click for Open in New Tab / Copy Path / Reveal in Explorer
- [ ] Recent Files: mirrors launcher list
- [ ] Bookmarks: user-pinned files and folders, right-click any file to add
- [ ] Resizable via drag handle, min width 160px, max 480px
- [ ] Open/closed state, width, and active tab persisted in `session.json`

---

## 13. Phase 8 - Polish and Installer

**Goal:** Final UX polish, Windows 11 effects, and MSI installer packaging.

### Animations

- [ ] Sidebar open/close: slide animation (150ms ease-out)
- [ ] Launcher to editor: fade transition (200ms)
- [ ] Tab open: fade in (100ms)
- [ ] Toast appear/dismiss: slide up from bottom-right (120ms)

### Toast Notifications

Non-blocking toasts appear in the bottom-right corner and auto-dismiss after 4 seconds. Actions can be embedded (e.g. "Reopen" on crash).

Triggers:
- Neovide process crash
- Theme import success or failure
- File save confirmation (optional, off by default)
- Setup error (Neovide not found)

### Zen Mode

`F11` hides all chrome - title bar, tab bar, status bar, sidebar. Only the Neovide panel fills the screen. Press `F11` again to restore. State is not persisted across restarts.

### Windows Effects

- [ ] Mica backdrop on title bar area on Windows 11 via `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_MAINWINDOW)`
- [ ] Acrylic blur fallback on Windows 10 via `SetWindowCompositionAttribute`
- [ ] Window opacity via `SetLayeredWindowAttributes` - affects entire NeoNote chrome, not the Neovide panel
- [ ] Smooth opacity transition on settings change (animate over 200ms)

### High-DPI

- [ ] Embed DPI-aware manifest via `build.rs` and `winres`
- [ ] Set `Per Monitor V2` awareness: `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`
- [ ] egui handles scaling automatically - verify at 100%, 125%, 150%, 200% scaling

### Installer

- [ ] Bundle `neovide.exe` in installer
- [ ] Generate `.msi` with `cargo-wix`
- [ ] Installer creates Start Menu shortcut and optional Desktop shortcut
- [ ] Optional file associations for `.txt` and `.md` (opt-in during install)
- [ ] Code signing: sign `neonote.exe` and `.msi` with a self-signed cert for development; document steps for EV cert for production release
- [ ] Uninstaller removes all app files but prompts to keep `%APPDATA%\NeoNote\`
- [ ] No auto-update in v1 - document manual update process

### Performance

- [ ] Pre-warm one Neovide process at all times (paused under 200MB free RAM)
- [ ] Theme JSON files parsed once at startup and cached in `ThemeStore`
- [ ] Recent files list loaded once at startup, updated in memory, flushed to disk on change

---

## 14. Data and Persistence

All user data lives exclusively under `%APPDATA%\NeoNote\`. Nothing is written elsewhere.

```text
%APPDATA%\NeoNote\
+-- config.json
+-- session.json
+-- recent_files.json
+-- themes\
    +-- built-in\          <- copied from app assets on first run, never user-modified
    +-- user\              <- user-imported and custom themes
```

### config.json

```json
{
  "active_theme": "catppuccin-mocha",
  "font_family": "JetBrains Mono",
  "font_size": 14,
  "line_height": 1.4,
  "tab_size": 2,
  "word_wrap": false,
  "cursor_trail_enabled": true,
  "cursor_vfx_override": null,
  "startup_mode": "windowed",
  "window_opacity": 100,
  "blur_behind": false,
  "remember_window_geometry": true,
  "restore_last_session": true,
  "show_launcher_on_startup": true,
  "default_open_folder": null,
  "keybindings": {}
}
```

### session.json

```json
{
  "window": { "x": 100, "y": 100, "width": 1280, "height": 800 },
  "sidebar": { "open": true, "width": 220, "active_tab": "explorer" },
  "tabs": [
    { "file_path": "C:/Users/you/notes.md" },
    { "file_path": "C:/Projects/readme.md" }
  ],
  "active_tab_index": 0
}
```

### recent_files.json

```json
[
  { "path": "C:/Users/you/notes.md", "last_opened": "2025-06-13T10:00:00Z" },
  { "path": "C:/Projects/readme.md", "last_opened": "2025-06-12T08:30:00Z" }
]
```

Max 20 entries. Oldest entry is dropped when limit is exceeded. Sorted descending by `last_opened` on read.

---

## 15. Error Handling Contracts

Every module has defined behavior for its failure modes. Silent failures and panics are not acceptable in production paths.

| Module | Failure | Behavior |
|---|---|---|
| Neovide path resolution | Binary not found | Show blocking setup dialog, do not crash |
| HWND discovery | Not found within 5s | Show recoverable toast, clean up process |
| Tier 1 embedding | Render fails after SetParent | Auto-switch to Tier 2 silently |
| Tier 2 fallback | Overlay positioning fails | Show error toast, offer to restart tab |
| RPC connection | Connect timeout | Retry 3 times with 1s backoff, then show status bar warning |
| RPC connection | Drop mid-session | Show `[--]` mode indicator, attempt reconnect every 5s |
| RPC command | NeoVim returns error | Log to stderr, show toast only for user-initiated actions |
| Theme JSON | Malformed file | Skip that theme, log warning, do not crash ThemeStore |
| Theme JSON | `neovim_colorscheme` not installed | Send command anyway, NeoVim will error silently - no crash |
| Config JSON | Missing or corrupt | Reset to defaults, write clean config, show one-time toast |
| Session JSON | Missing or corrupt | Start fresh session, do not restore tabs |
| Neovide crash | Process exits unexpectedly | Show "Neovide crashed - Reopen?" toast, remove tab from state |
| Pre-warm process | Fails to spawn | Disable pre-warming silently, spawn fresh on next new tab |

---

## 16. Built-in Themes

| Theme | Variant | neovim_colorscheme |
|---|---|---|
| Tokyo Night | Dark | `tokyonight` |
| Catppuccin Mocha | Dark | `catppuccin` |
| Catppuccin Latte | Light | `catppuccin` |
| Gruvbox Dark | Dark | `gruvbox` |
| Nord | Dark | `nord` |
| Rose Pine | Dark | `rose-pine` |
| One Dark Pro | Dark | `onedark` |
| Solarized Dark | Dark | `solarized` |
| Solarized Light | Light | `solarized` |

The `neovim_colorscheme` value is sent via `:colorscheme {value}`. If the user does not have the corresponding plugin installed, NeoVim will emit an error that NeoNote ignores gracefully. The wrapper UI theme still applies regardless.

---

## 17. Milestone Summary

| Phase | Deliverable | Risk |
|---|---|---|
| Phase 0 | Repo, module skeletons, assets, manifest | Low |
| Phase 1 | Neovide spawns and embeds with Tier 1 and Tier 2 fallback | High |
| Phase 2 | Frameless chrome, menus, RPC connection, status bar with live mode | Medium |
| Phase 3 | Tab system, pre-warm, HWND switching, dirty-close confirmation | High |
| Phase 4 | Theme JSON schema, ThemeStore, egui visuals mapping, RPC apply | Medium |
| Phase 5 | Theme panel, live preview, import, custom builder | Medium |
| Phase 6 | Settings panel with immediate persistence and RPC apply | Medium |
| Phase 7 | Launcher screen, animated transition, collapsible sidebar | Medium |
| Phase 8 | Animations, toasts, Mica/Acrylic, DPI, MSI installer | Low |

Phase 1 is the critical path. Do not begin Phase 2 until Phase 1 acceptance criteria all pass on a real Windows machine with a discrete GPU.

---

*NeoNote - Rust + egui + windows-rs*
