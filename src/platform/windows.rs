//! Windows-specific implementations.
//! This is only in the module tree on Windows targets.

use winapi::shared::windef::HWND;

/// null-safe window handle
#[derive(Copy, Clone, Debug)]
pub struct WindowHandle {
    hwnd: HWND,
}

impl WindowHandle {
    /// must not be called with a null pointer
    fn new(hwnd: HWND) -> WindowHandle {
        debug_assert!(!hwnd.is_null());
        WindowHandle { hwnd }
    }

    /// will never return null pointer
    fn hwnd(self) -> HWND {
        debug_assert!(!self.hwnd.is_null());
        self.hwnd
    }
}

/// wrapper around https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow
///
/// this converts null pointers into None
pub fn get_foreground_window() -> Option<WindowHandle> {
    unsafe {
        match winapi::um::winuser::GetForegroundWindow() {
            hwnd if hwnd.is_null() => None,
            hwnd => Some(WindowHandle::new(hwnd)),
        }
    }
}

/// wrapper around https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow
///
/// this does not handle null pointers, as it shouldn't be possible to get a null inside a `WindowHandle`.
/// `true` is returned if the foreground window was set successfully.
pub fn set_foreground_window(window_handle: WindowHandle) -> bool {
    unsafe {
        winapi::um::winuser::SetForegroundWindow(window_handle.hwnd()) != 0
    }
}
