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
- Keep editor font metrics centralized in Slint properties. Render each line as an independent fixed-height row. For non-cursor rows, a single text item is fine; for the cursor row, render prefix, cursor cell, and suffix as row-local segments with the cursor character drawn inside the block. Size the normal-mode block from the cursor cell text's own `preferred-width`; do not force a guessed monospace width or overlay a block on a separately rendered full line.
- During editor key handling, update only editor-facing properties; avoid reapplying static theme brushes on every keystroke.
- Multi-document support belongs in the app controller. Each open tab/document owns its own `NoteDocument`; switching documents should only change the active index and refresh the Slint snapshot.
