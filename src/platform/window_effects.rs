use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMSBT_MAINWINDOW, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
};

pub fn find_main_window_hwnd() -> Option<HWND> {
    let current_pid = unsafe { GetCurrentProcessId() };
    let mut found: Option<HWND> = None;
    let found_ptr: *mut Option<HWND> = &mut found;

    let cb_data: (u32, *mut Option<HWND>) = (current_pid, found_ptr);
    let lparam = LPARAM(&cb_data as *const _ as isize);

    let _ = unsafe { EnumWindows(Some(enum_windows_callback), lparam) };
    found
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data_ptr = lparam.0 as *const (u32, *mut Option<HWND>);
    let (target_pid, found_ptr) = unsafe { ((*data_ptr).0, (*data_ptr).1) };

    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    if pid == target_pid && unsafe { IsWindowVisible(hwnd).as_bool() } {
        unsafe { *found_ptr = Some(hwnd) };
        return BOOL(0);
    }

    BOOL(1)
}

pub fn apply_mica_for_hwnd(hwnd: HWND, enabled: bool) -> anyhow::Result<()> {
    if !enabled {
        return Ok(());
    }

    if try_set_backdrop(hwnd, DWMSBT_MAINWINDOW).is_ok() {
        return Ok(());
    }

    let _ = try_set_backdrop(hwnd, DWMSBT_TRANSIENTWINDOW);
    Ok(())
}

fn try_set_backdrop(
    hwnd: HWND,
    backdrop_type: windows::Win32::Graphics::Dwm::DWM_SYSTEMBACKDROP_TYPE,
) -> anyhow::Result<()> {
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_type as *const _ as *const _,
            std::mem::size_of::<windows::Win32::Graphics::Dwm::DWM_SYSTEMBACKDROP_TYPE>() as u32,
        )
    }
    .map_err(|e| anyhow::anyhow!("DwmSetWindowAttribute failed: {e}"))?;

    Ok(())
}
