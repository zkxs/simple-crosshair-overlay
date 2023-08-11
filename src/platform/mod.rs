//! Platform-specific implementations

#[cfg(not(target_os = "windows"))]
pub use generic::{get_foreground_window, set_foreground_window, WindowHandle};
#[cfg(target_os = "windows")]
pub use windows::{get_foreground_window, set_foreground_window, WindowHandle};

#[cfg(not(target_os = "windows"))]
mod generic;

#[cfg(target_os = "windows")]
mod windows;
