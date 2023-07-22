// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalSize;

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub window_dx: i32,
    pub window_dy: i32,
    pub window_width: u32,
    pub window_height: u32,
    #[serde(with = "crate::custom_serializer::argb_color")]
    pub color: u32,
}

impl Settings {
    pub fn size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.window_width, self.window_height)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            window_dx: 0,
            window_dy: 0,
            window_width: 6,
            window_height: 6,
            color: 0xB2FF0000, // 70% alpha red
        }
    }
}
