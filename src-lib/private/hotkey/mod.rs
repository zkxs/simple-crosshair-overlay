// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Keyboard reading system built to read hotkeys without a focused window.

pub use hotkey_manager::HotkeyManager;
pub use hotkey_manager::KeyBindings;
pub(crate) use keycode::Keycode; // needs to be pub(crate) so the platform-specific implementations can implement From conversions

mod hotkey_manager;
mod keycode;
