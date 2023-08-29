// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Platform-specific implementations

use std::fmt::Debug;

#[cfg(not(target_os = "windows"))]
pub use generic::{get_foreground_window, set_foreground_window, WindowHandle};
pub use generic::HotkeyManager;
#[cfg(target_os = "windows")]
pub use windows::{get_foreground_window, set_foreground_window, WindowHandle};

use crate::hotkey::Keycode;

pub mod generic; // pub so benchmarking can access

#[cfg(target_os = "windows")]
pub mod windows; // pub so benchmarking can access

/// `T` is the type used to represent keycodes internally
pub trait KeyboardState<T>: Default where T: KeycodeType {
    /// update internal keyboard state from keyboard
    fn poll(&mut self);

    fn get_state(&self) -> &[T];
}

pub trait KeycodeType: From<Keycode> + TryInto<Keycode> + Debug {
    /// maximum possible number of distinct keycode variants
    fn num_variants() -> usize;

    /// Convert a keycode into an index for a lookup table
    fn index(&self) -> usize;
}
