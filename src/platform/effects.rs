use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWM_SYSTEMBACKDROP_TYPE, DWMSBT_MAINWINDOW, DWMSBT_NONE,
    DWMWINDOWATTRIBUTE, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW,
    GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
};

pub fn set_mica_backdrop(hwnd: HWND, enabled: bool) {
    let backdrop = if enabled {
        DWMSBT_MAINWINDOW
    } else {
        DWMSBT_NONE
    };

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const _,
            std::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );
    }
}

pub fn set_window_opacity(hwnd: HWND, opacity_percent: u8) {
    let opacity = opacity_percent.clamp(40, 100);
    let alpha = ((opacity as f32 / 100.0) * 255.0).round() as u8;

    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as isize);
        let _ = SetLayeredWindowAttributes(hwnd, None, alpha, LWA_ALPHA);
    }
}

pub fn set_dark_mode_titlebar(hwnd: HWND) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &1u32 as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

pub fn set_rounded_corners(hwnd: HWND) {
    const DWMWA_WINDOW_CORNER_PREFERENCE: i32 = 33;
    const DWMWCP_ROUND: u32 = 1;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_WINDOW_CORNER_PREFERENCE),
            &DWMWCP_ROUND as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}
