# skill-selection-highlight-rendering

NeoNote selection highlights should render as connected editor-style shapes, not a stack of unrelated row rectangles.

Validated approach in this repo:

- Keep selection logic in Rust and derive highlight segments from the existing snapshot pipeline.
- Group vertically adjacent segments into one logical selection highlight group before rendering.
- Preserve per-line geometry in columns, including `0.5` cell overshoot only for non-empty line-ending character/line selections.
- Keep empty selected lines as one-cell markers.
- Keep Visual Block exact; do not add overshoot there.

Slint rendering pattern validated here:

- Use `Path` items bound from a model with `commands: shape.path-data`.
- Use `viewbox-width` and `viewbox-height` so Rust can emit normalized path coordinates in cell/line units.
- Let Slint scale the path with `char-width` and `line-height` at render time.
- Paint the highlight behind text and cursor.

Do not reintroduce:

- viewport-wide fill
- longest-line right-edge bridges
- vertical overlap hacks between rows
- per-row-only ownership of highlight shape

When selection widths change between adjacent lines:

- wide-to-narrow transitions should curve inward
- narrow-to-wide transitions should curve outward
- exposed outer corners should stay generously rounded
