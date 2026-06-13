use arboard::Clipboard;

pub fn read_text() -> Result<String, arboard::Error> {
    Clipboard::new()?.get_text()
}

pub fn write_text(text: &str) -> Result<(), arboard::Error> {
    Clipboard::new()?.set_text(text.to_string())
}
