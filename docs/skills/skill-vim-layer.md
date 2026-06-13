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
- Future operators, visual mode, undo, registers, and repeat should all reuse the same motion and text-buffer APIs.
- Pending normal-mode operators should retain their operator count until a motion or text object arrives. Combine operator and motion counts, then mutate a normalized flat character range.
- Keep text object behavior separate from motion behavior: `iw` stays inside word bounds, while `aw` may absorb adjacent horizontal whitespace.
- Yank operations update both the unnamed register and yank register `0`; delete/change update the unnamed register without replacing the yank register.
- `x` is a delete command and should update the unnamed register with the deleted character span without replacing yank register `0`.
- Operator `gg` motions should resolve to a linewise range. Counts typed between the operator and `gg`, such as `y2gg`, select the destination line.
- Plain `y`, `p`, and `P` use NeoNote's internal unnamed register. When the user enables `sync_clipboard`, the app controller may import/export that unnamed register to the system clipboard around those commands. Keep the Vim layer platform-free; explicit clipboard register support such as `"+y` can be added later.
