# NeoNote 1.1.0 — UI Polish Release

## Vision

NeoNote 1.1.0 is a polish release focused on improving perceived quality, smoothness, and native feel.

This release must not introduce major product features.

The goal is:

> NeoNote should feel like a finished native Windows application while remaining lightweight, fast, and focused on note-taking with Vim.

---

# Goals

## Visual

* Windows 11 Mica support
* Theme transition animations
* Tab hover animations
* Dialog open animations
* Settings UI polish

## Editor

* Cursor glide
* Smooth scrolling
* Lightweight cursor trail
* Search match counter

Cursor trail is included, but it is the first feature to disable internally if it threatens performance, complexity, or release quality.

---

# Non Goals

Do not implement:

* Workspace support
* File explorer
* Plugin system
* Markdown preview
* Split panes
* Vault system
* Obsidian-style features
* Heavy particle effects
* GPU-intensive animations
* NeoVide-style particle cursor effects

NeoNote must remain:

* Lightweight
* Native
* Fast
* Focused on notes

---

# Architecture Rules

Keep the existing architecture:

```text
Slint UI
    ↓
AppController
    ↓
Rust Editor State
    ↓
Snapshot Model
    ↓
Slint Rendering
```

Rules:

* Editor logic stays in Rust.
* Slint remains presentation-only.
* No Vim logic inside `.slint`.
* Animation state may exist in snapshots.
* Rust provides logical state and animation hints.
* Slint owns interpolation.

---

# Animation Architecture

Prefer Slint declarative animations over Rust-driven frame timers.

Rust provides:

* logical cursor position
* logical viewport position
* animation hints
* configuration flags

Slint performs:

* cursor interpolation
* scroll interpolation
* opacity animations
* theme transitions
* hover transitions

Do not create a permanent Rust timer running at 60 FPS.

Rust timers are allowed only for:

* one-shot effects
* delayed state clearing
* temporary notifications

Rust timers must stop automatically.

---

# Editor Metrics Contract

Cursor glide and smooth scrolling require pixel coordinates.

Rust must not guess rendered text metrics.

Slint owns visual metrics.

Rust owns logical editor state.

Selection highlight polish follows the same split:

* Rust computes logical selection groups and normalized highlight path data.
* Slint scales and paints the highlight via `Path` using editor text metrics.
* Keep highlights behind text/cursor.
* Do not use viewport fill, longest-line bridging, or vertical overlap hacks to fake connected selections.

---

## Required Metrics

```rust
pub struct EditorMetricsSnapshot {
    pub editor_origin_x: f32,
    pub editor_origin_y: f32,

    pub gutter_width: f32,

    pub content_padding_x: f32,
    pub content_padding_y: f32,

    pub line_height_px: f32,
    pub char_width_px: f32,

    pub cursor_width_px: f32,
}
```

---

## Cursor Target Formula

```text
cursor_target_x =
    editor_origin_x
    + gutter_width
    + content_padding_x
    + cursor_column * char_width_px

cursor_target_y =
    editor_origin_y
    + content_padding_y
    + visible_line_index * line_height_px
```

```text
visible_line_index = cursor_line - viewport_top_line
```

---

## Required Slint Properties

```slint
property <float> editor-origin-x;
property <float> editor-origin-y;

property <float> gutter-width;

property <float> content-padding-x;
property <float> content-padding-y;

property <float> line-height-px;
property <float> char-width-px;

property <int> cursor-line;
property <int> cursor-column;

property <int> viewport-top-line;

property <float> cursor-target-x;
property <float> cursor-target-y;

property <float> visual-cursor-x;
property <float> visual-cursor-y;
```

---

## Monospace Assumption

For NeoNote 1.1.0:

```text
cursor_x = column * char_width_px
```

is acceptable.

Known limitations:

* Emoji
* Full-width Unicode
* Combining marks
* Complex scripts
* Tabs

These may not animate perfectly.

Do not implement full display-column correctness in 1.1.0.

---

# Phase 1 — UI Configuration

Add config flags:

```rust
pub struct AppConfig {
    pub enable_mica: bool,
    pub enable_animations: bool,
    pub enable_cursor_glide: bool,
    pub enable_smooth_scroll: bool,
    pub enable_cursor_trail: bool,
}
```

