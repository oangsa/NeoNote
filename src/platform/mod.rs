pub mod clipboard;
pub mod effects;
pub mod file_association;
pub mod fonts;
pub mod ipc;
pub mod window_effects;

#[cfg(target_os = "windows")]
pub fn get_hwnd_from_slint(_window: &slint::Window) -> Option<windows::Win32::Foundation::HWND> {
    window_effects::find_main_window_hwnd()
}
