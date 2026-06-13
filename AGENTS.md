# AGENTS.md

## Project

NeoNote is a native Windows GUI application that wraps Neovide with a launcher screen, tab system, theme engine, and settings panel. It is built in Rust using egui and windows-rs. No WebView. No terminal interaction required by the end user.

## Implementation Plan

The full implementation plan is located at:

```
docs/neonote-implementation-plan.md
```

Read it before starting any task. It defines all 8 phases, component responsibilities, data schemas, and the full tech stack. Do not make architectural decisions that contradict it without first updating the plan document.

## Skills

Domain-specific knowledge is documented as skills under `docs/skills/`. Each skill file captures patterns, gotchas, and reusable knowledge for a specific area of the codebase.

When you learn something new — a Rust pattern, a Win32 quirk, a Neovide behavior, an egui limitation — write it as a skill file immediately. Do not wait until later.

### Naming Convention

Skill files follow the pattern:

```
docs/skills/skill-{topic}.md
```

Use lowercase kebab-case for the topic name. Examples:

```
skill-hwnd-embedding.md
skill-egui-layout.md
skill-tab-lifecycle.md
```

---

## Git Workflow

### Commit Format

```
<type>(<scope>): <short description>
```

### Types

- feat — new feature
- fix — bug fix
- chore — setup, config, dependencies
- style — CSS/styling only
- refactor — code restructure, no behavior change
- docs — documentation

### Examples

```
chore(setup): init Rust project with egui and windows-rs
chore(deps): add nvim-rs tokio serde rfd dependencies
feat(embed): spawn Neovide process and reparent HWND into panel
feat(titlebar): add custom frameless title bar with window controls
feat(tabs): add tab bar with show/hide HWND switching
feat(theme): add theme JSON loader and egui visuals mapping
feat(status): add status bar reading VIM mode from nvim-rs RPC
fix(embed): fix Neovide HWND not found on slow machines
fix(tabs): fix modified indicator not clearing after save
refactor(theme): extract color parsing into ThemeStore
docs(skills): add skill-hwnd-embedding for SetParent patterns
```

### Branch Strategy

```
main      <- production only, never commit directly
dev       <- integration branch, all features merge here first
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

---

## Rules

- Read `docs/neonote-implementation-plan.md` before starting any task.
- Follow phases in order. Phase 1 (HWND embedding) must be solid before building anything on top of it.
- When you learn something new, write a skill file in `docs/skills/skill-{topic}.md` immediately.
- Test before every push. Do not push broken builds to dev. Never push directly to main.
- Do not introduce WebView, Electron, or any browser-based rendering at any point.
- All user-facing config and data lives under `%APPDATA%\NeoNote\`. Do not scatter files elsewhere.
- Commit messages must follow the format above. No freeform commit messages.
