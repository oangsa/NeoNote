# Skill: Cursor Animation (blink + glide + smear)

Captures the rules and gotchas for the three cursor visual effects in NeoNote:
blinking, glide, and smear. All are driven from Rust-owned editor state and
rendered by Slint.

## Architecture

- **Floating overlay** (`ui/app-window.slint`): one `Rectangle` positioned from
  `cursor-prefix-measure.preferred-width` and `cursor-line * editor-line-height`.
  It carries `animate x`, `animate y`, and `animate opacity`.
- **Rust controller** (`src/app.rs`): owns `cursor_animation_kind` and
  `scroll_animation_kind`. The snapshot exposes these plus `cursor_insert_mode`
  and duration values to Slint.
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
- Vertical cursor movement is forced to `Immediate`; Slint must not animate
  cursor `y`. Animating the block cursor vertically can look like a persistent
  tinted column in front of each traversed line.
- **Insert mode is NOT forced to Immediate.** Insert uses the same diff logic as
  Normal mode, but the Slint duration expression selects `anim-duration-insert`
  (50 ms) when `cursor-insert-mode == true`, which is snappy enough that fast
  typing does not feel laggy. Normal mode uses `anim-duration-short` (80 ms) or
  `anim-duration-normal` (120 ms).
- Scroll animation is forced to `Immediate` in Insert mode so the viewport
  snaps-to-cursor and typed text never drifts.

## Cursor Geometry

- Keep cursor placement geometry centralized in `ui/app-window.slint` root
  metrics. Text rows, selection rectangles, the cursor rectangle, pointer
  coordinate conversion, and smear should all consume the same gutter, content
  padding, line height, measured prefix width, measured cell width, target x/y,
  cursor width, and cursor height properties.
- Guard empty cursor prefixes explicitly with `0px`; hidden empty `Text`
  measurement can otherwise report a width that nudges the first-column cursor.
- Do not apply a hard-coded cursor x offset unless a measured reproduction proves
  Slint text metrics need it. If an offset is ever required, apply it through the
  shared target/width metrics so every cursor path moves together.
- Avoid drawing a separate single-character inverted cursor glyph over the row
  text. It creates a second text layout path and can look like a misaligned dark
  overlay. Prefer a translucent block cursor over the authoritative row text.

## Smear (1.1.0 fallback-disabled)

- Cursor smear produced an unwanted grey vertical column in normal mode during
  same-column vertical movements, so the 1.1.0 fallback rule is active: keep the
  compatibility fields, but do not render or expose the effect.
- `AppConfig::normalize_disabled_features()` clamps `enable_cursor_smear` to
  `false`, including old config files that saved it as `true`.
- `AppSnapshot` and `SettingsSnapshot` report `enable_cursor_smear == false` and
  `smear_eligible == false` even if a test or old code forces the config field
  true.
- `ui/app-window.slint` keeps the generated properties for compatibility, but
  the smear overlay is never visible and the Settings panel does not expose a
  Cursor Smear toggle.

## Visual Selection

- The selection rectangle uses `border-radius: 4px` for rounded corners.
- On empty lines within a visual selection, `is-line-selected` is true but
  `selected-text` is empty. The Slint rendering falls back to
  `char-width-measure.preferred-width` (one cell) so the selection is visible
  even on empty lines.
- `char-width-measure` is an invisible "X" Text used only for cell-width
  calibration for the empty-line selection fallback.

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
