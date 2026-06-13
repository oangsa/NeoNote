use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory, IDWriteFontCollection,
};
use windows::Win32::Foundation::BOOL;
use windows::core::PCWSTR;

pub fn monospace_fonts() -> Vec<String> {
    vec!["JetBrains Mono".to_string(), "Consolas".to_string()]
}

pub fn check_font_exists(family_name: &str) -> bool {
    unsafe {
        let factory: IDWriteFactory = match DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut font_collection: Option<IDWriteFontCollection> = None;
        if factory.GetSystemFontCollection(&mut font_collection, false).is_err() {
            return false;
        }
        let Some(font_collection) = font_collection else {
            return false;
        };
        let mut name_utf16: Vec<u16> = family_name.encode_utf16().collect();
        name_utf16.push(0); // null terminator

        let mut index = 0u32;
        let mut exists = BOOL(0);
        let _ = font_collection.FindFamilyName(
            PCWSTR::from_raw(name_utf16.as_ptr()),
            &mut index,
            &mut exists,
        );
        exists.as_bool()
    }
}

pub fn select_editor_font(preferred: &str) -> String {
    if check_font_exists(preferred) {
        return preferred.to_string();
    }
    for font in &["Consolas", "Courier New", "Lucida Console"] {
        if check_font_exists(font) {
            return font.to_string();
        }
    }
    // Absolute fallback
    "Courier New".to_string()
}
