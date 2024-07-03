// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Platform-agnostic implementations.
//! This is only in the module tree on targets lacking a platform-specific implementation.
//! On platforms that do not support the operation they will no-op and indicate that the action failed.

use device_query::{DeviceQuery, DeviceState, Keycode as DeviceQueryKeycode};

use crate::private::hotkey;
use crate::private::hotkey::{KeyBindings, Keycode};
use crate::private::platform::{KeyboardState, KeycodeType};

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

pub struct DeviceQueryKeyboardState {
    device_state: DeviceState,
    keys: Vec<DeviceQueryKeycode>,
}

impl Default for DeviceQueryKeyboardState {
    fn default() -> Self {
        Self {
            device_state: DeviceState::new(),
            keys: Vec::new(),
        }
    }
}

impl KeyboardState<DeviceQueryKeycode> for DeviceQueryKeyboardState {
    fn poll(&mut self) {
        self.keys = self.device_state.get_keys();
    }

    fn get_state(&self) -> &[DeviceQueryKeycode] {
        &self.keys
    }
}

impl From<DeviceQueryKeycode> for Keycode {
    fn from(value: DeviceQueryKeycode) -> Self {
        match value {
            DeviceQueryKeycode::Key0 => Keycode::Key0,
            DeviceQueryKeycode::Key1 => Keycode::Key1,
            DeviceQueryKeycode::Key2 => Keycode::Key2,
            DeviceQueryKeycode::Key3 => Keycode::Key3,
            DeviceQueryKeycode::Key4 => Keycode::Key4,
            DeviceQueryKeycode::Key5 => Keycode::Key5,
            DeviceQueryKeycode::Key6 => Keycode::Key6,
            DeviceQueryKeycode::Key7 => Keycode::Key7,
            DeviceQueryKeycode::Key8 => Keycode::Key8,
            DeviceQueryKeycode::Key9 => Keycode::Key9,
            DeviceQueryKeycode::A => Keycode::A,
            DeviceQueryKeycode::B => Keycode::B,
            DeviceQueryKeycode::C => Keycode::C,
            DeviceQueryKeycode::D => Keycode::D,
            DeviceQueryKeycode::E => Keycode::E,
            DeviceQueryKeycode::F => Keycode::F,
            DeviceQueryKeycode::G => Keycode::G,
            DeviceQueryKeycode::H => Keycode::H,
            DeviceQueryKeycode::I => Keycode::I,
            DeviceQueryKeycode::J => Keycode::J,
            DeviceQueryKeycode::K => Keycode::K,
            DeviceQueryKeycode::L => Keycode::L,
            DeviceQueryKeycode::M => Keycode::M,
            DeviceQueryKeycode::N => Keycode::N,
            DeviceQueryKeycode::O => Keycode::O,
            DeviceQueryKeycode::P => Keycode::P,
            DeviceQueryKeycode::Q => Keycode::Q,
            DeviceQueryKeycode::R => Keycode::R,
            DeviceQueryKeycode::S => Keycode::S,
            DeviceQueryKeycode::T => Keycode::T,
            DeviceQueryKeycode::U => Keycode::U,
            DeviceQueryKeycode::V => Keycode::V,
            DeviceQueryKeycode::W => Keycode::W,
            DeviceQueryKeycode::X => Keycode::X,
            DeviceQueryKeycode::Y => Keycode::Y,
            DeviceQueryKeycode::Z => Keycode::Z,
            DeviceQueryKeycode::F1 => Keycode::F1,
            DeviceQueryKeycode::F2 => Keycode::F2,
            DeviceQueryKeycode::F3 => Keycode::F3,
            DeviceQueryKeycode::F4 => Keycode::F4,
            DeviceQueryKeycode::F5 => Keycode::F5,
            DeviceQueryKeycode::F6 => Keycode::F6,
            DeviceQueryKeycode::F7 => Keycode::F7,
            DeviceQueryKeycode::F8 => Keycode::F8,
            DeviceQueryKeycode::F9 => Keycode::F9,
            DeviceQueryKeycode::F10 => Keycode::F10,
            DeviceQueryKeycode::F11 => Keycode::F11,
            DeviceQueryKeycode::F12 => Keycode::F12,
            DeviceQueryKeycode::Escape => Keycode::Escape,
            DeviceQueryKeycode::Space => Keycode::Space,
            DeviceQueryKeycode::LControl => Keycode::LControl,
            DeviceQueryKeycode::RControl => Keycode::RControl,
            DeviceQueryKeycode::LShift => Keycode::LShift,
            DeviceQueryKeycode::RShift => Keycode::RShift,
            DeviceQueryKeycode::LAlt => Keycode::LAlt,
            DeviceQueryKeycode::RAlt => Keycode::RAlt,
            DeviceQueryKeycode::Meta => Keycode::Meta,
            DeviceQueryKeycode::Enter => Keycode::Enter,
            DeviceQueryKeycode::Up => Keycode::Up,
            DeviceQueryKeycode::Down => Keycode::Down,
            DeviceQueryKeycode::Left => Keycode::Left,
            DeviceQueryKeycode::Right => Keycode::Right,
            DeviceQueryKeycode::Backspace => Keycode::Backspace,
            DeviceQueryKeycode::CapsLock => Keycode::CapsLock,
            DeviceQueryKeycode::Tab => Keycode::Tab,
            DeviceQueryKeycode::Home => Keycode::Home,
            DeviceQueryKeycode::End => Keycode::End,
            DeviceQueryKeycode::PageUp => Keycode::PageUp,
            DeviceQueryKeycode::PageDown => Keycode::PageDown,
            DeviceQueryKeycode::Insert => Keycode::Insert,
            DeviceQueryKeycode::Delete => Keycode::Delete,
            DeviceQueryKeycode::Numpad0 => Keycode::Numpad0,
            DeviceQueryKeycode::Numpad1 => Keycode::Numpad1,
            DeviceQueryKeycode::Numpad2 => Keycode::Numpad2,
            DeviceQueryKeycode::Numpad3 => Keycode::Numpad3,
            DeviceQueryKeycode::Numpad4 => Keycode::Numpad4,
            DeviceQueryKeycode::Numpad5 => Keycode::Numpad5,
            DeviceQueryKeycode::Numpad6 => Keycode::Numpad6,
            DeviceQueryKeycode::Numpad7 => Keycode::Numpad7,
            DeviceQueryKeycode::Numpad8 => Keycode::Numpad8,
            DeviceQueryKeycode::Numpad9 => Keycode::Numpad9,
            DeviceQueryKeycode::NumpadSubtract => Keycode::NumpadSubtract,
            DeviceQueryKeycode::NumpadAdd => Keycode::NumpadAdd,
            DeviceQueryKeycode::NumpadDivide => Keycode::NumpadDivide,
            DeviceQueryKeycode::NumpadMultiply => Keycode::NumpadMultiply,
            DeviceQueryKeycode::Grave => Keycode::Grave,
            DeviceQueryKeycode::Minus => Keycode::Minus,
            DeviceQueryKeycode::Equal => Keycode::Equal,
            DeviceQueryKeycode::LeftBracket => Keycode::LeftBracket,
            DeviceQueryKeycode::RightBracket => Keycode::RightBracket,
            DeviceQueryKeycode::BackSlash => Keycode::BackSlash,
            DeviceQueryKeycode::Semicolon => Keycode::Semicolon,
            DeviceQueryKeycode::Apostrophe => Keycode::Apostrophe,
            DeviceQueryKeycode::Comma => Keycode::Comma,
            DeviceQueryKeycode::Dot => Keycode::Dot,
            DeviceQueryKeycode::Slash => Keycode::Slash,
        }
    }
}

