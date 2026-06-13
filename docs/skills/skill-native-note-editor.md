# skill-native-note-editor

NeoNote's normal note workflow is native Slint rendering backed by an in-process Vim-style editing layer. It must not spawn Neovide, Neovim, terminal windows, or renderer child processes for core editing.

Use `NoteDocument` and future `src/vim/` modules as the editor core. The default editor experience should preserve:

- Vim-style modal key handling implemented in pure Rust
- In-app cursor/mode feedback
- Embedded-in-app behavior with no external editor or renderer process
- Slint UI callbacks that route into Rust controller methods

User-facing file workflows should still be note-app actions:

- New File
- Open File
- Save
- Save As
- Recent Files

Use `.txt` as the default note extension in file dialogs and default save names.

Do not call `NeovideInstance`, `TabManager::open_tab`, `NvimEngine`, or RPC clients from New File, Open File, Save, Save As, Recent Files, launcher note actions, or ordinary note editing.
