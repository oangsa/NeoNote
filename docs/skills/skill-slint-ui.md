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

Build pattern validated in this repo:

- Add `slint` as a runtime dependency and `slint-build` as a build dependency.
- Compile the root `.slint` file from `build.rs` with `slint_build::compile("ui/app-window.slint")`.
- Include generated Rust bindings with `slint::include_modules!()` in `src/main.rs`.
- Use `ComponentHandle::as_weak()` inside callbacks so closures can update the window after controller mutations without ownership cycles.

Theme binding pattern validated in this repo:

- Use Slint `in property <brush>` fields for resolved palette tokens.
- Keep the active theme in `ThemeStore`; expose a compact Rust snapshot for Slint.
- Convert theme JSON hex strings to `slint::Brush` in the Rust bridge with `Color::from_rgb_u8`.
- Avoid naming custom component properties the same as inherited Slint properties such as `border-color`.
- Do not set `x` or `y` on children owned by a `HorizontalLayout` or `VerticalLayout`; the layout owns those coordinates.
- For configurable lengths such as editor font size or line height, prefer Slint `float` properties from Rust and multiply by `1px` at the Slint use site. This avoids depending on version-specific Rust length wrapper exports.
- For user-visible numeric values, send preformatted string labels from Rust instead of binding raw `float` values directly to `Text`; raw float conversion can expose awkward precision such as `1.4000`.
- In dense rows with labels plus trailing controls, reserve a fixed-width trailing action/value column and elide long text. This keeps settings and theme panel controls aligned at smaller panel widths.
