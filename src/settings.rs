// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalSize;

#[derive(Deserialize, Serialize)]
pub struct SavableSettings {
    pub window_dx: i32,
    pub window_dy: i32,
    pub window_width: u32,
    pub window_height: u32,
    #[serde(with = "crate::custom_serializer::argb_color")]
    pub color: u32,
    pub image_path: Option<PathBuf>,
}

impl Default for SavableSettings {
    fn default() -> Self {
        SavableSettings {
            window_dx: 0,
            window_dy: 0,
            window_width: 4,
            window_height: 4,
            color: 0xB2FF0000, // 70% alpha red
            image_path: None,
        }
    }
}

impl SavableSettings {
    #[cfg(target_os = "windows")]
    fn preprocess_color(&self) -> u32 {
        // premultiply alpha on Windows. No idea if other platforms need this done.
        premultiply_alpha(self.color)
    }

    #[cfg(not(target_os = "windows"))]
    fn preprocess_color(&self) -> u32 {
        self.color
    }

    pub fn load(self) -> Result<LoadedSettings, String> {
        let color = self.preprocess_color();

        Ok(
            LoadedSettings {
                savable: self,
                color,
                image: None, //TODO: handle image
            }
        )
    }
}

pub struct Image {
    //TODO
}

pub struct LoadedSettings {
    pub savable: SavableSettings,
    pub color: u32,
    pub image: Option<Image>,
}

impl LoadedSettings {
    pub fn size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.savable.window_width, self.savable.window_height)
    }
}

impl Default for LoadedSettings {
    fn default() -> Self {
        let savable = SavableSettings::default();
        let color = savable.preprocess_color();
        LoadedSettings {
            savable,
            color,
            image: None,
        }
    }
}

fn premultiply_alpha(argb_color: u32) -> u32 {
    let [b, g, r, a] = argb_color.to_le_bytes();

    const MAX_COLOR: u16 = 255;

    let b = b as u16;
    let g = g as u16;
    let r = r as u16;
    let alpha = a as u16; // we're reusing `a` later, so give alpha a special name

    let b = (b * alpha / MAX_COLOR) as u8;
    let g = (g * alpha / MAX_COLOR) as u8;
    let r = (r * alpha / MAX_COLOR) as u8;

    u32::from_le_bytes([b, g, r, a])
}
