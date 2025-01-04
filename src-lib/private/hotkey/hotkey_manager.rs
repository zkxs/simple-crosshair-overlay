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

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::private::platform::{KeyboardState, KeycodeType};

use super::Keycode;

/// the number of bits in this mask is the number of distinct keys that can be used across all keybinds
type Bitmask = u32;
type KeyBinding = Vec<Keycode>;

// serde defaults for new keybinds
fn default_cycle_monitor_keybind() -> KeyBinding {
    KeyBindings::default().cycle_monitor
}

fn default_toggle_color_picker_keybind() -> KeyBinding {
    KeyBindings::default().toggle_color_picker
}

/// format user can specify keybindings with
#[derive(Serialize, Deserialize)]
pub struct KeyBindings {
    up: KeyBinding,
    down: KeyBinding,
    left: KeyBinding,
    right: KeyBinding,
    #[serde(default = "default_cycle_monitor_keybind")]
    cycle_monitor: KeyBinding,
    scale_increase: KeyBinding,
    scale_decrease: KeyBinding,
    toggle_hidden: KeyBinding,
    toggle_adjust: KeyBinding,
    #[serde(default = "default_toggle_color_picker_keybind")]
    toggle_color_picker: KeyBinding,
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindings {
            up: vec![Keycode::Up],
            down: vec![Keycode::Down],
            left: vec![Keycode::Left],
            right: vec![Keycode::Right],
            cycle_monitor: vec![Keycode::LControl, Keycode::M],
            scale_increase: vec![Keycode::PageUp],
            scale_decrease: vec![Keycode::PageDown],
            toggle_hidden: vec![Keycode::LControl, Keycode::H],
            toggle_adjust: vec![Keycode::LControl, Keycode::J],
            toggle_color_picker: vec![Keycode::LControl, Keycode::K],
        }
    }
}

struct KeyBuffer<K>
where
    K: KeycodeType,
{
    lookup_table: Vec<Bitmask>,
    up_mask: Bitmask,
    down_mask: Bitmask,
    left_mask: Bitmask,
    right_mask: Bitmask,
    cycle_monitor_mask: Bitmask,
    scale_increase_mask: Bitmask,
    scale_decrease_mask: Bitmask,
    toggle_hidden_mask: Bitmask,
    toggle_adjust_mask: Bitmask,
    toggle_color_picker_mask: Bitmask,
    any_movement_mask: Bitmask,
    any_scale_mask: Bitmask,
    _keycode_type_marker: PhantomData<K>,
}

