// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Hotkey input system.
//!
//! The idea here is to do as much work as possible up front once, thereby minimizing
//! the hot part of it: polling the keyboard state and extracting what we care about.
//!
//! We care about if certain key combinations are pressed. To make this really fast, I make
//! heavy use of bitmasks.

use device_query::Keycode as DeviceQueryKeycode;
use serde::{Deserialize, Serialize};

use super::Keycode;
use super::keycode_to_table_index;

const KEYCODE_LENGTH: usize = 96;

type Bitmask = u32; // the number of bits in this mask is the number of distinct keys that can be used across all keybinds
type KeyBinding = Vec<Keycode>;

/// format user can specify keybindings with
#[derive(Serialize, Deserialize)]
pub struct KeyBindings {
    up: KeyBinding,
    down: KeyBinding,
    left: KeyBinding,
    right: KeyBinding,
    scale_increase: KeyBinding,
    scale_decrease: KeyBinding,
    toggle_hidden: KeyBinding,
    toggle_adjust: KeyBinding,
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindings {
            up: vec![Keycode::Up],
            down: vec![Keycode::Down],
            left: vec![Keycode::Left],
            right: vec![Keycode::Right],
            scale_increase: vec![Keycode::PageUp],
            scale_decrease: vec![Keycode::PageDown],
            toggle_hidden: vec![Keycode::LControl, Keycode::H],
            toggle_adjust: vec![Keycode::LControl, Keycode::J],
        }
    }
}

struct KeyBuffer {
    lookup_table: [Bitmask; KEYCODE_LENGTH],
    up_mask: Bitmask,
    down_mask: Bitmask,
    left_mask: Bitmask,
    right_mask: Bitmask,
    scale_increase_mask: Bitmask,
    scale_decrease_mask: Bitmask,
    toggle_hidden_mask: Bitmask,
    toggle_adjust_mask: Bitmask,
    any_movement_mask: Bitmask,
    any_scale_mask: Bitmask,
}

impl KeyBuffer {
    fn new(key_bindings: &KeyBindings) -> Result<KeyBuffer, &'static str> {
        let mut bit = 1;

        let mut lookup_table = [0; KEYCODE_LENGTH];
        let up_mask = update_key_buffer_values(&key_bindings.up, &mut bit, &mut lookup_table)?;
        let down_mask = update_key_buffer_values(&key_bindings.down, &mut bit, &mut lookup_table)?;
        let left_mask = update_key_buffer_values(&key_bindings.left, &mut bit, &mut lookup_table)?;
        let right_mask = update_key_buffer_values(&key_bindings.right, &mut bit, &mut lookup_table)?;
        let scale_increase_mask = update_key_buffer_values(&key_bindings.scale_increase, &mut bit, &mut lookup_table)?;
        let scale_decrease_mask = update_key_buffer_values(&key_bindings.scale_decrease, &mut bit, &mut lookup_table)?;
        let toggle_hidden_mask = update_key_buffer_values(&key_bindings.toggle_hidden, &mut bit, &mut lookup_table)?;
        let toggle_adjust_mask = update_key_buffer_values(&key_bindings.toggle_adjust, &mut bit, &mut lookup_table)?;
        let any_movement_mask = up_mask | down_mask | left_mask | right_mask;
        let any_scale_mask = scale_increase_mask | scale_decrease_mask;

        Ok(
            KeyBuffer {
                lookup_table,
                up_mask,
                down_mask,
                left_mask,
                right_mask,
                scale_increase_mask,
                scale_decrease_mask,
                toggle_hidden_mask,
                toggle_adjust_mask,
                any_movement_mask,
                any_scale_mask,
            }
        )
    }

    fn keycode_to_mask(&self, keycode: &DeviceQueryKeycode) -> Bitmask {
        self.lookup_table[keycode_to_table_index(keycode)]
    }

    fn update(&self, buf: &mut Bitmask, keys: &[DeviceQueryKeycode]) {
        *buf = 0;
        for keycode in keys {
            *buf |= self.keycode_to_mask(keycode);
        }
    }

    fn up(&self, buf: Bitmask) -> bool {
        buf & self.up_mask == self.up_mask
    }

    fn down(&self, buf: Bitmask) -> bool {
        buf & self.down_mask == self.down_mask
    }

    fn left(&self, buf: Bitmask) -> bool {
        buf & self.left_mask == self.left_mask
    }

    fn right(&self, buf: Bitmask) -> bool {
        buf & self.right_mask == self.right_mask
    }

    fn scale_increase(&self, buf: Bitmask) -> bool {
        buf & self.scale_increase_mask == self.scale_increase_mask
    }

    fn scale_decrease(&self, buf: Bitmask) -> bool {
        buf & self.scale_decrease_mask == self.scale_decrease_mask
    }

    fn toggle_hidden(&self, buf: Bitmask) -> bool {
        buf & self.toggle_hidden_mask == self.toggle_hidden_mask
    }

    fn toggle_adjust(&self, buf: Bitmask) -> bool {
        buf & self.toggle_adjust_mask == self.toggle_adjust_mask
    }

    //TODO: this is not strictly correct: if a movement keybind uses multiple keys it breaks
    fn any_movement(&self, buf: Bitmask) -> bool {
        buf & self.any_movement_mask != 0
    }

    //TODO: this is not strictly correct: if a scale keybind uses multiple keys it breaks
    fn any_scale(&self, buf: Bitmask) -> bool {
        buf & self.any_scale_mask != 0
    }
}

