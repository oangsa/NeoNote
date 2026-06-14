# skill-vim-behavior-current

This skill documents the Vim behavior that is implemented right now in NeoNote's Rust code.

Source of truth:

- `src/notes.rs`
- `src/vim/key.rs`
- `src/app.rs`

This file is intentionally code-derived. It describes current behavior, current quirks, and still-missing gaps.

## Mode And State Model

`NoteDocument` owns the current editor state. Implemented modes:

- `Normal`
- `Insert`
- `Visual`
- `VisualLine`
- `VisualBlock`
- `Command`
- `Search(Forward)`
- `Search(Backward)`
- `Replace`

The document currently owns:

- text content and dirty/open/path state
- cursor line and column
- count parsing
- pending command/operator/find/goto state
- registers and register routing
- undo/redo stacks
- last repeatable change
- visual anchor and last visual selection
- search pattern, direction, match list, and highlight toggle
- named marks
- jump list
- changelist
- viewport state
- command/search histories
- document-local settings such as numbering, wrapping, tab widths, and search options

## Input Boundary

`src/vim/key.rs` currently normalizes:

Normal-side input:

- `escape`, `ctrl+[`
- arrow keys -> `h`, `j`, `k`, `l`
- `return`, `backspace`
- `delete` ignored

Insert/replace-side input:

- `escape`, `ctrl+[`
- `ctrl+c`
- `ctrl+r`
- `ctrl+w`
- `ctrl+u`
- `return`
- `backspace`
- `delete`
- printable text
- arrow keys ignored

`AppController` routes `Insert` and `Replace` through the insert-key path.

## Normal Mode

Implemented entry and mode-switch commands:

- `i`, `I`, `a`, `A`
- `o`, `O`
- `R`
- `v`, `V`
- `ctrl+v`
- `:`
- `/`
- `?`
- `ZZ`
- `ZQ`

Implemented motions:

- `h`, `j`, `k`, `l`
- `w`, `W`, `b`, `B`, `e`, `E`
- `ge`, `gE`
- `0`, `^`, `$`
- `|`
- `+`, `-`, `_`
- `gg`
- `G`
- `{count}G`
- `f{char}`, `F{char}`, `t{char}`, `T{char}`
- `;`, `,`
- `%`
- `{`, `}`
- `(`, `)`
- `H`, `M`, `L`

Implemented viewport commands:

- `ctrl+d`
- `ctrl+u`
- `ctrl+f`
- `ctrl+b`
- `ctrl+e`
- `ctrl+y`
- `zz`
- `zt`
- `zb`

Implemented editing commands:

- `x`
- `X`
- `r{char}`
- `s`
- `S`
- `D`
- `C`
- `Y`
- `p`, `P`
- `J`
- `gJ`
- `~`
- `u`
- `ctrl+r`
- `.`

Implemented `g`-prefixed behavior:

- `gg`
- `gv`
- `gi`
- `g;`
- `g,`
- `ge`
- `gE`
- `gJ`
- `gu{motion}`
- `gU{motion}`
- `g~{motion}`
- `gUU`
- `guu`
- `g~~`

Counts are parsed before normal commands. `0` is treated as a motion only when no count is in progress.

## Insert And Replace

Insert mode currently supports:

- direct printable text insertion
- `return`
- `backspace`
- `delete`
- `ctrl+w`
- `ctrl+u`
- `ctrl+r {register}`
- `escape`
- `ctrl+[`
- `ctrl+c`

Current insert details:

- `backspace` joins upward at column `0`
- `delete` joins downward at end of line
- `ctrl+w` deletes the previous word segment
- `ctrl+u` deletes to start of line or joins upward from column `0`
- `ctrl+r {register}` inserts unnamed, clipboard, numbered, named, or small-delete register content without leaving insert mode

Replace mode currently supports:

- normal-mode `R` entry
- overwrite-on-type semantics
- append past line end
- exit with `escape` or `ctrl+[`

