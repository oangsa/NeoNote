# Vim Complete Reference — Normal, Visual & Insert Modes

> **How to read this guide:**
> - `[count]` means an optional number prefix (e.g. `3w`, `5j`)
> - `{motion}` means any motion command (e.g. `w`, `$`, `gg`)
> - `{char}` means any single character
> - Motions marked **exclusive** don't include the character under the cursor; **inclusive** do
> - **Normal mode** is the default. Press `Esc` from any mode to return to it.

---

## Table of Contents
1. [Mode Overview](#mode-overview)
2. [Normal Mode — Navigation](#normal-mode--navigation)
3. [Normal Mode — Operators & Text Objects](#normal-mode--operators--text-objects)
4. [Normal Mode — Editing Commands](#normal-mode--editing-commands)
5. [Normal Mode — Registers, Marks & Macros](#normal-mode--registers-marks--macros)
6. [Normal Mode — Search & Replace](#normal-mode--search--replace)
7. [Normal Mode — Folds & Diff](#normal-mode--folds--diff)
8. [Normal Mode — Windows, Tabs & Buffers](#normal-mode--windows-tabs--buffers)
9. [Insert Mode — Entering & Exiting](#insert-mode--entering--exiting)
10. [Insert Mode — Editing While Inserting](#insert-mode--editing-while-inserting)
11. [Insert Mode — Autocomplete (Ctrl+X sub-mode)](#insert-mode--autocomplete-ctrlx-sub-mode)
12. [Visual Mode — Entering & Behavior](#visual-mode--entering--behavior)
13. [Visual Mode — Motions (how they extend selection)](#visual-mode--motions-how-they-extend-selection)
14. [Visual Mode — Operators & Commands](#visual-mode--operators--commands)
15. [Visual Block Mode — Special Behaviors](#visual-block-mode--special-behaviors)
16. [Operator + Motion Combinations Cheatsheet](#operator--motion-combinations-cheatsheet)
17. [Exiting Vim](#exiting-vim)

---

## Mode Overview

```
                    ┌─────────────────────────────────┐
                    │           NORMAL MODE            │
                    │    (default — navigation,        │
                    │     operators, commands)         │
                    └──────────┬──────────┬───────────┘
                               │          │
          i / I / a / A /      │          │  v / V / Ctrl+v
          o / O / c / s /      │          │
          R                    │          │
                               ▼          ▼
              ┌─────────────┐       ┌──────────────┐
              │ INSERT MODE │       │ VISUAL MODE  │
              │  (type text)│       │ (select text)│
              └──────┬──────┘       └──────┬───────┘
                     │                     │
              Esc / Ctrl+c          Esc / Ctrl+c / operator
                     │                     │
                     └──────────┬──────────┘
                                ▼
                          NORMAL MODE
```

| Mode | Indicator in statusline | Purpose |
|---|---|---|
| Normal | *(none)* | Navigate, operate, issue commands |
| Insert | `-- INSERT --` | Type and edit text |
| Replace | `-- REPLACE --` | Overwrite characters |
| Visual (char) | `-- VISUAL --` | Select characters |
| Visual (line) | `-- VISUAL LINE --` | Select whole lines |
| Visual (block) | `-- VISUAL BLOCK --` | Select rectangular column |
| Insert-Normal | `-- (insert) --` | One normal command from insert mode |

---

## Normal Mode — Navigation

> Prefix any motion with `[count]` to repeat it (e.g. `5j` = move down 5 lines).

### Basic Cursor Movement

| Command | Behavior |
|---|---|
| `h` | Move left one character |
| `l` | Move right one character |
| `j` | Move down one real line |
| `k` | Move up one real line |
| `gj` | Move down one *display* line (wraps across soft-wrapped lines) |
| `gk` | Move up one *display* line |

### Screen Position Jumps

| Command | Behavior |
|---|---|
| `H` | Jump to top of screen (High) |
| `M` | Jump to middle of screen (Middle) |
| `L` | Jump to bottom of screen (Low) |

### Word Motions

| Command | Scope | Behavior |
|---|---|---|
| `w` | word | Jump to **start** of next word (exclusive) |
| `W` | WORD | Jump to **start** of next WORD, including punctuation (exclusive) |
| `e` | word | Jump to **end** of current/next word (inclusive) |
| `E` | WORD | Jump to **end** of current/next WORD (inclusive) |
| `b` | word | Jump **back** to start of current/previous word (exclusive) |
| `B` | WORD | Jump **back** to start of current/previous WORD (exclusive) |
| `ge` | word | Jump **back** to **end** of previous word (inclusive) |
| `gE` | WORD | Jump **back** to **end** of previous WORD (inclusive) |

> **word vs WORD:** A `word` is letters/digits/underscores. A `WORD` is any non-blank sequence — it never breaks on punctuation.

### Line Motions

| Command | Behavior |
|---|---|
| `0` | Jump to column 0 (absolute start of line) — exclusive |
| `^` | Jump to first non-blank character of line — exclusive |
| `$` | Jump to end of line (last character) — inclusive |
| `g_` | Jump to last non-blank character of line |
| `[count]|` | Jump to column `[count]` |

### Find Character on Current Line

| Command | Behavior |
|---|---|
| `f{char}` | Jump **forward** to next `{char}` on line (inclusive) |
| `F{char}` | Jump **backward** to previous `{char}` on line (inclusive) |
| `t{char}` | Jump **forward** to just **before** next `{char}` (exclusive) |
| `T{char}` | Jump **backward** to just **after** previous `{char}` (exclusive) |
| `;` | Repeat last `f/F/t/T` in same direction |
| `,` | Repeat last `f/F/t/T` in opposite direction |

### Document-Level Jumps

| Command | Behavior |
|---|---|
| `gg` | Go to first line of document |
| `G` | Go to last line of document |
| `[count]G` or `[count]gg` | Go to line `[count]` |
| `%` | Jump to matching bracket: `()`, `{}`, `[]` — use `:h matchpairs` for more |
| `}` | Jump forward to next blank line (paragraph/block end) |
| `{` | Jump backward to previous blank line (paragraph/block start) |
| `]]` | Jump to next `{` in first column (next function/section) |
| `[[` | Jump to previous `{` in first column |
| `][` | Jump to next `}` in first column |
| `[]` | Jump to previous `}` in first column |

### Scrolling (cursor stays unless noted)

| Command | Behavior |
|---|---|
| `Ctrl+e` | Scroll down one line (cursor stays) |
| `Ctrl+y` | Scroll up one line (cursor stays) |
| `Ctrl+d` | Scroll down half a page (cursor moves too) |
| `Ctrl+u` | Scroll up half a page (cursor moves too) |
| `Ctrl+f` | Scroll down full page (cursor to top of new view) |
| `Ctrl+b` | Scroll up full page (cursor to bottom of new view) |
| `zz` | Redraw with cursor line centered on screen |
| `zt` | Redraw with cursor line at top of screen |
| `zb` | Redraw with cursor line at bottom of screen |

### Jump List Navigation

| Command | Behavior |
|---|---|
| `Ctrl+o` | Go to older (previous) position in jump list |
| `Ctrl+i` | Go to newer (next) position in jump list |
| `:jumps` | Show the jump list |

### Change List Navigation

| Command | Behavior |
|---|---|
| `g;` | Go to older position in change list |
| `g,` | Go to newer position in change list |
| `:changes` | Show the change list |

### Declaration Jumps

| Command | Behavior |
|---|---|
| `gd` | Go to local declaration of word under cursor |
| `gD` | Go to global declaration of word under cursor |
| `Ctrl+]` | Jump to tag under cursor (requires tags file) |

---

## Normal Mode — Operators & Text Objects

Operators follow the grammar: `[count] operator [count] {motion or text-object}`

When an operator is **doubled** (e.g. `dd`, `yy`, `cc`), it operates on the **current line**.

### Operators

| Operator | Action |
|---|---|
| `d` | **Delete** (cut) text into unnamed register |
| `c` | **Change** (delete then enter Insert mode) |
| `y` | **Yank** (copy) text into unnamed register |
| `>` | **Indent** right one shiftwidth |
| `<` | **Indent** left one shiftwidth |
| `=` | **Auto-indent** (format) |
| `~` | **Toggle case** of each character |
| `g~` | Toggle case up to motion |
| `gu` | Convert to **lowercase** up to motion |
| `gU` | Convert to **UPPERCASE** up to motion |
| `!` | Filter through external command |
| `gq` | Format text (reflow to `textwidth`) |
| `gw` | Format text without moving cursor |

> When counts are combined, they multiply: `3d2w` deletes 6 words.

### Text Objects (used after operators or in Visual mode)

Text objects always select a **region**, regardless of cursor position within it.
`i` = **inner** (contents only), `a` = **around** (includes surrounding delimiters/whitespace).

| Text Object | Selects |
|---|---|
| `iw` / `aw` | Inner word / Around word (includes trailing space) |
| `iW` / `aW` | Inner WORD / Around WORD |
| `is` / `as` | Inner sentence / Around sentence |
| `ip` / `ap` | Inner paragraph / Around paragraph |
| `i(` / `a(` or `ib` / `ab` | Inner/around `()` block |
| `i{` / `a{` or `iB` / `aB` | Inner/around `{}` block |
| `i[` / `a[` | Inner/around `[]` block |
| `i<` / `a<` | Inner/around `<>` block |
| `it` / `at` | Inner/around XML/HTML `<tag>` block |
| `i"` / `a"` | Inner/around double-quoted string |
| `i'` / `a'` | Inner/around single-quoted string |
| `` i` `` / `` a` `` | Inner/around backtick-quoted string |

**Examples:**
```
diw   → delete word under cursor
ci"   → change everything inside double quotes
yap   → yank the entire paragraph (including trailing blank line)
>iB   → indent the inner {} block
=ip   → auto-indent the paragraph
```

---

## Normal Mode — Editing Commands

### Single-Key Shortcuts (no motion needed)

| Command | Behavior |
|---|---|
| `x` | Delete (cut) character under cursor |
| `X` | Delete character before cursor |
| `s` | Delete character and enter Insert mode (shorthand for `cl`) |
| `S` | Delete entire line and enter Insert mode (shorthand for `cc`) |
| `r{char}` | Replace single character under cursor with `{char}` (stays in Normal) |
| `R` | Enter Replace mode — overwrites characters until `Esc` |
| `~` | Toggle case of character under cursor, move right |
| `.` | Repeat last change (includes count if used) |
| `u` | Undo last change |
| `U` | Undo all changes to the current line |
| `Ctrl+r` | Redo (reverse of undo) |

### Join Lines

| Command | Behavior |
|---|---|
| `J` | Join line below to current line with a space |
| `gJ` | Join line below to current line without space |
| `[count]J` | Join next `[count]` lines |

### Paste

| Command | Behavior |
|---|---|
| `p` | Paste after cursor (or below current line if linewise) |
| `P` | Paste before cursor (or above current line if linewise) |
| `gp` | Paste after cursor; leave cursor after pasted text |
| `gP` | Paste before cursor; leave cursor after pasted text |
| `]p` | Paste and adjust indent to match current line |
| `[p` | Paste before and adjust indent to match current line |

### Indenting

| Command | Behavior |
|---|---|
| `>>` | Indent current line one shiftwidth to the right |
| `<<` | De-indent current line one shiftwidth to the left |
| `[count]>>` | Indent `[count]` lines |
| `>%` | Indent block from cursor to matching bracket |
| `>ib` | Indent inner `()` block |
| `>at` | Indent `<tag>` block |
| `=%` | Auto-indent block to matching bracket |
| `=iB` | Auto-indent inner `{}` block |
| `gg=G` | Auto-indent entire file |

---

## Normal Mode — Registers, Marks & Macros

### Registers

| Command | Behavior |
|---|---|
| `"x{op}` | Use register `x` for next delete/yank/paste. e.g. `"ayiw` yanks word into register `a` |
| `"xp` | Paste from register `x` |
| `"+y` | Yank into system clipboard |
| `"+p` | Paste from system clipboard |
| `"*y` | Yank into primary selection (X11) |
| `:reg` | Show all register contents |

**Special registers:**

| Register | Contents |
|---|---|
| `"` | Unnamed — last yank or delete |
| `0` | Last yank (not affected by deletes) |
| `1`–`9` | Delete history (most recent = `1`) |
| `+` | System clipboard (X11 clipboard) |
| `*` | Primary selection (X11) |
| `%` | Current filename |
| `#` | Alternate filename |
| `/` | Last search pattern |
| `:` | Last command-line command |
| `.` | Last inserted text |
| `-` | Last small delete (less than a line) |
| `=` | Expression register (evaluate on paste) |
| `_` | Black hole register (discard) |

### Marks

| Command | Behavior |
|---|---|
| `m{a-z}` | Set mark (lowercase = file-local) |
| `m{A-Z}` | Set global mark (works across files) |
| `` `{mark} `` | Jump to exact position (line + column) of mark |
| `'{mark}` | Jump to first non-blank character of mark's line |
| `` `0 `` | Position where Vim was last exited |
| `` `" `` | Position when last editing this file |
| `` `. `` | Position of last change in this file |
| ` `` ` | Position before last jump |
| `` y`a `` | Yank from cursor to mark `a` |
| `:marks` | List all marks |

### Macros

| Command | Behavior |
|---|---|
| `q{a-z}` | Start recording macro into register `{a-z}` |
| `q` | Stop recording |
| `@{a-z}` | Play back macro from register `{a-z}` |
| `@@` | Repeat last played macro |
| `[count]@{a-z}` | Play macro `[count]` times |

> **Tip:** Macros record every keystroke including mode switches. End macros with a motion that positions for the next repetition.

---

## Normal Mode — Search & Replace

| Command | Behavior |
|---|---|
| `/{pattern}` | Search forward for `{pattern}` |
| `?{pattern}` | Search backward for `{pattern}` |
| `n` | Repeat search in same direction |
| `N` | Repeat search in opposite direction |
| `*` | Search forward for exact word under cursor |
| `#` | Search backward for exact word under cursor |
| `g*` | Search forward for word under cursor (partial match OK) |
| `g#` | Search backward for word under cursor (partial match OK) |
| `gn` | Search last pattern forward and visually select the match |
| `gN` | Search last pattern backward and visually select the match |
| `:noh` | Remove search highlighting |
| `\vpattern` | "Very magic" — all non-alphanumeric chars are regex special |

### Substitution

| Command | Behavior |
|---|---|
| `:%s/old/new/g` | Replace all occurrences in file |
| `:%s/old/new/gc` | Replace all with confirmation (`y/n/a/q/l`) |
| `:s/old/new/g` | Replace all on current line only |
| `:'<,'>s/old/new/g` | Replace within visual selection (auto-filled after `v…:`) |
| `:%s/old/new/gi` | Case-insensitive replace |
| `:%s/\<word\>/new/g` | Replace exact whole word only |

---

## Normal Mode — Folds & Diff

| Command | Behavior |
|---|---|
| `zf{motion}` | Manually create fold (e.g. `zf5j` folds 5 lines down) |
| `zd` | Delete fold under cursor |
| `za` | Toggle fold open/closed |
| `zo` | Open fold under cursor |
| `zc` | Close fold under cursor |
| `zO` | Open all folds under cursor recursively |
| `zC` | Close all folds under cursor recursively |
| `zr` | Reduce fold level — open all folds one level |
| `zm` | More folds — close all folds one level |
| `zR` | Open all folds in file |
| `zM` | Close all folds in file |
| `zi` | Toggle `foldenable` (all folds on/off) |
| `]c` | Jump to start of next diff change |
| `[c` | Jump to start of previous diff change |
| `do` / `:diffget` | Obtain diff (pull change from other buffer) |
| `dp` / `:diffput` | Put diff (push change to other buffer) |
| `:diffthis` | Make this window part of diff |
| `:diffupdate` | Refresh diff highlighting |
| `:diffoff` | Turn off diff for current window |

---

## Normal Mode — Windows, Tabs & Buffers

### Windows

| Command | Behavior |
|---|---|
| `Ctrl+ws` or `:split` | Split window horizontally |
| `Ctrl+wv` or `:vsplit` | Split window vertically |
| `Ctrl+ww` | Cycle to next window |
| `Ctrl+wh/j/k/l` | Move focus left/down/up/right |
| `Ctrl+wH/J/K/L` | Move window to far left/bottom/top/right (full height/width) |
| `Ctrl+wx` | Exchange window with next |
| `Ctrl+wq` | Close window |
| `Ctrl+w=` | Equalize all window sizes |
| `Ctrl+w+` / `Ctrl+w-` | Increase / decrease window height |
| `Ctrl+w>` / `Ctrl+w<` | Increase / decrease window width |
| `Ctrl+wT` | Move current window to its own tab |

### Tabs

| Command | Behavior |
|---|---|
| `:tabnew [file]` | Open new tab (optionally with file) |
| `gt` / `:tabnext` | Go to next tab |
| `gT` / `:tabprev` | Go to previous tab |
| `[count]gt` | Go to tab number `[count]` |
| `:tabmove [n]` | Move current tab to position `n` (0-indexed) |
| `:tabclose` | Close current tab |
| `:tabonly` | Close all tabs except current |
| `:tabdo {cmd}` | Run command on all tabs |

### Buffers

| Command | Behavior |
|---|---|
| `:e file` | Edit file in current window (new buffer) |
| `:bn` / `:bp` | Next / previous buffer |
| `:bd` | Delete (close) buffer |
| `:b#` | Switch to buffer by number |
| `:b name` | Switch to buffer by filename |
| `:ls` / `:buffers` | List all buffers |
| `:sp file` | Open file in horizontal split |
| `:vsp file` | Open file in vertical split |
| `:vert ball` | Open all buffers as vertical splits |
| `:tab ball` | Open all buffers as tabs |

---

## Insert Mode — Entering & Exiting

### From Normal Mode → Insert Mode

| Command | Where insertion begins |
|---|---|
| `i` | Before cursor |
| `I` | At first non-blank character of line |
| `a` | After cursor |
| `A` | At end of line |
| `o` | New line below current line |
| `O` | New line above current line |
| `ea` | After the end of the current word |
| `gi` | At the last place you were in Insert mode |
| `s` | Delete character under cursor, then insert |
| `S` or `cc` | Delete entire line, then insert |
| `C` or `c$` | Delete from cursor to end of line, then insert |
| `ciw` | Delete inner word, then insert |
| `ci"` | Delete inside quotes, then insert *(any text object works)* |
| `R` | Enter Replace mode (overwrites rather than inserts) |

> **Count prefix:** Entering Insert mode with a count (e.g. `3i`) repeats everything you type until `Esc` — three times. Works with `i`, `I`, `a`, `A`, `o`, `O`.

### From Insert Mode → Normal Mode

| Command | Behavior |
|---|---|
| `Esc` | Exit Insert mode → Normal mode |
| `Ctrl+c` | Exit Insert mode → Normal mode (skips abbreviation expansion) |
| `Ctrl+[` | Same as `Esc` |

---

## Insert Mode — Editing While Inserting

> These commands work **without leaving Insert mode**.

### Deletion

| Command | Behavior |
|---|---|
| `Ctrl+h` | Delete character before cursor (same as `Backspace`) |
| `Ctrl+w` | Delete word before cursor |
| `Ctrl+u` | Delete all characters before cursor on current line |

> ⚠️ `Ctrl+u` cannot be undone in one step. See `:help i_CTRL-U`.

### Navigation (without leaving Insert)

| Command | Behavior |
|---|---|
| Arrow keys | Move cursor in any direction |
| `Ctrl+o {cmd}` | Execute **one** Normal mode command then return to Insert mode. E.g. `Ctrl+o zz` centers screen, `Ctrl+o d$` deletes to end of line |

### Paste from Register

| Command | Behavior |
|---|---|
| `Ctrl+r {reg}` | Insert contents of register `{reg}` at cursor (e.g. `Ctrl+r a`, `Ctrl+r "`, `Ctrl+r +`) |
| `Ctrl+r Ctrl+r {reg}` | Insert literally (no auto-indent adjustment) |
| `Ctrl+r =` | Open expression prompt — type an expression, result is inserted (e.g. `Ctrl+r = 2+2` inserts `4`) |

**Handy register shortcuts in Insert:**

| Command | Inserts |
|---|---|
| `Ctrl+r "` | Last yanked/deleted text |
| `Ctrl+r 0` | Last yanked text |
| `Ctrl+r +` | System clipboard |
| `Ctrl+r %` | Current filename |
| `Ctrl+r /` | Last search term |
| `Ctrl+r :` | Last command-line command |

### Indentation

| Command | Behavior |
|---|---|
| `Ctrl+t` | Indent current line one shiftwidth (insert a tab equivalent) |
| `Ctrl+d` | De-indent current line one shiftwidth |

### Scrolling (Insert mode)

| Command | Behavior |
|---|---|
| `Ctrl+x Ctrl+e` | Scroll screen down one line (cursor stays) |
| `Ctrl+x Ctrl+y` | Scroll screen up one line (cursor stays) |

### Digraphs & Special Characters

| Command | Behavior |
|---|---|
| `Ctrl+k {c1}{c2}` | Insert digraph (e.g. `Ctrl+k a:` → `ä`). See `:digraphs` |
| `Ctrl+v {code}` | Insert character by decimal code |
| `Ctrl+v x{hex}` | Insert character by hex code |
| `Ctrl+v u{hex}` | Insert Unicode character |

---

## Insert Mode — Autocomplete (Ctrl+X sub-mode)

Press `Ctrl+x` in Insert mode to enter autocomplete sub-mode, then follow with:

| Command | Completes from |
|---|---|
| `Ctrl+x Ctrl+n` | Keywords in current file (forward) |
| `Ctrl+x Ctrl+p` | Keywords in current file (backward) |
| `Ctrl+x Ctrl+l` | Whole lines in current file |
| `Ctrl+x Ctrl+f` | Filenames in current directory |
| `Ctrl+x Ctrl+]` | Tags (requires tags file) |
| `Ctrl+x Ctrl+i` | Keywords from included files |
| `Ctrl+x Ctrl+d` | Defined names / macros |
| `Ctrl+x Ctrl+k` | Dictionary words (requires `dictionary` setting) |
| `Ctrl+x Ctrl+t` | Thesaurus words (requires `thesaurus` setting) |
| `Ctrl+x Ctrl+o` | Omni-completion (language-aware, e.g. CSS, HTML, Python) |

**While a completion popup is open:**

| Key | Behavior |
|---|---|
| `Ctrl+n` | Select next match |
| `Ctrl+p` | Select previous match |
| `Enter` or `Ctrl+y` | Accept selected match |
| `Ctrl+e` | Cancel and restore original text |
| `Ctrl+l` | Add one character from current match |

---

## Visual Mode — Entering & Behavior

### Entering Visual Mode

| Command | Type | Status line |
|---|---|---|
| `v` | Character-wise | `-- VISUAL --` |
| `V` | Line-wise | `-- VISUAL LINE --` |
| `Ctrl+v` | Block-wise | `-- VISUAL BLOCK --` |
| `gv` | Re-enter with previous selection | *(whichever was last)* |
| `o` | While in visual: swap cursor to **other end** of selection | — |
| `O` | While in visual block: swap to **other corner** of block | — |

### How Selection Works

- Selection always spans from an **anchor** point to the **cursor**.
- The cursor is always at **one end** of the selection.
- **Pressing `o`** moves the cursor to the opposite end — useful when you need to extend or shrink from the other side.
- **Pressing `O`** in block mode moves to the diagonally opposite corner.
- Motions **expand** the selection when moving away from the anchor, and **contract** it when moving toward or past the anchor.
- The selection is always at least one character (in character-wise mode) or one line (in line-wise mode).

### Switching Between Visual Sub-modes

| Key (while in visual) | Result |
|---|---|
| `v` | Switch to character-wise (or exit if already character-wise) |
| `V` | Switch to line-wise (or exit if already line-wise) |
| `Ctrl+v` | Switch to block-wise (or exit if already block-wise) |

---

## Visual Mode — Motions (how they extend selection)

**All Normal mode motions work in Visual mode** — the difference is they *extend the selection* instead of just moving the cursor.

### Character Motions

| Motion | What gets selected |
|---|---|
| `h` / `l` | Shrink / extend selection by one character |
| `w` | Extend to start of next word |
| `W` | Extend to start of next WORD |
| `e` | Extend to end of current/next word |
| `E` | Extend to end of current/next WORD |
| `b` | Contract back to start of previous word |
| `B` | Contract back to start of previous WORD |
| `ge` | Contract back to end of previous word |
| `0` | Extend/contract to start of line |
| `^` | Extend/contract to first non-blank of line |
| `$` | Extend to end of line |
| `f{char}` | Extend forward to `{char}` on line (inclusive) |
| `t{char}` | Extend forward to just before `{char}` |
| `F{char}` | Contract backward to `{char}` |
| `;` / `,` | Repeat last `f/t/F/T` |

### Line & Document Motions

| Motion | What gets selected |
|---|---|
| `j` / `k` | Extend/contract selection down/up one line |
| `gg` | Extend selection to first line |
| `G` | Extend selection to last line |
| `[count]G` | Extend selection to line `[count]` |
| `H` / `M` / `L` | Extend selection to top/middle/bottom of screen |
| `%` | Extend to matching bracket |
| `}` | Extend to next paragraph end |
| `{` | Extend to previous paragraph start |

### Text Objects in Visual Mode

In Visual mode, text objects **redefine the entire selection** to the text object:

| Motion | New selection |
|---|---|
| `iw` / `aw` | Inner / around word |
| `ip` / `ap` | Inner / around paragraph |
| `i(` / `a(` | Inner / around `()` |
| `i{` / `a{` | Inner / around `{}` |
| `it` / `at` | Inner / around tag |
| `i"` / `a"` | Inner / around double quotes |

**Example workflow:**
```
v           → enter character-wise visual mode
e           → extend to end of word
}           → extend to next paragraph end
d           → delete the entire selected region
```

---

## Visual Mode — Operators & Commands

Once text is selected, these commands act on the selection:

### Delete / Change / Yank

| Command | Behavior |
|---|---|
| `d` or `x` | Delete (cut) selection into unnamed register |
| `D` | Delete to end of line (each selected line) |
| `c` | Delete selection and enter Insert mode at start |
| `s` | Same as `c` in character/line visual |
| `y` | Yank (copy) selection |
| `Y` | Yank entire lines (linewise) even in character visual |
| `p` / `P` | Paste over selection (replaces it) |

### Formatting & Case

| Command | Behavior |
|---|---|
| `~` | Toggle case of each character in selection |
| `u` | Convert selection to lowercase |
| `U` | Convert selection to UPPERCASE |
| `>` | Indent selection one shiftwidth right |
| `<` | De-indent selection one shiftwidth left |
| `=` | Auto-indent selected lines |
| `gq` | Reflow/wrap selected text to `textwidth` |
| `gw` | Reflow without moving cursor |
| `J` | Join all selected lines with a space |
| `gJ` | Join all selected lines without space |

### Search & Substitute on Selection

| Command | Behavior |
|---|---|
| `/pattern` | Search within file (selection not limited, but starts from cursor) |
| `:'<,'>s/old/new/g` | Substitute within visual selection (Vim fills `'<,'>` automatically when you press `:`) |
| `:'<,'>!cmd` | Filter selection through external command `cmd` |

### Other Visual Commands

| Command | Behavior |
|---|---|
| `!{cmd}` | Filter selection through shell command (replaces selection with output) |
| `r{char}` | Replace every character in selection with `{char}` |
| `:` | Enter command-line mode with `'<,'>` range pre-filled |
| `Ctrl+a` | Increment all numbers in selection |
| `Ctrl+x` | Decrement all numbers in selection |
| `gv` | Reselect the previous visual selection |

---

## Visual Block Mode — Special Behaviors

Visual block mode (`Ctrl+v`) selects a **rectangular region**. Its commands have unique behaviors:

### Entering and Navigating

| Command | Behavior |
|---|---|
| `Ctrl+v` | Enter visual block mode |
| `h` / `l` | Narrow / widen the block |
| `j` / `k` | Shorten / lengthen the block |
| `$` | Extend each row to end of its respective line |
| `0` | Bring left edge to column 0 |
| `O` | Toggle cursor to other corner of block |
| `o` | Toggle cursor to opposite end on same row |

### Editing Commands (apply to all lines in block)

| Command | Behavior |
|---|---|
| `d` or `x` | Delete the selected block |
| `D` | Delete from left edge of block to end of line (on each row) |
| `c` | Delete block and enter Insert on first line → press `Esc` to apply to all lines |
| `I{text}Esc` | **Insert** `{text}` before the block on all lines (effect appears after `Esc`) |
| `A{text}Esc` | **Append** `{text}` after the block on all lines (effect appears after `Esc`) |
| `r{char}` | Replace every character in block with `{char}` |
| `~` | Toggle case of every character in block |
| `u` / `U` | Lowercase / uppercase every character in block |
| `>` / `<` | Indent / de-indent from the block's left column |
| `y` | Yank block; pasting with `p` inserts as a block |
| `p` | Paste a previously yanked block |

> ⚠️ **Important:** With `I`, `A`, and `c` in block mode, text is typed on the first line only. The change **propagates to all selected lines only after you press `Esc`**.

### Practical Block Mode Examples

```
Ctrl+v 3j $    → select from cursor to end of next 3 lines
A;Esc          → append semicolon to the end of all 4 lines

Ctrl+v 5j      → select column across 6 lines
I// Esc        → prepend "// " (comment) to all 6 lines

Ctrl+v 2j 2l   → select a 3-row × 3-column block
r*             → fill the entire block with asterisks

Ctrl+v G $     → select from cursor to end of every remaining line
d              → delete the right portion of every line
```

---

## Operator + Motion Combinations Cheatsheet

These are the most useful combinations — all built from the grammar above.

### Delete (`d`)

| Command | What it deletes |
|---|---|
| `dd` | Current line |
| `d$` / `D` | From cursor to end of line |
| `d0` | From cursor to start of line |
| `dw` | From cursor to start of next word |
| `de` | From cursor to end of word |
| `diw` | Entire word (inner) |
| `daw` | Entire word including surrounding space |
| `di"` | Contents of double-quoted string |
| `da"` | Double-quoted string including quotes |
| `dip` | Inner paragraph |
| `dap` | Paragraph including surrounding blank lines |
| `diB` / `daB` | Inner / around `{}` block |
| `d}` | From cursor to end of paragraph |
| `dG` | From current line to end of file |
| `dgg` | From current line to start of file |
| `:3,5d` | Lines 3 through 5 |
| `:g/pattern/d` | All lines matching pattern |
| `:g!/pattern/d` | All lines NOT matching pattern |

### Change (`c`)

| Command | What it changes |
|---|---|
| `cc` / `S` | Entire current line |
| `cw` / `ce` | From cursor to end of word |
| `ciw` | Entire word |
| `ci"` / `ca"` | Inside / around double quotes |
| `ci(` / `ca(` | Inside / around parentheses |
| `cit` / `cat` | Inside / around XML tag |
| `ciB` / `caB` | Inside / around `{}` block |
| `c$` / `C` | From cursor to end of line |
| `c0` | From cursor to start of line |

### Yank (`y`)

| Command | What it copies |
|---|---|
| `yy` / `Y` | Current line (linewise) |
| `yw` | From cursor to start of next word |
| `yiw` | Inner word |
| `yaw` | Word and surrounding space |
| `yi"` | Inside double quotes |
| `yip` | Inner paragraph |
| `y$` | From cursor to end of line |
| `yG` | From current line to end of file |
| `` y`a `` | From cursor to mark `a` |

### Indent (`>` / `<`)

| Command | What it indents |
|---|---|
| `>>` / `<<` | Current line |
| `3>>` | Next 3 lines |
| `>iB` | Inner `{}` block |
| `>%` | To matching bracket |
| `>ap` | Entire paragraph |

### Format / Reflow (`gq`)

| Command | What it formats |
|---|---|
| `gqip` | Inner paragraph |
| `gqq` | Current line |
| `gq}` | To end of paragraph |

---

## Exiting Vim

| Command | Behavior |
|---|---|
| `:w` | Save file (write), stay open |
| `:w file` | Save as new filename |
| `:w !sudo tee %` | Save with sudo (when opened without permissions) |
| `:wq` or `:x` or `ZZ` | Save and quit |
| `:q` | Quit (fails if unsaved changes exist) |
| `:q!` or `ZQ` | Quit and discard all unsaved changes |
| `:wqa` | Save all buffers and quit |
| `:qa` | Quit all windows (fails if unsaved) |
| `:qa!` | Quit all windows, discard all unsaved changes |

---

## Quick Reference: Mode Transitions

```
Normal ──i/I/a/A/o/O/c/s/R──→ Insert ──Esc──→ Normal
Normal ──v──→ Visual(char) ──Esc or operator──→ Normal
Normal ──V──→ Visual(line) ──Esc or operator──→ Normal
Normal ──Ctrl+v──→ Visual(block) ──Esc or operator──→ Normal
Insert ──Ctrl+o──→ (one command) ──auto──→ Insert
Visual ──v/V/Ctrl+v──→ switch sub-mode
Visual ──:──→ fills '< '> range in command-line
```

---

*For the full official reference: `:help` in Vim, or visit [vimhelp.org](https://vimhelp.org)*