Defaults:

```text
enable_mica = true
enable_animations = true
enable_cursor_glide = true
enable_smooth_scroll = true
enable_cursor_trail = true
```

Requirements:

* Persist normally
* Old config files continue working
* Missing fields load defaults

---

# Phase 2 — Windows 11 Mica Support

Use:

```toml
raw-window-handle
windows
```

Create:

```text
src/platform/window_effects.rs
```

Add:

```rust
pub fn apply_mica_for_window(
    window: &slint::Window,
    enabled: bool,
) -> anyhow::Result<()>
```

Implementation requirements:

1. Obtain HWND through raw-window-handle.
2. Use DWM APIs through windows-rs.
3. Apply backdrop after window creation.

Use:

```text
DwmSetWindowAttribute
DWMWA_SYSTEMBACKDROP_TYPE
```

Preferred order:

```text
DWMSBT_MAINWINDOW
DWMSBT_TRANSIENTWINDOW
Fallback
```

Windows 10 behavior:

* Do not attempt Mica
* Use `theme_surface` or `theme_background_alt` for title bar and top chrome
* UI must still look intentional

Error rules:

* Never panic
* Never block startup
* Silently fall back

---

# Phase 3 — Animation Infrastructure

Add snapshot fields:

```rust
pub struct AppSnapshot {
    pub enable_animations: bool,
    pub enable_cursor_glide: bool,
    pub enable_smooth_scroll: bool,
    pub enable_cursor_trail: bool,

    pub animation_duration_short_ms: i32,
    pub animation_duration_normal_ms: i32,
    pub animation_duration_long_ms: i32,
}
```

Defaults:

```text
Short  = 80ms
Normal = 120ms
Long   = 180ms
```

When animations are disabled:

```text
All durations = 0ms
```

---

# Phase 4 — Theme Transition

Animate:

* Background
* Surface
* Border
* Text
* Accent colors

Duration:

```text
120–200ms
```

Easing:

```text
ease-out
```

When animations are disabled:

```text
duration = 0ms
```

---

# Phase 5 — Tab & Dialog Polish

## Tab Hover

Animate:

* Background
* Border
* Opacity

Duration:

```text
80–120ms
```

Rules:

* No layout shift
* No height changes
* No large shadows

---

## Dialog Open

Animation:

```text
Opacity: 0 → 1
Scale:   0.985 → 1.0
Y:       +6px → 0px
```

Duration:

```text
120–160ms
```

---

## Dialog Close

Animation:

```text
Opacity: 1 → 0
Scale:   1.0 → 0.985
```

Duration:

```text
100ms
```

---

# Phase 6 — Cursor Glide

## Rust Responsibilities

Rust provides:

```rust
pub cursor_line: i32,
pub cursor_column: i32,
pub cursor_animation_kind: CursorAnimationKind,
```

Rust does not compute cursor pixels.

---

## Slint Responsibilities

Slint computes:

```text
cursor_target_x
cursor_target_y
```

from:

* editor metrics
* logical cursor position
* viewport position

Slint animates:

```text
visual_cursor_x
visual_cursor_y
```

toward target.

---

## Cursor Animation Kind

```rust
pub enum CursorAnimationKind {
    Immediate,
    SmallMove,
    LargeJump,
}
```

Small move examples:

```vim
h
j
k
l
w
b
e
f
%
```

Duration:

```text
80–100ms
```

Large jump examples:

```vim
gg
G
n
N
Ctrl+O
Ctrl+I
```

Duration:

```text
120–140ms
```

Insert mode rule:

```text
Typing must remain immediate.
Disable cursor glide in Insert mode.
```

---

# Phase 7 — Smooth Scrolling

## Rust Responsibilities

Rust provides:

```rust
pub viewport_top_line: i32,
pub scroll_animation_kind: ScrollAnimationKind,
```

---

## Scroll Animation Kind

```rust
pub enum ScrollAnimationKind {
    Immediate,
    SmallMove,
    PageMove,
    LargeJump,
}
```

Mapping:

