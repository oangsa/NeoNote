#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NormalKey<'a> {
    EnterNormal,
    Input(&'a str),
    Ignore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertKey<'a> {
    EnterNormal,
    Cancel,
    RegisterPaste,
    Newline,
    Backspace,
    Delete,
    DeleteWord,
    DeleteLine,
    Text(&'a str),
    Ignore,
}

pub fn normalize_normal_key(key: &str) -> NormalKey<'_> {
    match key {
        "escape" | "ctrl+[" => NormalKey::EnterNormal,
        "left" => NormalKey::Input("h"),
        "right" => NormalKey::Input("l"),
        "up" => NormalKey::Input("k"),
        "down" => NormalKey::Input("j"),
        "return" | "backspace" => NormalKey::Input(key),
        "delete" => NormalKey::Ignore,
        value => NormalKey::Input(value),
    }
}

pub fn normalize_insert_key(key: &str) -> InsertKey<'_> {
    match key {
        "escape" | "ctrl+[" => InsertKey::EnterNormal,
        "ctrl+c" => InsertKey::Cancel,
        "ctrl+r" => InsertKey::RegisterPaste,
        "ctrl+w" => InsertKey::DeleteWord,
        "ctrl+u" => InsertKey::DeleteLine,
        "return" => InsertKey::Newline,
        "backspace" => InsertKey::Backspace,
        "delete" => InsertKey::Delete,
        "left" | "right" | "up" | "down" => InsertKey::Ignore,
        text if is_printable_editor_text(text) => InsertKey::Text(text),
        _ => InsertKey::Ignore,
    }
}

fn is_printable_editor_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| !ch.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_key_normalizes_navigation_and_ignores_editing_controls() {
        assert_eq!(normalize_normal_key("escape"), NormalKey::EnterNormal);
        assert_eq!(normalize_normal_key("ctrl+["), NormalKey::EnterNormal);
        assert_eq!(normalize_normal_key("left"), NormalKey::Input("h"));
        assert_eq!(normalize_normal_key("right"), NormalKey::Input("l"));
        assert_eq!(normalize_normal_key("up"), NormalKey::Input("k"));
        assert_eq!(normalize_normal_key("down"), NormalKey::Input("j"));
        assert_eq!(normalize_normal_key("return"), NormalKey::Input("return"));
        assert_eq!(
            normalize_normal_key("backspace"),
            NormalKey::Input("backspace")
        );
        assert_eq!(normalize_normal_key("x"), NormalKey::Input("x"));
    }

    #[test]
    fn insert_key_separates_controls_from_printable_text() {
        assert_eq!(normalize_insert_key("escape"), InsertKey::EnterNormal);
        assert_eq!(normalize_insert_key("ctrl+["), InsertKey::EnterNormal);
        assert_eq!(normalize_insert_key("ctrl+r"), InsertKey::RegisterPaste);
        assert_eq!(normalize_insert_key("return"), InsertKey::Newline);
        assert_eq!(normalize_insert_key("backspace"), InsertKey::Backspace);
        assert_eq!(normalize_insert_key("delete"), InsertKey::Delete);
        assert_eq!(normalize_insert_key("left"), InsertKey::Ignore);
        assert_eq!(normalize_insert_key("a"), InsertKey::Text("a"));
        assert_eq!(normalize_insert_key("\n"), InsertKey::Ignore);
    }
}
