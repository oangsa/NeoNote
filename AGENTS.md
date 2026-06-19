# AGENTS.md

## Project

NeoNote is a native Windows note-taking application built in Rust with a Slint UI, `windows-rs` platform helpers, and native Windows file dialogs. It has an in-process Vim-style editor implemented in Rust — no Neovide, Neovim, or external editor process.

## Build & Test

```bash
cargo build        # dev build (Slint UI compiled via build.rs)
cargo build --release
cargo test         # all unit tests (~100 tests across notes.rs, app.rs, vim/key.rs)
cargo test <name>  # single test by name
cargo test <name> -- --nocapture  # single test with eprintln/dbg output visible
```

`build.rs` compiles `ui/app-window.slint` via `slint_build::compile` and embeds a Windows manifest via `winres`. If any Slint property, struct, or callback is wrong, the build fails at compile time — not at runtime.

The NSIS installer requires `makensis` on PATH:
```bash
powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1
```

## Architecture

```
ui/app-window.slint          ← single Slint UI file (all structs, properties, callbacks)
src/main.rs                  ← app entry, bridges Slint <-> Rust via apply_snapshot() and apply_editor_snapshot()
src/app.rs                   ← AppController, snapshot model, all app logic and callback wiring
src/notes.rs                 ← Pane, TextBuffer, Vim mode logic, text buffer (~7500 lines, bulk of editor logic)
src/vim/key.rs               ← key normalization (normal/insert/command modes)
src/vim/surround.rs          ← Vim surround operations
src/startup.rs               ← CLI arg parsing (files, --register-file-associations)
src/persistence/             ← %APPDATA%\NeoNote\ paths, config, session, recent files
src/theme/                   ← theme schema, loading, built-in themes from assets/themes/built-in/
src/platform/                ← Windows helpers: IPC, clipboard, fonts, file_association, window_effects
src/i18n.rs                  ← UI string localization (AppLanguage enum)
docs/neonote-implementation-plan.md  ← canonical architecture plan (read before any task)
docs/skills/                 ← domain-specific skill files
```

`src/ui/` contains legacy egui widgets — ignore it. All active UI work is in Slint.

## Implementation Plan

Read `docs/neonote-implementation-plan.md` before starting any task. Do not contradict it. If the architecture changes, update the plan in the same task.

## Slint ↔ Rust Data Flow

All UI state flows through the snapshot model:

1. `AppController::snapshot()` in `src/app.rs` builds an `AppSnapshot`
2. `apply_snapshot()` / `apply_editor_snapshot()` in `src/main.rs` maps it to Slint types
3. `ui/app-window.slint` renders from properties and models

When adding a new Slint field:
- Add to the Rust struct (e.g., `EditorLineSnapshot`, `SelectionHighlightSnapshot`)
- Add to `AppSnapshot`
- Add to the bridge mapping in `src/main.rs` (both `apply_snapshot` and/or `apply_editor_snapshot`)
- Add to the Slint `export struct`

Slint uses **dashes** (`full-line-width`), Rust uses **underscores** (`full_line_width`). Generated bindings convert automatically.

## Slint Quirks

- **Single .slint file**: `ui/app-window.slint` contains everything — structs, properties, callbacks, and all UI.
- **Property timing**: `root.editor-content-width` depends on `editor-scroll.width` and evaluates to `0` at layout init. Use `parent.width` inside the ScrollView for lazy resolution at render time.
- **Named elements**: `:=` named elements (e.g., `char-width-measure := Text { ... }`) are scoped to their parent and referenced by name.
- **Overlay layering**: Children render back-to-front. Selection overlay must be before the text `VerticalLayout` in the ScrollView.
- **ScrollView content**: The ScrollView contains a `VerticalLayout` (editor lines), measurement Text elements, and the cursor overlay. Selection highlights are rendered as a separate `for` loop before the VerticalLayout.

## Editor & Vim Rules

- The Vim layer is pure logic — no UI rendering, no file I/O, no spawning processes.
- Normal mode is the default for new and opened documents.
- Motions clamp safely at file and line boundaries.
- All buffer mutations go through controlled text-buffer APIs.
- Counts, motions, operators, visual ranges, undo, registers, and repeat must be unit-tested as pure logic.
- `notes.rs` is the core file (~7500 lines). `Pane` holds per-document cursor/vim state; `TextBuffer` holds content, registers, undo/redo, search state.
- `enable_cursor_smear` is force-disabled (`normalize_disabled_features` in config) — it's a planned feature not yet active.

## Config & Persistence

- All user config and data lives under `%APPDATA%\NeoNote\`.
- Config struct: `src/persistence/config.rs` (`AppConfig` with serde defaults and `#[serde(default)]` for forward-compat).
- Session state: `src/persistence/session.rs` — saves open files, cursor positions, unsaved content for dirty buffers.
- Recent files: `src/persistence/recent_files.rs`.
- IPC single-instance: `src/platform/ipc.rs` — second launch forwards files to existing instance via named pipe.

## Git Workflow

Commit format: `<type>(<scope>): <short description>`

Types: `feat`, `fix`, `chore`, `style`, `refactor`, `docs`, `test`

Branch strategy:
- `main` — production only, never commit directly
- `dev` — integration branch
- `feat/xxx`, `fix/xxx` — feature branches off dev

## Rules

- Read `docs/neonote-implementation-plan.md` before any task.
- Migrate UI work toward Slint; do not add new egui/eframe surfaces.
- Keep the editor path in-process and Rust-native.
- Write new discoveries to `docs/skills/skill-{topic}.md` immediately.
- After implementing, run tests and commit once tests pass (unless told not to).
- Test before every push. Do not push broken builds to `dev`.
- Commit messages must follow the format above. No freeform messages.
- Do not revert unrelated user changes in a dirty worktree.
