# NeoNote

NeoNote is a native Windows note-taking app built in Rust with a Slint UI, native Windows dialogs, and an in-process Vim-style editor core.

It does not embed or launch Neovim, Neovide, a terminal UI, WebView, Electron, or any browser renderer for normal editing. The note workflow stays inside the NeoNote process.

## Version

Current release target: `1.0.0`

## Highlights

- Native Windows desktop app
- Slint-based UI
- In-process Vim-style editing
- Native file open/save dialogs
- Recent files, session persistence, and themes
- Unicode-aware text editing and file loading
- Built-in Windows NSIS installer

## Current behavior

NeoNote currently supports:

- Multiple open note tabs
- Drag-reorder tabs
- New, Open, Save, and Save As
- `.txt` as the default note type
- Built-in themes
- Normal, insert, visual, command, search, registers, undo/redo, and a large set of Vim motions/operators

User-facing config and app data live under:

```text
%APPDATA%\NeoNote\
```

## Requirements

For local development:

- Windows
- Rust toolchain
- Visual Studio C++ build tools / MSVC target

For installer builds:

- NSIS with `makensis` available on `PATH`

## Run from source

```powershell
cargo run
```

You can also pass files to open:

```powershell
cargo run -- path\to\note.txt
```

Supported utility flags:

```text
--register-file-associations
--unregister-file-associations
```

## Test

```powershell
cargo test
```

## Build release binary

```powershell
cargo build --release
```

Release output:

```text
target\release\neonote.exe
```

## Build the NSIS installer

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1
```

Installer output:

```text
target\installer\NeoNote-Setup-1.0.0.exe
```

The installer currently:

- installs `NeoNote.exe` to `$LOCALAPPDATA\Programs\NeoNote`
- ships built-in theme JSON files under `assets\themes\built-in`
- creates Start Menu and desktop shortcuts
- writes an uninstaller

## Project layout

```text
src\
  app.rs           controller and app state
  notes.rs         text buffer and current Vim/editor behavior
  vim\             extracted pure Vim modules and key normalization
  persistence\     config, session, recent files
  platform\        Windows helpers
  theme\           theme loading and schema
  main.rs          Slint bootstrap

ui\
  app-window.slint main Slint window

installer\
  neonote.nsi      NSIS installer script

scripts\
  build-installer.ps1
```

## Architecture

The active path is:

```text
Slint input/callbacks
  -> Rust app controller
  -> editor/document state
  -> Slint models and properties
```

Slint owns presentation. Rust owns behavior.

The Vim/editor layer must remain pure app logic:

- no external editor process
- no RPC backend
- no embedded terminal
- no browser-based editor surface

## Notes for contributors

- Read [docs/neonote-implementation-plan.md](docs/neonote-implementation-plan.md) before making architectural changes.
- Follow the repo rules in [AGENTS.md](AGENTS.md).
- Keep normal editing in-process and Rust-native.
- Keep all user-facing data under `%APPDATA%\NeoNote\`.
- Run `cargo test` before commits and pushes.
