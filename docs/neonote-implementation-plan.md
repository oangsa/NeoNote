# NeoNote - Implementation Plan

## Summary

NeoNote is a native Windows note-taking app built in Rust with a Slint UI, `windows-rs` platform helpers, `serde` persistence, and native Windows file dialogs.

The editor is an in-process Vim-style editing layer implemented in Rust. NeoNote does not spawn, embed, wrap, connect to, or render Neovide or Neovim for normal editing.

The current codebase may still contain legacy egui-era structure. Future UI work should migrate the application to Slint and should not add new egui/eframe UI surfaces.

## Product Rules

- Native Rust application with Slint as the UI toolkit.
- No WebView, Electron, Tauri, Chromium, or browser-rendered UI.
- No terminal UI.
- No `neovide.exe`, `nvim.exe`, RPC editor backend, external editor process, or HWND editor embedding in the normal note workflow.
- New File, Open File, Save, Save As, Recent Files, launcher note actions, and ordinary editing use the in-process editor state.
- `.txt` is the default note extension and default save name.
- All user-facing data lives under `%APPDATA%\NeoNote\`.

## Target Architecture

```text
NeoNote process
  |
  +-- Slint UI
  |     +-- app window
  |     +-- custom title/menu area
  |     +-- launcher
  |     +-- editor view
  |     +-- status bar
  |     +-- theme panel
  |     +-- settings panel
  |
  +-- Rust app controller
  |     +-- file dialogs and file commands
  |     +-- recent files
  |     +-- config/session persistence
  |     +-- theme store
  |     +-- Slint model/property updates
  |
  +-- in-process editor core
        +-- text buffer
        +-- Vim mode state
        +-- cursor / selection
        +-- motions
        +-- operators
        +-- command mode
        +-- undo / repeat / registers
```

Slint owns presentation. Rust owns behavior.

Slint callbacks should be narrow:

```text
on-new-file        -> controller.new_file()
on-open-file       -> controller.open_file_dialog()
on-save            -> controller.save()
on-editor-key      -> controller.handle_editor_key(...)
on-theme-selected  -> controller.apply_theme(...)
```

## Active And Target Modules

Current modules to preserve conceptually:

```text
src/
  main.rs
  app.rs                  <- migrate to Slint controller/bootstrap
  notes.rs                <- current text buffer + first Vim-layer slice
  persistence/
  platform/
  theme/
```

Target UI/module layout:

```text
ui/
  app-window.slint
  components/
    title-bar.slint
    menu-bar.slint
    launcher.slint
    editor-view.slint
    status-bar.slint
    theme-panel.slint
    settings-panel.slint

src/
  main.rs                 <- Slint bootstrap
  app.rs                  <- controller/state orchestration
  notes.rs                <- temporary editor model until src/vim extraction
  vim/
    mod.rs
    key.rs
    state.rs
    buffer.rs
    position.rs
    motion.rs
    operator.rs
    text_object.rs
    visual.rs
    registers.rs
    undo.rs
    repeat.rs
    search.rs
    marks.rs
    jump_list.rs
    change_list.rs
    command.rs
    substitute.rs
    macros.rs
    surround.rs
    splits.rs
  slint_bridge/
    mod.rs                <- generated-binding helpers and model adapters
  persistence/
  platform/
  theme/
