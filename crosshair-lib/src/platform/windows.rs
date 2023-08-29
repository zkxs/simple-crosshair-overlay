// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Windows-specific implementations.
//! This is only in the module tree on Windows targets.

use std::io::Error as OsError;

use winapi::ctypes::c_int;
use winapi::shared::windef::HWND;
use winapi::um::winuser;

use crate::hotkey;
use crate::hotkey::{KeyBindings, Keycode};
use crate::platform::{KeyboardState, KeycodeType};

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
        match winuser::GetForegroundWindow() {
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
        winuser::SetForegroundWindow(window_handle.hwnd()) != 0
    }
}

/// wrapper around https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getkeyboardstate
///
/// this reads keyboard state without allocating
fn get_keyboard_state(buffer: &mut [u8; 256], keycodes: &mut Vec<c_int>) -> Result<(), OsError> {
    keycodes.clear();
    let result = unsafe { winuser::GetKeyboardState(buffer.as_mut_ptr()) };
    if result == 0 {
        // If the function fails, the return value is zero
        Err(OsError::last_os_error())
    } else {
        // If the function succeeds, the return value is nonzero.
        for (vk, state) in buffer.iter().enumerate() {
            if state & 0b1000_0000 != 0 {
                // If the high-order bit is 1, the key is down; otherwise, it is up.
                keycodes.push(vk as c_int);
            }
        }
        Ok(())
    }
}

pub struct WinApiKeyboardState {
    p_byte: [u8; 256],
    vk_state: Vec<c_int>,
}

impl Default for WinApiKeyboardState {
    fn default() -> Self {
        Self {
            p_byte: [0; 256],
            vk_state: Vec::new(),
        }
    }
}

impl KeyboardState<c_int> for WinApiKeyboardState {
    fn poll(&mut self) {
        get_keyboard_state(&mut self.p_byte, &mut self.vk_state).unwrap();
    }

    fn get_state(&self) -> &[c_int] {
        &self.vk_state
    }
}

impl TryFrom<c_int> for Keycode {
    type Error = ();

