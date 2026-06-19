# skill-slint-editor-input

NeoNote's Slint editor surface should capture key events with a `FocusScope`, not a Slint text input widget.

Pattern validated in this repo:

- Declare a Slint callback such as `editor-key(string)`.
- Put a `FocusScope` around the rendered editor surface.
- In `key-pressed(event)`, map special keys with Slint `Key` constants before calling Rust:
  - `Key.Escape` -> `"escape"`
  - `Key.Return` -> `"return"`
  - `Key.Backspace` -> `"backspace"`
  - `Key.Delete` -> `"delete"`
  - arrow keys -> Vim-style direction intents at the Rust boundary
- Return `accept` after forwarding the key so widgets do not consume editor input inconsistently.
- Add a `TouchArea` that calls `focus()` on the editor `FocusScope`, and refocus the editor from Rust after handling editor input.

Rust remains the editor authority:

- Normal mode keys route to `NoteDocument::handle_normal_input`.
- Insert mode printable text routes to controlled buffer mutation APIs.
- File actions stay at the app-controller layer.
- Render the caret from Rust-owned cursor state. Normal mode clamps the marker onto a character; insert mode may render after the final character on a line.

Performance pattern:

- Expose the editor as a Slint model with one row per buffer line. Each row should include line number, text, cursor-line flag, cursor column, and cursor style. This avoids trying to align a custom cursor overlay to Slint's internal multiline text layout.
- Do not inject a cursor marker into a cloned copy of the text for every key press.
- Use a block rectangle for Normal mode and a thin bar for Insert mode.
- Keep editor font metrics centralized in Slint properties. Render every row's visible text through the same full-line `Text` item, regardless of cursor focus.
- Draw the cursor as a background rectangle behind the full-line text. Position it from shared editor metrics (`editor-origin-x`, `gutter-width`, `content-padding-x`, `char-width-px`) so the cursor and any cursor effects share the same target.
- When `cursor-prefix` is empty, position the cursor at the editor text origin instead of using `prefix-measure.preferred-width`; Slint may report a non-zero preferred width for an empty/hidden text item depending on layout context.
- Do not rely on a small negative `cursor-x-adjust` to compensate for glyph side-bearing; keep all editor geometry derived from explicit metrics.
- Render visual-mode selections from numeric column spans (`selection-start-column`, `selection-end-column`, `selection-render-end-column`) rather than measured selected-text widths. This keeps multi-line selections aligned and lets empty interior lines extend to a shared bridge right edge.
- Do not reconstruct the focused line as prefix/cell/suffix visible text segments. That creates a second text layout path and can drift from non-focused row placement.
- During editor key handling, update only editor-facing properties; avoid reapplying static theme brushes on every keystroke.
- Multi-document support belongs in the app controller. Each open tab/document owns its own `NoteDocument`; switching documents should only change the active index and refresh the Slint snapshot.