impl<K> KeyBuffer<K>
where
    K: KeycodeType,
{
    fn new(key_bindings: &KeyBindings) -> Result<KeyBuffer<K>, &'static str> {
        // build the lookup table and compute each hotkeys bitmask combination
        let mut bit = 1;
        let mut lookup_table = vec![0; K::num_variants()];
        let up_mask =
            Self::update_key_buffer_values(&key_bindings.up, &mut bit, &mut lookup_table)?;
        let down_mask =
            Self::update_key_buffer_values(&key_bindings.down, &mut bit, &mut lookup_table)?;
        let left_mask =
            Self::update_key_buffer_values(&key_bindings.left, &mut bit, &mut lookup_table)?;
        let right_mask =
            Self::update_key_buffer_values(&key_bindings.right, &mut bit, &mut lookup_table)?;
        let cycle_monitor_mask = Self::update_key_buffer_values(
            &key_bindings.cycle_monitor,
            &mut bit,
            &mut lookup_table,
        )?;
        let scale_increase_mask = Self::update_key_buffer_values(
            &key_bindings.scale_increase,
            &mut bit,
            &mut lookup_table,
        )?;
        let scale_decrease_mask = Self::update_key_buffer_values(
            &key_bindings.scale_decrease,
            &mut bit,
            &mut lookup_table,
        )?;
        let toggle_hidden_mask = Self::update_key_buffer_values(
            &key_bindings.toggle_hidden,
            &mut bit,
            &mut lookup_table,
        )?;
        let toggle_adjust_mask = Self::update_key_buffer_values(
            &key_bindings.toggle_adjust,
            &mut bit,
            &mut lookup_table,
        )?;
        let toggle_color_picker_mask = Self::update_key_buffer_values(
            &key_bindings.toggle_color_picker,
            &mut bit,
            &mut lookup_table,
        )?;
        let any_movement_mask = up_mask | down_mask | left_mask | right_mask;
        let any_scale_mask = scale_increase_mask | scale_decrease_mask;

        Ok(KeyBuffer {
            lookup_table,
            up_mask,
            down_mask,
            left_mask,
            right_mask,
            cycle_monitor_mask,
            scale_increase_mask,
            scale_decrease_mask,
            toggle_hidden_mask,
            toggle_adjust_mask,
            toggle_color_picker_mask,
            any_movement_mask,
            any_scale_mask,
            _keycode_type_marker: Default::default(),
        })
    }

    /// - `key_combination`: a set of keys to use for a specific hotkey action
    /// - `bit`: a bitmask with a single bit set which is used to represent a single key. For example,
    ///   Ctrl might end up as 0b1. This bit is shifted for each distinct key we use.
    /// - `lookup_table`: a lookup table where each item is a key. A value of zero indicates no hotkey
    ///   uses this key. A nonzero value indicates at least one hotkey uses this key.
    ///
    /// This function is called for each hotkey you want to register, and it returns bitmask
    /// representing which keys must be pressed for that hotkey. Each key used as part of the hotkey
    /// system is assigned a unique bit in this masking scheme. This means if a u32 is used as the
    /// bitmask type then only 32 distinct keys may be used across all hotkeys.
    fn update_key_buffer_values(
        key_combination: &[Keycode],
        bit: &mut Bitmask,
        lookup_table: &mut [Bitmask],
    ) -> Result<Bitmask, &'static str> {
        let mut mask: Bitmask = 0;
        for keycode in key_combination {
            let lookup_table_mask = &mut lookup_table[K::from(*keycode).index()];
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

    /// Get the bitmask that corresponds to this specific key. This returns a mask with a single bit
    /// set for keys used in any hotkey, and returns zero for keys not used in any hotkey.
    #[inline(always)]
    fn keycode_to_mask(&self, keycode: &K) -> Bitmask {
        self.lookup_table[keycode.index()]
    }

    /// Generate the bitmask that corresponds to the currently pressed key combination.
    fn update(&self, buf: &mut Bitmask, keys: &[K]) {
        *buf = 0;
        for keycode in keys {
            *buf |= self.keycode_to_mask(keycode);
        }
    }

    /// Check if the currently pressed keys contain the "up" key combination
    fn up(&self, buf: Bitmask) -> bool {
        buf & self.up_mask == self.up_mask
    }

    /// Check if the currently pressed keys contain the "down" key combination
    fn down(&self, buf: Bitmask) -> bool {
        buf & self.down_mask == self.down_mask
    }

    /// Check if the currently pressed keys contain the "left" key combination
    fn left(&self, buf: Bitmask) -> bool {
        buf & self.left_mask == self.left_mask
    }

    /// Check if the currently pressed keys contain the "right" key combination
    fn right(&self, buf: Bitmask) -> bool {
        buf & self.right_mask == self.right_mask
    }

    /// Check if the currently pressed keys contain the "cycle_monitor" key combination
    fn cycle_monitor(&self, buf: Bitmask) -> bool {
        buf & self.cycle_monitor_mask == self.cycle_monitor_mask
    }

    /// Check if the currently pressed keys contain the "scale_increase" key combination
    fn scale_increase(&self, buf: Bitmask) -> bool {
        buf & self.scale_increase_mask == self.scale_increase_mask
    }

    /// Check if the currently pressed keys contain the "scale_decrease" key combination
    fn scale_decrease(&self, buf: Bitmask) -> bool {
        buf & self.scale_decrease_mask == self.scale_decrease_mask
    }

    /// Check if the currently pressed keys contain the "toggle_hidden" key combination
    fn toggle_hidden(&self, buf: Bitmask) -> bool {
        buf & self.toggle_hidden_mask == self.toggle_hidden_mask
    }

    /// Check if the currently pressed keys contain the "toggle_adjust" key combination
    fn toggle_adjust(&self, buf: Bitmask) -> bool {
        buf & self.toggle_adjust_mask == self.toggle_adjust_mask
    }

    /// Check if the currently pressed keys contain the "toggle_color_picker" key combination
    fn toggle_color_picker(&self, buf: Bitmask) -> bool {
        buf & self.toggle_color_picker_mask == self.toggle_color_picker_mask
    }

    //TODO: this is not strictly correct: if a movement keybind uses multiple keys it breaks, as it will return `true` for partial binding presses
    /// Check if the currently pressed keys contain any movement keys
    fn any_movement(&self, buf: Bitmask) -> bool {
        buf & self.any_movement_mask != 0
    }

    //TODO: this is not strictly correct: if a scale keybind uses multiple keys it breaks, as it will return `true` for partial binding presses
    /// Check if the currently pressed keys contain any scaling keys
    fn any_scale(&self, buf: Bitmask) -> bool {
        buf & self.any_scale_mask != 0
    }
}

