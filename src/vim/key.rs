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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditorKey {
    EnterNormal,
    Cancel,
    RegisterPaste,
    Newline,
    Backspace,
    Delete,
    DeleteWord,
    DeleteLine,
    Input(String),
    Ignore,
}

pub fn normalize_normal_key(key: &str) -> EditorKey {
    match key {
        "escape" | "ctrl+[" => EditorKey::EnterNormal,
        "left" => EditorKey::Input("h".to_string()),
        "right" => EditorKey::Input("l".to_string()),
        "up" => EditorKey::Input("k".to_string()),
        "down" => EditorKey::Input("j".to_string()),
        "return" | "backspace" => EditorKey::Input(key.to_string()),
        "delete" => EditorKey::Ignore,
        value => {
            if value.is_empty() || value.chars().any(|c| c.is_control()) {
                EditorKey::Ignore
            } else {
                EditorKey::Input(value.to_string())
            }
        }
    }
}

pub fn normalize_insert_key(key: &str) -> EditorKey {
    match key {
        "escape" | "ctrl+[" => EditorKey::EnterNormal,
        "ctrl+c" => EditorKey::Cancel,
        "ctrl+r" => EditorKey::RegisterPaste,
        "ctrl+w" => EditorKey::DeleteWord,
        "ctrl+u" => EditorKey::DeleteLine,
        "return" => EditorKey::Newline,
        "backspace" => EditorKey::Backspace,
        "delete" => EditorKey::Delete,
        "left" | "right" | "up" | "down" => EditorKey::Ignore,
        text if is_printable_editor_text(text) => EditorKey::Input(text.to_string()),
        _ => EditorKey::Ignore,
    }
}

fn is_printable_editor_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| !ch.is_control())
}

pub fn normalize_command_key(key: &str) -> EditorKey {
    match key.to_lowercase().as_str() {
        "escape" | "ctrl+[" | "ctrl+c" => EditorKey::EnterNormal,
        "backspace" => EditorKey::Input("backspace".to_string()),
        "delete" => EditorKey::Input("delete".to_string()),
        "return" | "enter" => EditorKey::Input("return".to_string()),
        "up" => EditorKey::Input("up".to_string()),
        "down" => EditorKey::Input("down".to_string()),
        "left" => EditorKey::Input("left".to_string()),
        "right" => EditorKey::Input("right".to_string()),
        "ctrl+r" => EditorKey::Input("ctrl+r".to_string()),
        _ => {
            if key.chars().count() == 1 && !key.chars().any(|c| c.is_control()) {
                EditorKey::Input(key.to_string())
            } else {
                EditorKey::Ignore
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_key_normalizes_navigation_and_ignores_editing_controls() {
        assert_eq!(normalize_normal_key("escape"), EditorKey::EnterNormal);
        assert_eq!(normalize_normal_key("ctrl+["), EditorKey::EnterNormal);
        assert_eq!(normalize_normal_key("left"), EditorKey::Input("h".to_string()));
        assert_eq!(normalize_normal_key("right"), EditorKey::Input("l".to_string()));
        assert_eq!(normalize_normal_key("up"), EditorKey::Input("k".to_string()));
        assert_eq!(normalize_normal_key("down"), EditorKey::Input("j".to_string()));
        assert_eq!(normalize_normal_key("return"), EditorKey::Input("return".to_string()));
        assert_eq!(
            normalize_normal_key("backspace"),
            EditorKey::Input("backspace".to_string())
        );
        assert_eq!(normalize_normal_key("x"), EditorKey::Input("x".to_string()));
    }

    #[test]
    fn insert_key_separates_controls_from_printable_text() {
        assert_eq!(normalize_insert_key("escape"), EditorKey::EnterNormal);
        assert_eq!(normalize_insert_key("ctrl+["), EditorKey::EnterNormal);
        assert_eq!(normalize_insert_key("ctrl+r"), EditorKey::RegisterPaste);
        assert_eq!(normalize_insert_key("return"), EditorKey::Newline);
        assert_eq!(normalize_insert_key("backspace"), EditorKey::Backspace);
        assert_eq!(normalize_insert_key("delete"), EditorKey::Delete);
        assert_eq!(normalize_insert_key("left"), EditorKey::Ignore);
        assert_eq!(normalize_insert_key("a"), EditorKey::Input("a".to_string()));
        assert_eq!(normalize_insert_key("\n"), EditorKey::Ignore);
    }
}
