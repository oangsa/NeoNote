#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowHandle(pub isize);

#[cfg(windows)]
pub fn find_window_for_pid(pid: u32) -> Option<WindowHandle> {
    use windows::Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
        },
    };

    struct Search {
        pid: u32,
        hwnd: Option<HWND>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        let mut window_pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

        let has_owner = GetWindow(hwnd, GW_OWNER)
            .map(|owner| !owner.0.is_null())
            .unwrap_or(false);

        if window_pid == search.pid && IsWindowVisible(hwnd).as_bool() && !has_owner {
            search.hwnd = Some(hwnd);
            return BOOL(0);
        }

        BOOL(1)
    }

    let mut search = Search { pid, hwnd: None };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut search as *mut Search as isize));
    }

    search.hwnd.map(|hwnd| WindowHandle(hwnd.0 as isize))
}

#[cfg(not(windows))]
pub fn find_window_for_pid(_pid: u32) -> Option<WindowHandle> {
    None
}

#[cfg(windows)]
pub fn move_window(hwnd: WindowHandle, width: i32, height: i32) {
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::MoveWindow};

    unsafe {
        let _ = MoveWindow(
            HWND(hwnd.0 as *mut core::ffi::c_void),
            0,
            0,
            width.max(0),
            height.max(0),
            true,
        );
    }
}

#[cfg(not(windows))]
pub fn move_window(_hwnd: WindowHandle, _width: i32, _height: i32) {}

#[cfg(windows)]
pub fn show_window(hwnd: WindowHandle, show: bool) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW},
    };

    unsafe {
        let _ = ShowWindow(
            HWND(hwnd.0 as *mut core::ffi::c_void),
            if show { SW_SHOW } else { SW_HIDE },
        );
    }
}

#[cfg(not(windows))]
pub fn show_window(_hwnd: WindowHandle, _show: bool) {}