Repeat capture:

- insert and replace sessions are stored as a net replay sequence
- `last_insert_pos` is tracked for `gi`
- insert-register paste participates in the same insert session

## Operators

Implemented operators:

- `d`
- `c`
- `y`
- `>`
- `<`
- `=`
- `gu`
- `gU`
- `g~`

Implemented doubled operators:

- `dd`
- `cc`
- `yy`
- `>>`
- `<<`
- `==`

Implemented operator motions:

- `h`, `j`, `k`, `l`
- `w`, `W`, `b`, `B`, `e`, `E`
- `0`, `^`, `$`
- `G`
- `;`, `,`
- `%`
- `{`, `}`
- `(`, `)`
- `H`, `M`, `L`
- `f`, `F`, `t`, `T`
- `gg`

## Text Objects

Implemented text objects:

- `iw`, `aw`
- `iW`, `aW`
- `i'`, `a'`
- `i"`, `a"`
- `i(`, `a(`
- `i)`, `a)`
- `i[`, `a[`
- `i]`, `a]`
- `i{`, `a{`
- `i}`, `a}`
- `ip`, `ap`
- `il`, `al`

Current semantics:

- small words use `is_alphanumeric() || '_'`
- big words use any non-whitespace, non-newline character
- delimited text objects search backward for open and forward for close
- paragraph objects are blank-line delimited
- line objects expand linewise

## Visual, Visual Line, And Visual Block

Implemented visual entry:

- `v`
- `V`
- `ctrl+v`

Implemented visual movement:

- `h`, `j`, `k`, `l`
- `w`, `W`, `b`, `B`, `e`, `E`
- `0`, `^`, `$`
- `|`
- `+`, `-`, `_`
- `G`
- counted forms of the supported visual motions
- `o`

Implemented visual operators:

- `d`
- `x`
- `c`
- `y`
- `>`
- `<`
- `=`
- `~`
- `u`
- `U`

Implemented visual command/search entry:

- `:`
- `/`
- `?`

Visual command-line entry:

- stores the last visual selection
- exits visual mode
- prefills `'<,'>`

Blockwise visual behavior:

- block yank stores per-row slices and marks the register as `blockwise`
- block delete removes the rectangular region
- block change deletes the region and enters insert with deferred block replay
- block `I` and `A` defer multi-line insert/append behavior
- block paste uses a dedicated blockwise paste path
- short target lines are space-padded to the target column
- paste can extend past the end of the current document by creating lines

Current rendering path exposed to Slint:

- Rust computes `line_selection_cols()`
- `AppController` exports `selected_prefix` and `selected_text`
- visual and yank highlights render independently from search highlight

## Registers

Implemented explicit register targets:

- unnamed `"`
- named `a-z`
- uppercase append `A-Z`
- clipboard `+`
- black-hole `_`

Implemented implicit registers:

- yank register `0`
- small delete `-`
- numbered delete `1-9`

Register metadata:

- `linewise`
- `blockwise`

Current register behavior:

- unnamed yanks also update `0`
- unnamed small characterwise deletes update `-`
- unnamed non-small deletes rotate `1-9` and write `1`
- named writes also update unnamed
- uppercase named writes append into the lowercase target register and update unnamed
- clipboard writes also update unnamed
- black-hole drops the write
- linewise writes normalize with a trailing `\n`

Paste behavior:

- linewise registers paste linewise
- blockwise registers paste blockwise
- other registers paste charwise

## Search

Implemented search entry and navigation:

- `/`
- `?`
- `return`
- `backspace`
- `delete`
- cursor movement and history while entering the search
- `escape`
- `ctrl+[`
- `n`
- `N`
- `*`
- `#`

Search options currently wired in document settings:

- `ignorecase`
- `smartcase`
- `hlsearch`
- `incsearch`

Current search semantics:

