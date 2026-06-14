# skill-vim-layer

NeoNote's Vim layer is pure editor logic between Slint input callbacks and the text buffer.

Keep this layer free of:

- rendering
- process spawning
- RPC calls
- direct UI drawing
- direct filesystem writes

Core rules:

- Normal mode is the default mode for a new/opened document.
- Counts are parsed before commands; leading `0` is a line-start motion when no count is pending.
- Motions must clamp at file and line boundaries instead of panicking.
- Insert mode mutates `NoteDocument` directly so Slint text input controls do not own an incompatible caret.
- Future visual mode and advanced commands should reuse the same motion, text object, register, undo, and text-buffer APIs.
- Pending normal-mode operators should retain their operator count until a motion or text object arrives. Combine operator and motion counts, then mutate a normalized flat character range.
- Keep text object behavior separate from motion behavior: `iw` stays inside word bounds, while `aw` may absorb adjacent horizontal whitespace.
- Yank operations update both the unnamed register and yank register `0`; delete/change update the unnamed register without replacing the yank register.
- `x` is a delete command and should update the unnamed register with the deleted character span without replacing yank register `0`.
- Operator `gg` motions should resolve to a linewise range. Counts typed between the operator and `gg`, such as `y2gg`, select the destination line.
- Linewise ranges (such as from `yy`, `dd`, `cc`, etc.) can have `start == end` when operating on the last empty line of a document. Bypassing the `start >= end` empty range check for linewise operations is required to allow those actions to succeed on empty lines.
- Linewise register updates (yank or delete) must append a trailing newline (`\n`) if the text does not already end with one, ensuring standard Vim register behavior (which always ends linewise values with newlines, even on the last line of a file).
- Register prefixes are stored as pending Vim state. Named registers `a-z`, explicit clipboard register `+`, black-hole register `_`, unnamed register `"`, and yank register `0` stay inside the pure Vim layer.
- Plain `y`, `p`, and `P` use NeoNote's internal unnamed register. When the user enables `sync_clipboard`, the app controller may import/export the system clipboard around paste/yank/delete/change commands. Keep the Vim layer platform-free; explicit `+` register state is mirrored by the app boundary through `set_clipboard_register`.
- Undo/redo snapshots are editor-state transactions recorded before buffer mutations. Restore clears pending operators/counts/register prefixes and clamps the cursor for the restored mode.
- Dot repeat should only remember repeatable buffer changes. Do not let `u`, `Ctrl+r`, or yank-only commands replace the last repeat target.
- Phase 6 text objects currently include words, quotes, brackets, paragraphs, and line objects. Paragraph objects are linewise and use blank lines as paragraph boundaries.
- Visual mode is represented in the Vim layer as explicit mode state plus a flattened anchor offset. Slint receives row-level selection flags from Rust; do not let Slint compute selection ranges.
- Search state stores the last pattern, direction, and flattened match ranges. `:noh` clears highlight ranges but keeps the pattern so `n`/`N` can still repeat the last search.
- Pending search and ex-command input are represented as pending Vim commands. Normal key normalization should pass `return` and `backspace` through so those pending commands can accept or edit user input.
- Key normalization belongs in `src/vim/key.rs`. Keep it pure: convert app/Slint key strings into normal-mode and insert-mode intents, and let the app controller perform platform/UI side effects.
