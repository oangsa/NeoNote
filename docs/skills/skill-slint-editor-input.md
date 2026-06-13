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

- Keep `document-text` as the raw buffer content. Do not inject a cursor marker into a cloned copy of the text for every key press.
- Expose cursor row/column and mode as separate Slint properties, then draw a cursor rectangle over the editor surface.
- Use a block rectangle for Normal mode and a thin bar for Insert mode.
- Keep editor font metrics centralized in Slint properties. Render the full document text, including the active line, through one multiline `Text` element so every row uses the same text origin and shaping path. In Slint 1.16, `Text` has no `line-height` property; derive the cursor row pitch from hidden single-line and two-line text probes using the same font. Draw the cursor as a separate overlay positioned from a Slint-measured monospace cell width; do not rebuild the active line from prefix/cell/suffix `Text` segments because that can drift at column zero.
- During editor key handling, update only editor-facing properties; avoid reapplying static theme brushes on every keystroke.
- Multi-document support belongs in the app controller. Each open tab/document owns its own `NoteDocument`; switching documents should only change the active index and refresh the Slint snapshot.