    fn try_from(virtual_key: c_int) -> Result<Self, Self::Error> {
        match virtual_key {
            winuser::VK_BACK => Ok(Keycode::Backspace),
            winuser::VK_TAB => Ok(Keycode::Tab),
            winuser::VK_RETURN => Ok(Keycode::Enter),
            winuser::VK_LSHIFT => Ok(Keycode::LShift),
            winuser::VK_RSHIFT => Ok(Keycode::RShift),
            winuser::VK_LCONTROL => Ok(Keycode::LControl),
            winuser::VK_RCONTROL => Ok(Keycode::RControl),
            winuser::VK_LMENU => Ok(Keycode::LAlt),
            winuser::VK_RMENU => Ok(Keycode::RAlt),
            winuser::VK_CAPITAL => Ok(Keycode::CapsLock),
            winuser::VK_ESCAPE => Ok(Keycode::Escape),
            winuser::VK_SPACE => Ok(Keycode::Space),
            winuser::VK_PRIOR => Ok(Keycode::PageUp),
            winuser::VK_NEXT => Ok(Keycode::PageDown),
            winuser::VK_END => Ok(Keycode::End),
            winuser::VK_HOME => Ok(Keycode::Home),
            winuser::VK_LEFT => Ok(Keycode::Left),
            winuser::VK_UP => Ok(Keycode::Up),
            winuser::VK_RIGHT => Ok(Keycode::Right),
            winuser::VK_DOWN => Ok(Keycode::Down),
            winuser::VK_INSERT => Ok(Keycode::Insert),
            winuser::VK_DELETE => Ok(Keycode::Delete),
            0x30 => Ok(Keycode::Key0),
            0x31 => Ok(Keycode::Key1),
            0x32 => Ok(Keycode::Key2),
            0x33 => Ok(Keycode::Key3),
            0x34 => Ok(Keycode::Key4),
            0x35 => Ok(Keycode::Key5),
            0x36 => Ok(Keycode::Key6),
            0x37 => Ok(Keycode::Key7),
            0x38 => Ok(Keycode::Key8),
            0x39 => Ok(Keycode::Key9),
            0x41 => Ok(Keycode::A),
            0x42 => Ok(Keycode::B),
            0x43 => Ok(Keycode::C),
            0x44 => Ok(Keycode::D),
            0x45 => Ok(Keycode::E),
            0x46 => Ok(Keycode::F),
            0x47 => Ok(Keycode::G),
            0x48 => Ok(Keycode::H),
            0x49 => Ok(Keycode::I),
            0x4A => Ok(Keycode::J),
            0x4B => Ok(Keycode::K),
            0x4C => Ok(Keycode::L),
            0x4D => Ok(Keycode::M),
            0x4E => Ok(Keycode::N),
            0x4F => Ok(Keycode::O),
            0x50 => Ok(Keycode::P),
            0x51 => Ok(Keycode::Q),
            0x52 => Ok(Keycode::R),
            0x53 => Ok(Keycode::S),
            0x54 => Ok(Keycode::T),
            0x55 => Ok(Keycode::U),
            0x56 => Ok(Keycode::V),
            0x57 => Ok(Keycode::W),
            0x58 => Ok(Keycode::X),
            0x59 => Ok(Keycode::Y),
            0x5A => Ok(Keycode::Z),
            winuser::VK_LWIN => Ok(Keycode::Meta), // loss of left/right information
            winuser::VK_RWIN => Ok(Keycode::Meta), // loss of left/right information
            winuser::VK_NUMPAD0 => Ok(Keycode::Numpad0),
            winuser::VK_NUMPAD1 => Ok(Keycode::Numpad1),
            winuser::VK_NUMPAD2 => Ok(Keycode::Numpad2),
            winuser::VK_NUMPAD3 => Ok(Keycode::Numpad3),
            winuser::VK_NUMPAD4 => Ok(Keycode::Numpad4),
            winuser::VK_NUMPAD5 => Ok(Keycode::Numpad5),
            winuser::VK_NUMPAD6 => Ok(Keycode::Numpad6),
            winuser::VK_NUMPAD7 => Ok(Keycode::Numpad7),
            winuser::VK_NUMPAD8 => Ok(Keycode::Numpad8),
            winuser::VK_NUMPAD9 => Ok(Keycode::Numpad9),
            winuser::VK_MULTIPLY => Ok(Keycode::NumpadMultiply),
            winuser::VK_ADD => Ok(Keycode::NumpadAdd),
            winuser::VK_SUBTRACT => Ok(Keycode::NumpadSubtract),
            winuser::VK_DIVIDE => Ok(Keycode::NumpadDivide),
            winuser::VK_F1 => Ok(Keycode::F1),
            winuser::VK_F2 => Ok(Keycode::F2),
            winuser::VK_F3 => Ok(Keycode::F3),
            winuser::VK_F4 => Ok(Keycode::F4),
            winuser::VK_F5 => Ok(Keycode::F5),
            winuser::VK_F6 => Ok(Keycode::F6),
            winuser::VK_F7 => Ok(Keycode::F7),
            winuser::VK_F8 => Ok(Keycode::F8),
            winuser::VK_F9 => Ok(Keycode::F9),
            winuser::VK_F10 => Ok(Keycode::F10),
            winuser::VK_F11 => Ok(Keycode::F11),
            winuser::VK_F12 => Ok(Keycode::F12),
            winuser::VK_OEM_3 => Ok(Keycode::Grave),
            winuser::VK_OEM_MINUS => Ok(Keycode::Minus),
            winuser::VK_OEM_PLUS => Ok(Keycode::Equal),
            winuser::VK_OEM_4 => Ok(Keycode::LeftBracket),
            winuser::VK_OEM_6 => Ok(Keycode::RightBracket),
            winuser::VK_OEM_5 => Ok(Keycode::BackSlash),
            winuser::VK_OEM_1 => Ok(Keycode::Semicolon),
            winuser::VK_OEM_7 => Ok(Keycode::Apostrophe),
            winuser::VK_OEM_COMMA => Ok(Keycode::Comma),
            winuser::VK_OEM_PERIOD => Ok(Keycode::Dot),
            winuser::VK_OEM_2 => Ok(Keycode::Slash),
            _ => Err(()),
        }
    }
}

