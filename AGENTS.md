# AGENTS.md

## Project

NeoNote is a native Windows note-taking application built in Rust with a Slint UI, `windows-rs` platform helpers, and native Windows file dialogs.

NeoNote's editor is an in-process Vim-style editor implemented in Rust. It does not wrap, spawn, embed, connect to, or render Neovide or Neovim for normal editing.

Core product rules:

- Use Slint for the application UI.
- No WebView, Electron, Tauri, Chromium, browser rendering, or terminal UI.
- No `neovide.exe`, `nvim.exe`, external editor process, RPC editor backend, or HWND editor embedding in the normal note workflow.
- New File, Open File, Save, Save As, Recent Files, launcher note actions, and ordinary editing must use the in-process editor state.
- `.txt` is the default note file type and default save name.
- All user-facing config and data lives under `%APPDATA%\NeoNote\`.

## Implementation Plan

The canonical implementation plan is:

```text
docs/neonote-implementation-plan.md
```

Read it before starting any task. It defines the active architecture, Slint migration path, editor rules, milestones, persistence rules, and test expectations.

Do not make architectural decisions that contradict the plan. If the architecture genuinely needs to change, update the plan first in the same task.

## Current Architecture

The target app path is:

```text
Slint input/callbacks
  -> Rust app controller
  -> NoteDocument and future src/vim pure editor modules
  -> Slint models/properties for buffer, cursor, status, panels
```

Current important modules:

- `src/app.rs` - current app shell and editor routing; migrate from egui to a Rust controller behind Slint callbacks
- `src/notes.rs` - current text buffer and first Vim-layer slice
- `src/ui/` - legacy Rust UI widgets; retire during the Slint migration
- `src/theme/` - theme schema and loading; migrate visual mapping to Slint global properties/tokens
- `src/persistence/` - `%APPDATA%\NeoNote\` paths, config, session, recent files
- `src/platform/` - native Windows platform helpers

Future Vim logic should be extracted into `src/vim/` as pure editor modules: key normalization, state, motions, operators, text objects, visual mode, registers, undo, repeat, search, command mode, macros, surround, and split commands.

Future Slint UI files should live under `ui/` or `src/ui_slint/` according to the chosen build setup, with generated bindings kept behind small Rust controller APIs.

## Editor Rules

- The Vim layer is pure logic. It must not render UI, spawn processes, call RPC, open dialogs, or write files directly.
- File operations are app-level commands. The editor may request save/open/new/quit actions, but the app performs filesystem and dialog work.
- All buffer mutations go through controlled text-buffer APIs.
- Slint text input widgets must not become the editor engine if their internal caret conflicts with Vim cursor ownership.
- Normal mode is the default for new and opened documents.
- Counts, motions, operators, visual ranges, undo, registers, and repeat must be unit-tested as pure logic.
- Motions must clamp safely at file and line boundaries.
- Internal text ranges should be normalized before mutation.

## Slint UI Rules

- Use Slint components, properties, callbacks, and models for UI state.
- Keep business/editor logic in Rust, not in `.slint` files.
- Slint should render the editor surface from Rust-owned document state.
- UI callbacks should call narrow Rust controller methods.
- Theme tokens should be exposed to Slint through a single theme state/model, not scattered constants.
- Native file dialogs remain Rust-side through the selected dialog crate.
- Avoid UI thread blocking; expensive work should happen in Rust tasks/helpers and update Slint state afterward.

## Skills

Domain-specific knowledge is documented as skills under `docs/skills/`. Each skill captures patterns, gotchas, and reusable knowledge for a specific area of the codebase.

When you learn something new about this project, write it as a skill file immediately.

Examples:

```text
docs/skills/skill-vim-layer.md
docs/skills/skill-slint-ui.md
docs/skills/skill-theme-engine.md
docs/skills/skill-persistence.md
docs/skills/skill-windows-effects.md
```

Skill file naming:

```text
docs/skills/skill-{topic}.md
```

Use lowercase kebab-case for `{topic}`.

## Git Workflow

### Commit Format

```text
<type>(<scope>): <short description>
```

### Types

- `feat` - new feature
- `fix` - bug fix
- `chore` - setup, config, dependencies
- `style` - styling only
- `refactor` - code restructure, no behavior change
- `docs` - documentation
- `test` - test-only changes

### Examples

```text
chore(ui): add slint runtime and build integration
refactor(app): move editor routing behind Slint controller
feat(vim): add count-aware word motions
feat(editor): render native buffer with Vim cursor
feat(command): add basic ex command parser
feat(theme): expose theme tokens to Slint
fix(vim): clamp cursor after deleting final line
fix(editor): keep focus after entering insert mode
docs(plan): align roadmap with Slint and in-process Vim layer
test(vim): add golden tests for operator motions
```

### Branch Strategy

```text
main      <- production only, never commit directly
dev       <- integration branch
feat/xxx  <- feature branches, branched off dev
fix/xxx   <- bug fix branches, branched off dev
```

### Workflow

Start a new feature:

```bash
git checkout dev
git pull origin dev
git checkout -b feat/your-feature-name
```

Done, merge back to dev:

```bash
git checkout dev
git merge feat/your-feature-name
git push origin dev
```

When dev is stable and tested, merge to main:

```bash
git checkout main
git merge dev
git push origin main
```

## Rules

- Read `docs/neonote-implementation-plan.md` before starting any task.
- Follow the plan milestones in order unless the user explicitly asks for a different narrow fix.
- Migrate UI work toward Slint; do not add new egui/eframe UI surfaces.
- Keep the normal editor path in-process and Rust-native.
- Do not reintroduce Neovide, Neovim RPC, HWND editor embedding, terminal editor rendering, WebView, Electron, Tauri, or browser-based rendering.
- Keep all user-facing config and data under `%APPDATA%\NeoNote\`.
- Write new reusable discoveries to `docs/skills/skill-{topic}.md` immediately.
- After implementing a feature or fix, run the relevant tests and commit the completed work once tests pass, unless the user explicitly says not to commit.
- Test before every push. Do not push broken builds to `dev`. Never push directly to `main`.
- Commit messages must follow the format above. No freeform commit messages.
- Do not revert unrelated user changes in a dirty worktree.