```

Retired modules must not be reintroduced for normal editing:

- `src/embed/`
- `src/rpc/`
- `src/nvim/`
- Neovide-backed `src/tabs/`
- editor RPC color application

## UI Toolkit Direction

### Slint Is The Target

All new UI work should be implemented in Slint.

Use Slint for:

- window layout
- title/menu/status/launcher/settings/theme surfaces
- editor surface rendering
- panels, dialogs, popups, lists, and controls
- theme token binding

Keep Rust responsible for:

- Vim/editor logic
- file I/O
- native file dialogs
- persistence
- platform effects
- theme loading and validation
- transforming editor state into Slint-friendly models

### egui / eframe Migration

Existing egui code is legacy. The migration should remove it in controlled phases:

1. Add Slint dependencies and build integration.
2. Create a Slint shell that can open and render the main window.
3. Move launcher, menu, title/status bars, theme panel, and settings panel to Slint.
4. Move the editor view to Slint while preserving Rust-owned Vim state.
5. Remove egui/eframe dependencies once Slint reaches feature parity.

Do not add new egui features unless they are tiny temporary shims needed to keep the app compiling during migration.

## Editor Model

### `NoteDocument`

Current owner of:

- text content
- optional file path
- dirty flag
- open/launcher state
- Vim mode
- cursor line and column
- count prefix
- pending command/operator state

This may later be split into `TextBuffer` plus `VimState`, but the same rules apply.

### Future `TextBuffer`

All text edits should flow through controlled APIs:

```rust
insert_text(pos, text)
delete_range(range)
replace_range(range, text)
get_range(range)
line_range(line)
word_range_at(pos)
cursor()
set_cursor(pos)
selection()
set_selection(selection)
```

### Future `VimState`

Pure editor state only:

```text
mode
pending_operator
pending_motion
count
register
visual
command_line
search
registers
marks
jumplist
changelist
repeat
macros
surround
```

`VimState` must not render UI, spawn processes, open dialogs, write files, call RPC, or call platform APIs directly.

## Core Editing Rules

- Normal mode is the default for new and opened documents.
- Insert mode mutates the project-owned text buffer directly.
- Do not let a Slint text input widget own the core editor caret if that conflicts with Vim cursor ownership.
- Counts are parsed before commands.
- Leading `0` is a line-start motion when no count is pending.
- Cursor movement clamps at file and line boundaries.
- Internal text ranges are half-open and normalized before mutation.
- Visual selections may have reversed anchor/cursor; mutation ranges must still be normalized.
- File operations are app-level commands. Vim command mode may request file actions, but the app performs filesystem work.
- Scroll commands affect viewport state, not buffer text.
- When `sync_clipboard` is enabled, the app controller syncs the Vim unnamed register with the system clipboard. The Vim layer must still remain platform-free.

## Theme Model

Themes style NeoNote's Slint UI and Vim mode colors. They do not apply external editor colorschemes.

ThemeStore remains the Rust authority for:

- loading built-in themes
- loading user themes from `%APPDATA%\NeoNote\themes\user\`
- skipping malformed JSON
- committed active theme
- preview theme
- persistence of `active_theme` in `config.json`

Slint receives resolved theme tokens through properties/globals/models.

### Theme JSON

```json
{
  "name": "Tokyo Night",
  "variant": "dark",
  "author": "NeoNote",
  "colors": {
    "background": "#111318",
    "background_alt": "#181b22",
    "surface": "#222631",
    "border": "#343a46",
    "text": "#e6e8ef",
    "text_muted": "#9aa3b2",
    "accent_primary": "#6ea8fe",
    "accent_secondary": "#8fd7c7",
    "success": "#8bd17c",
    "warning": "#f5c56b",
    "error": "#ff7b86",
    "cursor": "#f2d16b"
  },
  "vim_modes": {
    "normal": "#6ea8fe",
    "insert": "#8bd17c",
    "visual": "#b38cff",
    "command": "#f5c56b",
    "replace": "#ff7b86"
  }
}
```

Old theme files may contain obsolete external-editor fields. The active Rust schema should ignore them and only use NeoNote UI colors and Vim mode colors.

## Data And Persistence

All user-facing data lives under:

```text
%APPDATA%\NeoNote\
  config.json
  session.json
  recent_files.json
  themes\
    user\