impl From<Keycode> for c_int {
    fn from(keycode: Keycode) -> Self {
        match keycode {
            Keycode::Key0 => 0x30,
            Keycode::Key1 => 0x31,
            Keycode::Key2 => 0x32,
            Keycode::Key3 => 0x33,
            Keycode::Key4 => 0x34,
            Keycode::Key5 => 0x35,
            Keycode::Key6 => 0x36,
            Keycode::Key7 => 0x37,
            Keycode::Key8 => 0x38,
            Keycode::Key9 => 0x39,
            Keycode::A => 0x41,
            Keycode::B => 0x42,
            Keycode::C => 0x43,
            Keycode::D => 0x44,
            Keycode::E => 0x45,
            Keycode::F => 0x46,
            Keycode::G => 0x47,
            Keycode::H => 0x48,
            Keycode::I => 0x49,
            Keycode::J => 0x4A,
            Keycode::K => 0x4B,
            Keycode::L => 0x4C,
            Keycode::M => 0x4D,
            Keycode::N => 0x4E,
            Keycode::O => 0x4F,
            Keycode::P => 0x50,
            Keycode::Q => 0x51,
            Keycode::R => 0x52,
            Keycode::S => 0x53,
            Keycode::T => 0x54,
            Keycode::U => 0x55,
            Keycode::V => 0x56,
            Keycode::W => 0x57,
            Keycode::X => 0x58,
            Keycode::Y => 0x59,
            Keycode::Z => 0x5A,
            Keycode::F1 => winuser::VK_F1,
            Keycode::F2 => winuser::VK_F2,
            Keycode::F3 => winuser::VK_F3,
            Keycode::F4 => winuser::VK_F4,
            Keycode::F5 => winuser::VK_F5,
            Keycode::F6 => winuser::VK_F6,
            Keycode::F7 => winuser::VK_F7,
            Keycode::F8 => winuser::VK_F8,
            Keycode::F9 => winuser::VK_F9,
            Keycode::F10 => winuser::VK_F10,
            Keycode::F11 => winuser::VK_F11,
            Keycode::F12 => winuser::VK_F12,
            Keycode::Escape => winuser::VK_ESCAPE,
            Keycode::Space => winuser::VK_SPACE,
            Keycode::LControl => winuser::VK_LCONTROL,
            Keycode::RControl => winuser::VK_RCONTROL,
            Keycode::LShift => winuser::VK_LSHIFT,
            Keycode::RShift => winuser::VK_RSHIFT,
            Keycode::LAlt => winuser::VK_LMENU,
            Keycode::RAlt => winuser::VK_RMENU,
            Keycode::Meta => winuser::VK_LWIN, // assume left meta key
            Keycode::Enter => winuser::VK_RETURN,
            Keycode::Up => winuser::VK_UP,
            Keycode::Down => winuser::VK_DOWN,
            Keycode::Left => winuser::VK_LEFT,
            Keycode::Right => winuser::VK_RIGHT,
            Keycode::Backspace => winuser::VK_BACK,
            Keycode::CapsLock => winuser::VK_CAPITAL,
            Keycode::Tab => winuser::VK_TAB,
            Keycode::Home => winuser::VK_HOME,
            Keycode::End => winuser::VK_END,
            Keycode::PageUp => winuser::VK_PRIOR,
            Keycode::PageDown => winuser::VK_NEXT,
            Keycode::Insert => winuser::VK_INSERT,
            Keycode::Delete => winuser::VK_DELETE,
            Keycode::Numpad0 => winuser::VK_NUMPAD0,
            Keycode::Numpad1 => winuser::VK_NUMPAD1,
            Keycode::Numpad2 => winuser::VK_NUMPAD2,
            Keycode::Numpad3 => winuser::VK_NUMPAD3,
            Keycode::Numpad4 => winuser::VK_NUMPAD4,
            Keycode::Numpad5 => winuser::VK_NUMPAD5,
            Keycode::Numpad6 => winuser::VK_NUMPAD6,
            Keycode::Numpad7 => winuser::VK_NUMPAD7,
            Keycode::Numpad8 => winuser::VK_NUMPAD8,
            Keycode::Numpad9 => winuser::VK_NUMPAD9,
            Keycode::NumpadSubtract => winuser::VK_SUBTRACT,
            Keycode::NumpadAdd => winuser::VK_ADD,
            Keycode::NumpadDivide => winuser::VK_DIVIDE,
            Keycode::NumpadMultiply => winuser::VK_MULTIPLY,
            Keycode::Grave => winuser::VK_OEM_3,
            Keycode::Minus => winuser::VK_OEM_MINUS,
            Keycode::Equal => winuser::VK_OEM_PLUS,
            Keycode::LeftBracket => winuser::VK_OEM_4,
            Keycode::RightBracket => winuser::VK_OEM_6,
            Keycode::BackSlash => winuser::VK_OEM_5,
            Keycode::Semicolon => winuser::VK_OEM_1,
            Keycode::Apostrophe => winuser::VK_OEM_7,
            Keycode::Comma => winuser::VK_OEM_COMMA,
            Keycode::Dot => winuser::VK_OEM_PERIOD,
            Keycode::Slash => winuser::VK_OEM_2,
        }
    }
}

impl KeycodeType for c_int {
    fn num_variants() -> usize {
        256
    }

    fn index(&self) -> usize {
        debug_assert!((*self as usize) < Self::num_variants());
        *self as usize
    }
}

pub type HotkeyManager = hotkey::HotkeyManager<WinApiKeyboardState, c_int>;

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

#[cfg(test)]
mod test_windows {
    use super::*;

    #[test]
    fn test_get_keyboard_state() {
        let mut buffer = [0; 256];
        let mut vec = Vec::new();
        get_keyboard_state(&mut buffer, &mut vec).unwrap();
    }
}