pub struct HotkeyManager {
    previous_state: Bitmask,
    state: Bitmask,
    movement_key_held_frames: u32,
    scale_key_held_frames: u32,
    key_buffer: Box<KeyBuffer>,
}

impl HotkeyManager {
    pub fn new(key_bindings: &KeyBindings) -> Result<HotkeyManager, &'static str> {
        Ok(
            HotkeyManager {
                previous_state: 0,
                state: 0,
                movement_key_held_frames: 0,
                scale_key_held_frames: 0,
                key_buffer: Box::new(KeyBuffer::new(key_bindings)?),
            }
        )
    }

    /// updates state with current key data
    pub fn process_keys(&mut self, keys: Vec<DeviceQueryKeycode>) {
        self.previous_state = self.state;

        // calculate state
        let key_buffer: &KeyBuffer = &self.key_buffer;
        key_buffer.update(&mut self.state, &keys);

        self.movement_key_held_frames = if key_buffer.any_movement(self.state) {
            self.movement_key_held_frames + 1
        } else {
            0
        };

        self.scale_key_held_frames = if key_buffer.any_scale(self.state) {
            self.scale_key_held_frames + 1
        } else {
            0
        };
    }

    pub fn toggle_hidden(&self) -> bool {
        let key_buffer: &KeyBuffer = &self.key_buffer;
        !key_buffer.toggle_hidden(self.previous_state) && key_buffer.toggle_hidden(self.state)
    }

    pub fn toggle_adjust(&self) -> bool {
        let key_buffer: &KeyBuffer = &self.key_buffer;
        !key_buffer.toggle_adjust(self.previous_state) && key_buffer.toggle_adjust(self.state)
    }

    pub fn move_up(&self) -> u32 {
        if self.key_buffer.up(self.state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    pub fn move_down(&self) -> u32 {
        if self.key_buffer.down(self.state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    pub fn move_left(&self) -> u32 {
        if self.key_buffer.left(self.state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    pub fn move_right(&self) -> u32 {
        if self.key_buffer.right(self.state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    pub fn scale_increase(&self) -> u32 {
        if self.key_buffer.scale_increase(self.state) {
            scale_ramp(self.scale_key_held_frames)
        } else {
            0
        }
    }

    pub fn scale_decrease(&self) -> u32 {
        if self.key_buffer.scale_decrease(self.state) {
            scale_ramp(self.scale_key_held_frames)
        } else {
            0
        }
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        HotkeyManager::new(&KeyBindings::default()).expect("default keybindings were invalid")
    }
}

fn update_key_buffer_values(key_combination: &[Keycode], bit: &mut Bitmask, lookup_table: &mut [Bitmask; KEYCODE_LENGTH]) -> Result<Bitmask, &'static str> {
    let mut mask: Bitmask = 0;
    for keycode in key_combination {
        let lookup_table_mask = &mut lookup_table[keycode_to_table_index(&keycode.into())];
        if *lookup_table_mask == 0 {
            // if the previous shift overflowed the mask will be zero
            if *bit == 0 {
                return Err("Only 32 distinct keys may be used for hotkeys at this time. Congratulations if you're seeing this, as I didn't think anyone would be crazy enough to use that many keys.");
            }

            // generate a new mask and add to the table
            *lookup_table_mask = *bit;
            *bit <<= 1;
        }
        mask |= *lookup_table_mask;
    }
    Ok(mask)
}

fn move_ramp(frames: u32) -> u32 {
    if frames < 10 {
        // 0-9
        1
    } else if frames < 20 {
        // 10-19
        4
    } else if frames < 40 {
        // 20-39
        16
    } else if frames < 60 {
        // 40-59
        32
    } else {
        // 60+
        64
    }
}

fn scale_ramp(frames: u32) -> u32 {
    if frames < 10 {
        // 0-9
        1
    } else if frames < 20 {
        // 10-19
        4
    } else if frames < 40 {
        // 20-39
        16
    } else if frames < 60 {
        // 40-59
        32
    } else {
        // 60+
        64
    }
}
