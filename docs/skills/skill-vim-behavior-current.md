# skill-vim-behavior-current

This skill inventories the Vim behavior currently implemented in NeoNote from code only.

Code sources used:

- `src/notes.rs`
- `src/vim/key.rs`
- `src/app.rs`

No existing skill files were used to build this document.

## Mode Model

Implemented modes:

- `Normal`
- `Insert`
- `Visual`
- `VisualLine`
- `VisualBlock`
- `Command`
- `Search(Forward)`
- `Search(Backward)`
- `Replace`

`NoteDocument` currently owns:

- buffer text, path, dirty/open state
- cursor line and column
- count prefix
- pending commands/operators
- registers
- undo/redo stacks
- last repeatable change
- visual anchor and last visual selection
- search state
- marks
- jumplist
- changelist
- viewport state
- command/search history state
- per-document settings

## Input Normalization

Normal-side normalization in `src/vim/key.rs`:

- `escape`, `ctrl+[`
- arrows -> `h`, `j`, `k`, `l`
- `return`, `backspace` forwarded as normal inputs
- `delete` ignored

Insert/replace-side normalization:

- `escape`, `ctrl+[`
- `ctrl+c`
- `ctrl+r`
- `ctrl+w`
- `ctrl+u`
- `return`
- `backspace`
- `delete`
- printable text
- arrows ignored

`AppController` routes both `Insert` and `Replace` through insert-key handling.

## Normal Mode

Implemented mode entry and switching:

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
- `f`, `F`, `t`, `T`
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

Implemented search and mark commands:

- `n`, `N`
- `*`, `#`
- `m{a-z}`
- `'{a-z}`
- `` `{a-z} ``
- `ctrl+o`
- `ctrl+i`

Implemented `g`-prefix behavior:

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

Count handling:

- counts parse before commands
- `0` is a motion only when no count is in progress

## Insert And Replace

Insert mode supports:

- direct text insertion
- `return`
- `backspace`
- `delete`
- `ctrl+w`
- `ctrl+u`
- `ctrl+r {register}`
- `escape`
- `ctrl+[`
- `ctrl+c`

Current insert behavior:

- `backspace` joins upward at column `0`
- `delete` joins downward at line end
- `ctrl+w` deletes the previous word segment
- `ctrl+u` deletes to column `0` or joins upward
- `ctrl+r {register}` inserts register text without leaving insert mode

Replace mode supports:

- normal-mode `R` entry
- overwrite-on-type
- append past line end
- exit with `escape` or `ctrl+[`

Repeat capture:

- insert/replace replay stores net inserted text and edit controls
- `last_insert_pos` is tracked for `gi`

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

Implemented doubled forms:

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
- paragraph objects are blank-line delimited
- line objects are linewise

## Visual, Visual Line, And Visual Block

Implemented visual entry:

- `v`
- `V`
- `ctrl+v`

Implemented visual motions:

- `h`, `j`, `k`, `l`
- `w`, `W`, `b`, `B`, `e`, `E`
- `0`, `^`, `$`
- `|`
- `+`, `-`, `_`
- `G`
- counts on the supported visual motions
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

Visual command/search entry:

- `:`
- `/`
- `?`

Visual command behavior:

- stores last visual selection
- exits visual mode
- prefills `'<,'>`

Visual block behavior:

- block yank
- block delete
- block change
- block `I`
- block `A`
- deferred multi-line block insert/append replay
- blockwise register metadata
- dedicated blockwise paste path
- pad short lines with spaces
- create new lines if block paste extends past EOF

Current Slint-facing selection model:

- `line_selection_cols()` computes per-line spans
- `AppController` exports `selected_prefix` and `selected_text`

## Registers

Implemented register targets:

- unnamed `"`
- named `a-z`
- uppercase append `A-Z`
- clipboard `+`
- black-hole `_`

Implemented implicit registers:

- yank `0`
- small delete `-`
- numbered deletes `1-9`

Register metadata:

- `linewise`
- `blockwise`

Current behavior:

- unnamed yanks also write `0`
- unnamed small character deletes write `-`
- unnamed non-small deletes rotate `1-9` and write `1`
- named writes update unnamed
- uppercase named writes append into lowercase named registers and update unnamed
- clipboard writes update unnamed
- black-hole drops the write
- linewise text is normalized with trailing newline

Paste behavior:

- linewise registers paste linewise
- blockwise registers paste blockwise
- others paste charwise

## Search

Implemented search entry/navigation:

- `/`
- `?`
- `return`
- `backspace`
- `delete`
- cursor movement/history while editing the prompt
- `escape`
- `ctrl+[`
- `n`
- `N`
- `*`
- `#`

Implemented search-related options:

- `ignorecase`
- `smartcase`
- `hlsearch`
- `incsearch`

Current semantics:

- plain substring search, not regex
- case handling respects `ignorecase` and `smartcase`
- `n` follows stored direction
- `N` reverses stored direction
- `*` and `#` search the current small-word text object
- `incsearch` refreshes matches while editing
- `hlsearch` controls visible highlighting

## Command Mode And Ex Commands

Command-line editing currently supports:

- insert at cursor
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

Separate histories exist for command and search prompts.

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

Substitute confirm input:

- `y`
- `n`
- `a`
- `l`
- `q`
- `escape`
- `ctrl+[`

Ex actions routed through `AppController`:

- save current doc
- save all docs
- quit
- save and quit
- edit path
- new blank doc
- show message output for inspection commands

## Marks, Jumps, Changelist, And Repeat

Implemented marks/jumps:

- `m{a-z}`
- `'{a-z}` line jump
- `` `{a-z} `` exact jump
- `ctrl+o`
- `ctrl+i`

Jump-list push sites currently include:

- mark jumps
- search execution/repeat
- `%`, `{`, `}`, `(`, `)`
- `G`, `gg`
- `H`, `M`, `L`
- `ctrl+d`, `ctrl+u`, `ctrl+f`, `ctrl+b`

Changelist:

- mutation sites push change locations
- `g;` moves backward
- `g,` moves forward

Repeat:

- `.` replays `last_change`
- yanks, `u`, `ctrl+r`, command entry, and search entry are excluded from repeat capture

## App-Level Integration

`AppController` currently adds:

- clipboard import into unnamed before `p`/`P` when sync is enabled
- clipboard export after unnamed-register changes
- ex-action routing
- pointer cursor placement
- row-drag selection in the Slint view
- status output for command/search/substitute prompts and document state

Mode-to-theme mapping:

- `NORMAL`
- `INSERT`
- `VISUAL`
- `V-LINE`
- `V-BLOCK`
- `COMMAND`
- `/`
- `?`
- `REPLACE`

## Still Missing

Not complete yet:

- macro recording/replay is only scaffolded
- Unicode-aware visual-column model is not implemented
- regex search/substitute semantics are not implemented

## Current Quirks

- search is still substring-based
- visual/block behavior is char-count based, not true display-column based
- insert register paste inserts raw register text, including linewise trailing newlines
