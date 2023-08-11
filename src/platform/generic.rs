//! Platform-agnostic implementations.
//! This is only in the module tree on targets lacking a platform-specific implementation.
//! On platforms that do not support the operation they will no-op and indicate that the action failed.

/// platform-independent window handle (it's nothing)
#[derive(Copy, Clone, Debug)]
pub struct WindowHandle;

/// Always returns `None`, as this requires a platform-specific implementation.
pub fn get_foreground_window() -> Option<WindowHandle> {
    None
}

/// Always no-ops and returns `false` for the result (indicating failure), as this requires a platform-specific implementation.
pub fn set_foreground_window(_window_handle: WindowHandle) -> bool {
   false
}