pub struct HotkeyManager<KS, K>
where
    KS: KeyboardState<K>,
    K: KeycodeType,
{
    previous_state: Bitmask,
    current_state: Bitmask,
    movement_key_held_frames: u32,
    scale_key_held_frames: u32,
    key_buffer: KeyBuffer<K>,
    keyboard_state: KS,
}

impl<KS, K> HotkeyManager<KS, K>
where
    KS: KeyboardState<K>,
    K: KeycodeType,
{
    pub(crate) fn new_generic(
        key_bindings: &KeyBindings,
    ) -> Result<HotkeyManager<KS, K>, &'static str> {
        Ok(HotkeyManager {
            previous_state: 0,
            current_state: 0,
            movement_key_held_frames: 0,
            scale_key_held_frames: 0,
            key_buffer: KeyBuffer::new(key_bindings)?,
            keyboard_state: KS::default(),
        })
    }

    pub fn poll_keys(&mut self) {
        self.keyboard_state.poll();
    }

    /// updates state with current key data
    pub fn process_keys(&mut self) {
        self.previous_state = self.current_state;

        // calculate state
        let key_buffer = &self.key_buffer;
        key_buffer.update(&mut self.current_state, self.keyboard_state.get_state());

        self.movement_key_held_frames = if key_buffer.any_movement(self.current_state) {
            self.movement_key_held_frames + 1
        } else {
            0
        };

        self.scale_key_held_frames = if key_buffer.any_scale(self.current_state) {
            self.scale_key_held_frames + 1
        } else {
            0
        };
    }

    /// check if "toggle_hidden" key combination was just pressed
    pub fn toggle_hidden(&self) -> bool {
        let key_buffer = &self.key_buffer;
        !key_buffer.toggle_hidden(self.previous_state)
            && key_buffer.toggle_hidden(self.current_state)
    }

    /// check if "toggle_adjust" key combination was just pressed
    pub fn toggle_adjust(&self) -> bool {
        let key_buffer = &self.key_buffer;
        !key_buffer.toggle_adjust(self.previous_state)
            && key_buffer.toggle_adjust(self.current_state)
    }

    /// check if "toggle_color_picker" key combination was just pressed
    pub fn toggle_color_picker(&self) -> bool {
        let key_buffer = &self.key_buffer;
        !key_buffer.toggle_color_picker(self.previous_state)
            && key_buffer.toggle_color_picker(self.current_state)
    }

    /// check if "cycle_monitor" key combination was just pressed
    pub fn cycle_monitor(&self) -> bool {
        let key_buffer = &self.key_buffer;
        !key_buffer.cycle_monitor(self.previous_state)
            && key_buffer.cycle_monitor(self.current_state)
    }

    /// calculate the move up speed based on how long movement keys have been held
    pub fn move_up(&self) -> u32 {
        if self.key_buffer.up(self.current_state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    /// calculate the move down speed based on how long movement keys have been held
    pub fn move_down(&self) -> u32 {
        if self.key_buffer.down(self.current_state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    /// calculate the move left speed based on how long movement keys have been held
    pub fn move_left(&self) -> u32 {
        if self.key_buffer.left(self.current_state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    /// calculate the move right speed based on how long movement keys have been held
    pub fn move_right(&self) -> u32 {
        if self.key_buffer.right(self.current_state) {
            move_ramp(self.movement_key_held_frames)
        } else {
            0
        }
    }

    /// calculate the scale increase speed based on how long scaling keys have been held
    pub fn scale_increase(&self) -> u32 {
        if self.key_buffer.scale_increase(self.current_state) {
            scale_ramp(self.scale_key_held_frames)
        } else {
            0
        }
    }

    /// calculate the scale decrease speed based on how long scaling keys have been held
    pub fn scale_decrease(&self) -> u32 {
        if self.key_buffer.scale_decrease(self.current_state) {
            scale_ramp(self.scale_key_held_frames)
        } else {
            0
        }
    }
}

// TODO: this should probably be fps-aware
fn move_ramp(frames: u32) -> u32 {
    if frames < 2 {
        1
    } else if frames < 10 {
        0
    } else if frames < 25 {
        1
    } else if frames < 35 {
        4
    } else if frames < 55 {
        16
    } else if frames < 75 {
        32
    } else {
        64
    }
}

// TODO: this should probably be fps-aware
fn scale_ramp(frames: u32) -> u32 {
    if frames < 2 {
        1
    } else if frames < 10 {
        0
    } else if frames < 25 {
        1
    } else if frames < 35 {
        4
    } else if frames < 55 {
        16
    } else if frames < 75 {
        32
    } else {
        64
    }
}