impl From<Keycode> for DeviceQueryKeycode {
    fn from(value: Keycode) -> Self {
        match value {
            Keycode::Key0 => DeviceQueryKeycode::Key0,
            Keycode::Key1 => DeviceQueryKeycode::Key1,
            Keycode::Key2 => DeviceQueryKeycode::Key2,
            Keycode::Key3 => DeviceQueryKeycode::Key3,
            Keycode::Key4 => DeviceQueryKeycode::Key4,
            Keycode::Key5 => DeviceQueryKeycode::Key5,
            Keycode::Key6 => DeviceQueryKeycode::Key6,
            Keycode::Key7 => DeviceQueryKeycode::Key7,
            Keycode::Key8 => DeviceQueryKeycode::Key8,
            Keycode::Key9 => DeviceQueryKeycode::Key9,
            Keycode::A => DeviceQueryKeycode::A,
            Keycode::B => DeviceQueryKeycode::B,
            Keycode::C => DeviceQueryKeycode::C,
            Keycode::D => DeviceQueryKeycode::D,
            Keycode::E => DeviceQueryKeycode::E,
            Keycode::F => DeviceQueryKeycode::F,
            Keycode::G => DeviceQueryKeycode::G,
            Keycode::H => DeviceQueryKeycode::H,
            Keycode::I => DeviceQueryKeycode::I,
            Keycode::J => DeviceQueryKeycode::J,
            Keycode::K => DeviceQueryKeycode::K,
            Keycode::L => DeviceQueryKeycode::L,
            Keycode::M => DeviceQueryKeycode::M,
            Keycode::N => DeviceQueryKeycode::N,
            Keycode::O => DeviceQueryKeycode::O,
            Keycode::P => DeviceQueryKeycode::P,
            Keycode::Q => DeviceQueryKeycode::Q,
            Keycode::R => DeviceQueryKeycode::R,
            Keycode::S => DeviceQueryKeycode::S,
            Keycode::T => DeviceQueryKeycode::T,
            Keycode::U => DeviceQueryKeycode::U,
            Keycode::V => DeviceQueryKeycode::V,
            Keycode::W => DeviceQueryKeycode::W,
            Keycode::X => DeviceQueryKeycode::X,
            Keycode::Y => DeviceQueryKeycode::Y,
            Keycode::Z => DeviceQueryKeycode::Z,
            Keycode::F1 => DeviceQueryKeycode::F1,
            Keycode::F2 => DeviceQueryKeycode::F2,
            Keycode::F3 => DeviceQueryKeycode::F3,
            Keycode::F4 => DeviceQueryKeycode::F4,
            Keycode::F5 => DeviceQueryKeycode::F5,
            Keycode::F6 => DeviceQueryKeycode::F6,
            Keycode::F7 => DeviceQueryKeycode::F7,
            Keycode::F8 => DeviceQueryKeycode::F8,
            Keycode::F9 => DeviceQueryKeycode::F9,
            Keycode::F10 => DeviceQueryKeycode::F10,
            Keycode::F11 => DeviceQueryKeycode::F11,
            Keycode::F12 => DeviceQueryKeycode::F12,
            Keycode::Escape => DeviceQueryKeycode::Escape,
            Keycode::Space => DeviceQueryKeycode::Space,
            Keycode::LControl => DeviceQueryKeycode::LControl,
            Keycode::RControl => DeviceQueryKeycode::RControl,
            Keycode::LShift => DeviceQueryKeycode::LShift,
            Keycode::RShift => DeviceQueryKeycode::RShift,
            Keycode::LAlt => DeviceQueryKeycode::LAlt,
            Keycode::RAlt => DeviceQueryKeycode::RAlt,
            Keycode::Meta => DeviceQueryKeycode::Meta,
            Keycode::Enter => DeviceQueryKeycode::Enter,
            Keycode::Up => DeviceQueryKeycode::Up,
            Keycode::Down => DeviceQueryKeycode::Down,
            Keycode::Left => DeviceQueryKeycode::Left,
            Keycode::Right => DeviceQueryKeycode::Right,
            Keycode::Backspace => DeviceQueryKeycode::Backspace,
            Keycode::CapsLock => DeviceQueryKeycode::CapsLock,
            Keycode::Tab => DeviceQueryKeycode::Tab,
            Keycode::Home => DeviceQueryKeycode::Home,
            Keycode::End => DeviceQueryKeycode::End,
            Keycode::PageUp => DeviceQueryKeycode::PageUp,
            Keycode::PageDown => DeviceQueryKeycode::PageDown,
            Keycode::Insert => DeviceQueryKeycode::Insert,
            Keycode::Delete => DeviceQueryKeycode::Delete,
            Keycode::Numpad0 => DeviceQueryKeycode::Numpad0,
            Keycode::Numpad1 => DeviceQueryKeycode::Numpad1,
            Keycode::Numpad2 => DeviceQueryKeycode::Numpad2,
            Keycode::Numpad3 => DeviceQueryKeycode::Numpad3,
            Keycode::Numpad4 => DeviceQueryKeycode::Numpad4,
            Keycode::Numpad5 => DeviceQueryKeycode::Numpad5,
            Keycode::Numpad6 => DeviceQueryKeycode::Numpad6,
            Keycode::Numpad7 => DeviceQueryKeycode::Numpad7,
            Keycode::Numpad8 => DeviceQueryKeycode::Numpad8,
            Keycode::Numpad9 => DeviceQueryKeycode::Numpad9,
            Keycode::NumpadSubtract => DeviceQueryKeycode::NumpadSubtract,
            Keycode::NumpadAdd => DeviceQueryKeycode::NumpadAdd,
            Keycode::NumpadDivide => DeviceQueryKeycode::NumpadDivide,
            Keycode::NumpadMultiply => DeviceQueryKeycode::NumpadMultiply,
            Keycode::Grave => DeviceQueryKeycode::Grave,
            Keycode::Minus => DeviceQueryKeycode::Minus,
            Keycode::Equal => DeviceQueryKeycode::Equal,
            Keycode::LeftBracket => DeviceQueryKeycode::LeftBracket,
            Keycode::RightBracket => DeviceQueryKeycode::RightBracket,
            Keycode::BackSlash => DeviceQueryKeycode::BackSlash,
            Keycode::Semicolon => DeviceQueryKeycode::Semicolon,
            Keycode::Apostrophe => DeviceQueryKeycode::Apostrophe,
            Keycode::Comma => DeviceQueryKeycode::Comma,
            Keycode::Dot => DeviceQueryKeycode::Dot,
            Keycode::Slash => DeviceQueryKeycode::Slash,
        }
    }
}