```

### `config.json`

```json
{
  "active_theme": "tokyonight",
  "font_family": "JetBrains Mono",
  "font_size": 14,
  "line_height": 1.4,
  "tab_size": 4,
  "word_wrap": false,
  "sync_clipboard": true,
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

### `recent_files.json`

```json
[
  {
    "path": "C:/Users/you/notes/today.txt",
    "last_opened": "1781200000"
  }
]
```

Recent files are capped at 20 and sorted descending by `last_opened`.

## Milestones

### Phase 0 - Stabilize Current Native Editor Baseline

Status: mostly complete.

Scope:

- in-process document model
- native file dialogs
- recent files
- theme loading
- settings shell
- launcher/editor routing
- no external editor processes

Acceptance:

- `cargo test` passes.
- App opens without spawning external editor processes.
- Launcher can create/open a note.
- Save/Save As use `.txt` defaults.

### Phase 1 - Slint Foundation

Status: first shell implemented.

Scope:

- Add Slint dependencies and build integration.
- Add initial `.slint` app window.
- Bootstrap Slint from `main.rs`.
- Create Rust controller object for app state.
- Wire basic callbacks: New, Open, Save, Save As, theme panel open, settings panel open.
- Preserve current file workflows.

Acceptance:

- App launches through Slint.
- No egui/eframe UI is required for the main window.
- Existing persistence and file workflow tests still pass.
- No `neovide.exe` or `nvim.exe` starts.

### Phase 2 - Slint Shell Feature Parity

Scope:

- Title/menu/status bars in Slint.
- Launcher in Slint.
- Theme panel in Slint.
- Settings panel in Slint.
- Toasts/notifications in Slint.
- ThemeStore tokens applied to Slint properties.

Acceptance:

- Slint shell matches current app workflows.
- Theme preview/apply works.
- Recent files render and update.
- Status bar shows native document state.

### Phase 3 - Slint Editor View

Scope:

- Render text buffer, line numbers, cursor, selections, mode indicators, and command line in Slint.
- Forward key events from Slint to Rust key normalization.
- Keep Rust as the owner of buffer and cursor state.
- Support focus handoff after New/Open and insert-entry commands.

Acceptance:

- Typing after `i`, `a`, `o`, or `O` works without clicking.
- `Esc`, `hjkl`, counts, word motions, line motions, `dd`, and `x` work through Slint.
- No Slint text input widget owns the core Vim caret.

Current implementation note:

- The Slint editor renders from an `EditorLine` model, one row per buffer line.
- Rust owns text, cursor line/column, Vim mode, and cursor row metadata (`cursor_prefix`, `cursor_cell`, `cursor_suffix`, `cursor_block`).
- Slint renders each row's visible text through one full-line `Text` item. The cursor is a background rectangle behind that text, positioned from the measured cursor-prefix width and sized from the measured cursor-cell width.
- For column 0, the cursor uses the editor text origin directly instead of the empty prefix measurement. A small `cursor-x-adjust` compensates for glyph side-bearing.
- Do not render the focused row as visible prefix/cursor/suffix text segments; that creates a separate text layout path and can misalign the focused line.
- The editor viewport is scrollable through Slint `ScrollView`. Mouse clicks and drags route through narrow controller callbacks that update Rust-owned cursor and selection state.

### Phase 4 - Extract Vim Core

Scope:

- Create `src/vim/`.
- Move key normalization out of app/controller code.
- Split `NoteDocument` into text-buffer APIs and `VimState`.
- Define canonical `CursorPos` and `TextRange`.
- Preserve current behavior while improving testability.

Acceptance:

- Existing tests still pass.
- Slint controller routes normalized keys into the Vim layer.
- Vim logic has no Slint dependencies except optional key conversion at the boundary.

### Phase 5 - Minimal Vim Navigation Completion

Scope:

- Normal mode
- Insert mode
- `Esc`
- direct insert text, enter, backspace, delete
- `h j k l`
- `w b e`
- `W B E`
- `0 ^ $`
- `gg G {n}G`
- `i I a A o O`
- count prefixes
- `dd`
- `x`

Acceptance:

- User can create/open a `.txt` note.
- User can press `i`, type immediately, press `Esc`, and navigate without clicking.
- Motions clamp safely at file boundaries.
- Unit tests cover counts, word motions, line motions, insert mutation, `dd`, and `hjkl`.

### Phase 6 - Operators And Text Objects

Scope:

- Operators: `d`, `y`, `c`, `>`, `<`, `=`, `gu`, `gU`, `~`
- Doubled operators: `dd`, `yy`, `cc`
- Operator + motion: `dw`, `d$`, `caw`, `ygg`
- Counts: `3dw`, `d3w`
- Text objects: `iw`, `aw`, quotes, brackets, paragraphs, lines

Acceptance:

- Operators compose with motions and text objects.
- Range calculation is unit-tested.
- Buffer mutation is undo-transaction-ready.

### Phase 7 - Undo, Redo, Registers, Repeat

Scope:

- Edit transactions
- `u`
- `Ctrl+r`
- `.`
- unnamed register
- yank register `0`
- named registers `a-z`
- clipboard register `+`
- black-hole register `_`
- register prefixes such as `"ayy`, `"ap`, `"_dd`

Acceptance:

- Common edits are undoable and redoable.
- Yank/delete/change populate expected registers.
- Dot repeat works for common edits.

### Phase 8 - Visual Mode And Search

Scope:

- `v`
- `V`
- visual char and line selection
- visual operators
- `o`
- `gv`
- `/`, `?`
- `n`, `N`
- `*`, `#`
- `:noh`
- search highlights
- marks and jumplist basics

Acceptance:

- Visual selections render from Vim state in Slint.
- Operators work on visual selection.
- Search navigation updates cursor and highlights matches.

### Phase 9 - Command Mode

Scope:

- command-line state
- `:w`
- `:w {path}`
- `:q`
- `:q!`
- `:wq`
- `ZZ`
- `:e {path}`
- `:enew`
- `:{n}`
- `:/pattern`
- `:s/foo/bar/`
- `:s/foo/bar/g`
- `:%s/foo/bar/g`
- `:%s/foo/bar/gc`

Acceptance:

- Command parser is unit-tested.
- File commands emit app-level actions.
- Substitute works for line and whole-file ranges.

### Phase 10 - Advanced Motions And Viewport

Scope:

- `f`, `F`, `t`, `T`, `;`, `,`
- `%`
- `{`, `}`
- `(`, `)`
- `Ctrl+d`, `Ctrl+u`
- `Ctrl+f`, `Ctrl+b`
- `Ctrl+e`, `Ctrl+y`
- `zz`, `zt`, `zb`

Acceptance:

- Character search repeats correctly.
- Bracket matching handles `()`, `[]`, `{}`, `<>`.
- Viewport commands are separated from buffer mutation.

### Phase 11 - Macros, Changelist, Visual Block

Scope:

- `q{a-z}`
- `q`
- `@{a-z}`
- `@@`
- `g;`
- `g,`
- `Ctrl+v`
- visual block selection
- visual block delete/yank/change
- block insert

Acceptance:

- Macro playback is deterministic and guarded against recursion.
- Changelist navigation works for edits.
- Visual block supports virtual columns and pads short lines when needed.

### Phase 12 - Surround

Scope:

- `ys{motion}{char}`
- `ysiw"`
- `yss(`
- `yss)`
- `ys$"`
- `ysiw<div>`
- `ds"`
- `ds(`
- `dst`
- `cs"'`
- `cs({`
- `cs(}`
- `cst<span>`
- visual `S"`

Acceptance:

- Required add/delete/change surround commands work.
- Surround changes are undoable.
- Surround parser is unit-tested.

### Phase 13 - Splits

Scope:

- `Ctrl+w s`
- `Ctrl+w v`
- `Ctrl+w h/j/k/l`
- `Ctrl+w c`
- `Ctrl+w o`

Rules:

- Vim state emits split commands.
- App owns split tree, panes, active pane, and buffer IDs.
- Panes own Vim state and reference buffers.

Acceptance:

- Split commands create, close, and focus panes.
- Multiple panes can reference the same buffer.

### Phase 14 - Remove Legacy egui / eframe

Scope:

- Remove egui-specific UI modules after Slint parity.
- Remove `egui` and `eframe` dependencies.
- Replace egui color conversions with Slint-compatible color/token adapters.
- Keep only reusable non-UI Rust logic.

Acceptance:

- App builds and runs on Slint only.
- `cargo test` passes.
- No `egui`/`eframe` dependency remains unless explicitly retained for non-UI tooling with documented justification.

## Testing Strategy

Run `cargo test` after each phase and before commits.

Unit-test pure editor logic:

- key normalization
- count parsing
- cursor clamping
- motion resolver
- text object resolver
- operator range calculation
- insert/delete/replace buffer mutation
- undo transactions
- registers
- dot repeat
- visual range normalization
- search
- command parser
- substitute engine
- surround parser
- macro playback

Unit-test UI/controller boundaries where practical:

- Slint callback methods call the correct controller command.
- Controller updates Slint models/properties after document changes.
- ThemeStore maps active/preview theme tokens into Slint-facing state.

Add golden tests for editing fixtures:

```text
name: change_inner_word
input: hello world
keys: ciwtest<Esc>
output: test world
cursor: [0, 3]
mode: Normal
```

Manual checks:

- Launch app and verify no `neovide.exe` or `nvim.exe` process starts during New/Open/Edit.
- New File creates an untitled `.txt` note.
- Open File and Save As default to `.txt`.
- Typing after `i`, `a`, `o`, or `O` works without clicking.
- `Esc`, `hjkl`, counts, word motions, line motions, `dd`, and `x` work.
- Status bar shows mode, file title/path, cursor line/column, and document stats.
- Theme switching affects Slint wrapper/editor UI and mode colors.

## Migration Notes

The project previously explored Neovide HWND embedding, direct embedded Neovim rendering, and egui UI. Those approaches are retired for future work.

Do not add new dependencies or modules for:

- `nvim-rs`
- Neovim msgpack RPC
- Neovide process management
- HWND editor embedding
- terminal rendering
- browser/WebView rendering
- new egui/eframe UI surfaces

Old theme files may contain obsolete fields from those explorations. The active Rust theme schema should ignore external-editor fields and only use NeoNote UI colors and Vim mode colors.