- plain substring matching, not regex
- case handling respects `ignorecase` and `smartcase`
- `n` follows stored direction
- `N` reverses stored direction
- `*` and `#` use the current small-word text object
- `incsearch` refreshes search state while editing the query
- `hlsearch` controls whether matches are shown as active highlights

## Command Mode And Ex Commands

Command-line editing currently supports:

- printable insert at command cursor
- `backspace`
- `delete`
- `left`, `right`
- `home`, `end`
- `ctrl+b`, `ctrl+e`
- `ctrl+u`
- `ctrl+w`
- `up`, `down` history
- `return`
- `escape`
- `ctrl+[`

Separate histories are maintained for command mode and search mode.

Implemented ex commands:

- `:w`
- `:w {path}`
- `:wa`
- `:wall`
- `:q`
- `:q!`
- `:wq`
- `:e {path}`
- `:enew`
- `:{number}`
- `:noh`
- `:nohlsearch`
- `:set {option}`
- `:set no{option}`
- `:set {option}={value}`
- `:reg`
- `:registers`
- `:marks`
- `:jumps`
- `:changes`
- `:s/pattern/replacement/flags`
- `:%s/pattern/replacement/flags`
- `:'<,'>s/pattern/replacement/flags`
- `:{start},{end}s/pattern/replacement/flags`
- `:{start},$s/pattern/replacement/flags`
- `:&`
- `:&&`

Implemented `:set` keys:

- `number`, `nu`
- `relativenumber`, `rnu`
- `wrap`
- `tabstop`, `ts`
- `shiftwidth`, `sw`
- `ignorecase`, `ic`
- `smartcase`, `scs`
- `hlsearch`, `hls`
- `incsearch`, `is`

Substitute flags:

- `g`
- `i`
- `c`

Substitute-confirm input:

- `y`
- `n`
- `a`
- `l`
- `q`
- `escape`
- `ctrl+[`

Ex actions emitted to `AppController`:

- save current document
- save all documents
- quit current document or app
- save and quit
- edit path
- new blank document
- show text output for inspection commands

## Marks, Jumps, Changelist, And Repeat

Implemented marks and jumps:

- `m{a-z}`
- `'{a-z}` line jump
- `` `{a-z} `` exact jump
- `ctrl+o`
- `ctrl+i`

Current jump-list push sites include:

- mark jumps
- search execution and repeats
- `%`, `{`, `}`, `(`, `)`
- `G`, `gg`
- `H`, `M`, `L`
- `ctrl+d`, `ctrl+u`, `ctrl+f`, `ctrl+b`

Implemented changelist behavior:

- mutation sites push into a Rust-owned changelist
- `g;` moves backward through changes
- `g,` moves forward through changes

Repeat behavior:

- `.` replays `last_change`
- pure yanks, `u`, `ctrl+r`, command entry, and search entry are excluded from repeat capture

## App-Level Integration

`AppController` currently adds:

- clipboard import into unnamed before `p`/`P` when sync is enabled
- clipboard export after unnamed-register changes
- ex-action handling for save/open/quit/new/save-all/message display
- pointer cursor placement and row-drag selection for the Slint view
- status-bar output for command input, search input, substitution prompts, dirty state, and current search pattern

Mode-to-theme mapping:

- `NORMAL` -> normal color
- `INSERT` -> insert color
- `VISUAL`, `V-LINE`, `V-BLOCK` -> visual color
- `COMMAND`, `/`, `?` -> command color
- `REPLACE` -> replace color

## Still Missing

These are not complete yet even though some scaffolding exists:

- macro recording and replay (`q{a-z}`, `q`, `@{a-z}`, `@@`)
- Unicode-aware separation of byte index, character column, and visual column
- full Vim/Neovim regex search and substitute semantics

## Current Quirks

These are useful implementation reminders:

- search is still substring-based, not regex-based
- blockwise paste exists, but visual columns are still char-count based rather than true display-column aware
- insert-register paste currently inserts register text directly through the existing insert path, including linewise text with trailing newlines
