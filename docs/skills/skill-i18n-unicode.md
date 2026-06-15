# skill-i18n-unicode

NeoNote's current multilingual and Unicode support follows these patterns:

- Persist the UI language in `AppConfig.language` using the `AppLanguage` enum from `src/i18n.rs`.
- Keep translation lookup in Rust, not in `.slint` logic. `AppLanguage::strings()` is the current source of localized shell labels.
- Send localized labels into Slint through a single `UiText` struct property on `AppWindow`.
- Keep dynamic note/editor behavior in Rust. Localized status messages and untitled labels are produced in `AppController`.
- The editor buffer remains Rust-owned and Unicode-safe at the `char` level. String mutation helpers must convert char indices to byte indices before slicing or replacing.
- File open/save preserves the detected text encoding through `TextBuffer.encoding`.
- Current file encoding support covers UTF-8, UTF-8 with BOM, UTF-16 LE with BOM, and UTF-16 BE with BOM.
- When adding search or case-insensitive matching, do not lowercase the entire haystack and then reuse those indices. That can shift positions for Unicode characters whose lowercase form expands. Compare lowered windows against lowered patterns while keeping original char offsets.

Current limitation:

- Cursor movement and editing are Unicode-scalar-value based, not grapheme-cluster based. Complex emoji or combining-mark clusters may still behave as multiple editor columns until a future grapheme-aware pass is implemented.
