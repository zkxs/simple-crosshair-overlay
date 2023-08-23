// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Platform-specific implementations

#[cfg(not(target_os = "windows"))]
pub use generic::{get_foreground_window, set_foreground_window, WindowHandle};
#[cfg(target_os = "windows")]
pub use windows::{get_foreground_window, set_foreground_window, WindowHandle};

#[cfg(not(target_os = "windows"))]
mod generic;

#[cfg(target_os = "windows")]
mod windows;
