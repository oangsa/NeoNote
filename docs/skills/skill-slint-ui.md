# skill-slint-ui

NeoNote's target UI toolkit is Slint.

Use Slint for:

- app window layout
- custom title/menu/status areas
- launcher
- editor view
- settings and theme panels
- toasts and lightweight popups
- theme token binding

Keep Rust responsible for:

- Vim/editor logic
- file dialogs and file I/O
- persistence
- platform effects
- theme loading and validation
- transforming editor state into Slint-friendly properties/models

Rules:

- Do not add new egui/eframe UI surfaces.
- Keep `.slint` files focused on presentation and interaction callbacks.
- Route Slint callbacks into narrow Rust controller methods.
- Do not let a Slint text input widget own the core Vim caret; render the editor from Rust-owned buffer/cursor state.
- Expose theme colors through a single Slint-facing theme state instead of scattering hardcoded colors.