```text
Immediate = insert typing, settings/font changes, disabled animation
SmallMove = j/k or small scroll movement
PageMove  = Ctrl+D / Ctrl+U / Ctrl+F / Ctrl+B
LargeJump = gg / G / n / N / search jump
```

---

## Slint Responsibilities

Slint computes:

```text
viewport_target_y = viewport_top_line * line_height_px
```

Slint animates visible viewport offset.

Durations:

```text
SmallMove: 80–100ms
PageMove:  120–160ms
LargeJump: 140–180ms
Immediate: 0ms
```

Rules:

* New target replaces old target
* Never queue animations
* Responsiveness wins
* If lag appears, fall back to instant scrolling

---

# Phase 8 — Cursor Trail

## Goal

Add subtle cursor motion history.

This is not a particle system.

---

## Rules

Allowed:

* 2–3 previous cursor positions
* decreasing opacity
* short fade

Not allowed:

* particles
* sparkles
* ripple
* glow
* permanent 60 FPS Rust loop

---

## Config

Cursor trail depends on:

```text
enable_animations = true
enable_cursor_glide = true
enable_cursor_trail = true
```

If any are false:

```text
cursor trail disabled
```

---

## Implementation

Slint should render:

```text
Current cursor
Ghost cursor 1
Ghost cursor 2
Ghost cursor 3
```

Ghost opacity:

```text
Ghost 1: 35%
Ghost 2: 20%
Ghost 3: 10%
```

Fade duration:

```text
100–160ms
```

---

## Fallback Rule

If this cannot be implemented cleanly with Slint animations:

```text
Keep the config flag.
Ship with cursor trail disabled internally.
Do not block 1.1.0 release.
```

---

# Phase 9 — Search Match Counter

Add snapshot fields:

```rust
pub struct SearchMatchSnapshot {
    pub search_match_current: i32,
    pub search_match_total: i32,
    pub search_match_label: String,
}
```

Behavior:

```text
No search:             hidden
Search with matches:   current/total
Search without match:  0/0
```

Must update after:

```vim
/
?
n
N
*
#
```

Tests required:

* no search
* one match
* multiple matches
* n
* N
* zero matches

---

# Phase 10 — Settings Polish

Add toggles:

```text
Visual
────────────────────
[ ] Use Mica
[ ] Enable Animations

Editor
────────────────────
[ ] Cursor Glide
[ ] Smooth Scrolling
[ ] Cursor Trail
```

Polish existing controls:

* Font Size
* Line Height
* Window Opacity

Preferred:

```text
Compact slider controls
```

Acceptable:

```text
- value +
```

with:

* hover states
* focus states
* keyboard navigation
* animation

Settings must:

* Save immediately
* Apply immediately where possible
* Not require restart

---

# Testing

## Visual

Verify:

* Mica works on Windows 11
* Windows 10 fallback works
* Theme transitions work
* Hover transitions work
* Dialog animations work
* Animation disable setting works

## Editor

Verify:

* Cursor glide works
* Cursor trail is subtle and does not lag
* Insert mode remains responsive
* Smooth scrolling works
* Search counter updates correctly

## Performance

Verify:

* Startup speed unchanged
* Memory usage unchanged
* No animation queue buildup
* Holding j/k remains responsive
* No permanent Rust animation loop exists

---

# Suggested Commit Order

```text
feat(config): add ui polish settings

feat(windows): add mica window effect support

feat(ui): add animation infrastructure

feat(theme): add animated theme transitions

feat(ui): add tab hover animations

feat(ui): add dialog open animations

feat(editor): add cursor glide

feat(editor): add smooth scrolling

feat(editor): add cursor trail with fallback-disable behavior

feat(search): add search match counter

feat(settings): polish settings controls

test(search): add search counter coverage
```

---

# Definition of Done

NeoNote 1.1.0 is complete when:

* UI feels noticeably smoother
* Cursor movement feels modern
* Cursor trail exists or is safely disabled behind config
* Scrolling feels modern
* Search counter is correct
* Mica works on supported systems
* Windows 10 fallback looks intentional
* All effects can be disabled
* No permanent animation loop exists
* Startup speed remains unchanged
* Memory usage remains unchanged
* Existing Vim behavior remains unchanged
* NeoNote still feels lightweight and native
