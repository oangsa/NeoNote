# Skill: Cursor Animation (blink + glide + trail)

Captures the rules and gotchas for the three cursor visual effects in NeoNote:
blinking, glide, and trail. All three are driven from Rust-owned editor state
and rendered by a single floating Slint overlay rectangle plus per-line ghost
rectangles.

## Architecture

- **Floating overlay** (`ui/app-window.slint`): one `Rectangle` positioned from
  `cursor-prefix-measure.preferred-width` and `cursor-line * editor-line-height`.
  It carries `animate x`, `animate y`, and `animate opacity`.
- **Ghost trail** (`ui/app-window.slint`): `for ghost-col in line.trail-columns`
  renders up to 3 faint rectangles behind the active cursor.
- **Rust controller** (`src/app.rs`): owns `cursor_trail: Vec<(usize, usize)>`,
  `cursor_animation_kind`, and `scroll_animation_kind`. The snapshot exposes
  these plus `cursor_insert_mode` and duration values to Slint.
- **Blink timer** (`src/main.rs`): a single `slint::Timer` at 500 ms (2 Hz) that
  toggles `cursor-blink-visible` when the editor has been idle for >1 s. This is
  the sanctioned exception to the "no permanent Rust timer" rule in the
  implementation plan — 2 Hz is low-frequency and only toggles a bool property.

## Blink

- The overlay opacity is `has-document && (!enable-cursor-blink || cursor-blink-visible) ? 1.0 : 0.0`.
- `animate opacity { duration: 100ms; easing: ease-out; }` gives a soft fade.
- On every `editor-key` and `editor-pointer-event`, stamp
  `last_editor_activity = Instant::now()` and force
  `cursor-blink-visible = true` so the caret is solid immediately after input.
- The timer keeps the caret solid for the first 1 s of idleness, then toggles.
- `enable_cursor_blink` config gates the whole effect; when off, opacity stays
  1.0 unconditionally.

## Glide

- `CursorAnimationKind` (`Immediate`, `SmallMove`, `LargeJump`) is computed from
  line/column diffs in `handle_editor_key`.
- **Insert mode is NOT forced to Immediate.** Insert uses the same diff logic as
  Normal mode, but the Slint duration expression selects `anim-duration-insert`
  (50 ms) when `cursor-insert-mode == true`, which is snappy enough that fast
  typing does not feel laggy. Normal mode uses `anim-duration-short` (80 ms) or
  `anim-duration-normal` (120 ms).
- Scroll animation is forced to `Immediate` in Insert mode so the viewport
  snaps-to-cursor and typed text never drifts.

## Trail

- `cursor_trail` is a single `Vec<(usize, usize)>` on the controller (line, col).
- **Must be cleared on every document-switch path.** `reset_cursor_visual_state()`
  clears the trail and sets animation kinds to Immediate. It is called from:
  `previous_document`, `next_document`, `switch_to_document`, `close_document`,
  `new_file`, `open_path` (both branches), and `open_file_or_focus_existing`.
- **Must be filtered by current line length.** In `editor_lines()`, only emit a
  `trail_columns` entry when `t_col <= line.chars().count()`. Otherwise ghosts
  render beyond the line text after an edit shortens the line.
- Ghosts are static at `with_alpha(0.15)`; do not attach `animate opacity` to
  them unless a property actually drives the opacity value (a dead animate block
  is misleading and was removed).

## Gotchas

- Removing serde fields from `AppConfig` is safe because the struct does not use
  `deny_unknown_fields`; old config files with the removed keys still load (the
  extra keys are silently ignored by serde by default for structs).
- When adding a new `in property` to the Slint `AppWindow`, also add it to the
  `SettingsData` struct if it is a settings toggle, and wire it in both
  `apply_snapshot` (for settings data) and `apply_editor_snapshot` (for
  editor-facing properties).
- The blink timer must be kept alive for the duration of `window.run()`; bind it
  to a `_blink_timer` variable (like the IPC timer) so it is not dropped early.
- `slint::Timer::start` with `TimerMode::Repeated` fires on the UI thread; keep
  the closure cheap (read a config bool, compute elapsed, set one property).