impl KeycodeType for DeviceQueryKeycode {
    fn num_variants() -> usize {
        96
    }

    fn index(&self) -> usize {
        match &self {
            DeviceQueryKeycode::Key0 => 0,
            DeviceQueryKeycode::Key1 => 1,
            DeviceQueryKeycode::Key2 => 2,
            DeviceQueryKeycode::Key3 => 3,
            DeviceQueryKeycode::Key4 => 4,
            DeviceQueryKeycode::Key5 => 5,
            DeviceQueryKeycode::Key6 => 6,
            DeviceQueryKeycode::Key7 => 7,
            DeviceQueryKeycode::Key8 => 8,
            DeviceQueryKeycode::Key9 => 9,
            DeviceQueryKeycode::A => 10,
            DeviceQueryKeycode::B => 11,
            DeviceQueryKeycode::C => 12,
            DeviceQueryKeycode::D => 13,
            DeviceQueryKeycode::E => 14,
            DeviceQueryKeycode::F => 15,
            DeviceQueryKeycode::G => 16,
            DeviceQueryKeycode::H => 17,
            DeviceQueryKeycode::I => 18,
            DeviceQueryKeycode::J => 19,
            DeviceQueryKeycode::K => 20,
            DeviceQueryKeycode::L => 21,
            DeviceQueryKeycode::M => 22,
            DeviceQueryKeycode::N => 23,
            DeviceQueryKeycode::O => 24,
            DeviceQueryKeycode::P => 25,
            DeviceQueryKeycode::Q => 26,
            DeviceQueryKeycode::R => 27,
            DeviceQueryKeycode::S => 28,
            DeviceQueryKeycode::T => 29,
            DeviceQueryKeycode::U => 30,
            DeviceQueryKeycode::V => 31,
            DeviceQueryKeycode::W => 32,
            DeviceQueryKeycode::X => 33,
            DeviceQueryKeycode::Y => 34,
            DeviceQueryKeycode::Z => 35,
            DeviceQueryKeycode::F1 => 36,
            DeviceQueryKeycode::F2 => 37,
            DeviceQueryKeycode::F3 => 38,
            DeviceQueryKeycode::F4 => 39,
            DeviceQueryKeycode::F5 => 40,
            DeviceQueryKeycode::F6 => 41,
            DeviceQueryKeycode::F7 => 42,
            DeviceQueryKeycode::F8 => 43,
            DeviceQueryKeycode::F9 => 44,
            DeviceQueryKeycode::F10 => 45,
            DeviceQueryKeycode::F11 => 46,
            DeviceQueryKeycode::F12 => 47,
            DeviceQueryKeycode::Escape => 48,
            DeviceQueryKeycode::Space => 49,
            DeviceQueryKeycode::LControl => 50,
            DeviceQueryKeycode::RControl => 51,
            DeviceQueryKeycode::LShift => 52,
            DeviceQueryKeycode::RShift => 53,
            DeviceQueryKeycode::LAlt => 54,
            DeviceQueryKeycode::RAlt => 55,
            DeviceQueryKeycode::Meta => 56,
            DeviceQueryKeycode::Enter => 57,
            DeviceQueryKeycode::Up => 58,
            DeviceQueryKeycode::Down => 59,
            DeviceQueryKeycode::Left => 60,
            DeviceQueryKeycode::Right => 61,
            DeviceQueryKeycode::Backspace => 62,
            DeviceQueryKeycode::CapsLock => 63,
            DeviceQueryKeycode::Tab => 64,
            DeviceQueryKeycode::Home => 65,
            DeviceQueryKeycode::End => 66,
            DeviceQueryKeycode::PageUp => 67,
            DeviceQueryKeycode::PageDown => 68,
            DeviceQueryKeycode::Insert => 69,
            DeviceQueryKeycode::Delete => 70,
            DeviceQueryKeycode::Numpad0 => 71,
            DeviceQueryKeycode::Numpad1 => 72,
            DeviceQueryKeycode::Numpad2 => 73,
            DeviceQueryKeycode::Numpad3 => 74,
            DeviceQueryKeycode::Numpad4 => 75,
            DeviceQueryKeycode::Numpad5 => 76,
            DeviceQueryKeycode::Numpad6 => 77,
            DeviceQueryKeycode::Numpad7 => 78,
            DeviceQueryKeycode::Numpad8 => 79,
            DeviceQueryKeycode::Numpad9 => 80,
            DeviceQueryKeycode::NumpadSubtract => 81,
            DeviceQueryKeycode::NumpadAdd => 82,
            DeviceQueryKeycode::NumpadDivide => 83,
            DeviceQueryKeycode::NumpadMultiply => 84,
            DeviceQueryKeycode::Grave => 85,
            DeviceQueryKeycode::Minus => 86,
            DeviceQueryKeycode::Equal => 87,
            DeviceQueryKeycode::LeftBracket => 88,
            DeviceQueryKeycode::RightBracket => 89,
            DeviceQueryKeycode::BackSlash => 90,
            DeviceQueryKeycode::Semicolon => 91,
            DeviceQueryKeycode::Apostrophe => 92,
            DeviceQueryKeycode::Comma => 93,
            DeviceQueryKeycode::Dot => 94,
            DeviceQueryKeycode::Slash => 95,
        }
    }
}

pub type HotkeyManager = hotkey::HotkeyManager<DeviceQueryKeyboardState, DeviceQueryKeycode>;

impl HotkeyManager {
    pub fn new(key_bindings: &KeyBindings) -> Result<HotkeyManager, &'static str> {
        HotkeyManager::new_generic(key_bindings)
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        HotkeyManager::new(&KeyBindings::default()).expect("default keybindings were invalid")
    }
}
