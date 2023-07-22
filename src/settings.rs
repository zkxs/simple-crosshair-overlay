// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

use std::fs::File;
use std::mem;
use std::path::{Path, PathBuf};

use png::ColorType;
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
    pub fn load(self) -> Result<LoadedSettings, String> {
        let color = premultiply_alpha(self.color);
        let image = if let Some(image_path) = &self.image_path {
            Some(load_png(image_path.as_path())?)
        } else {
            None
        };

        Ok(
            LoadedSettings {
                savable: self,
                color,
                image,
            }
        )
    }
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u32>,
}

pub struct LoadedSettings {
    pub savable: SavableSettings,
    pub color: u32,
    pub image: Option<Image>,
}

impl LoadedSettings {
    pub fn size(&self) -> PhysicalSize<u32> {
        if let Some(image) = &self.image {
            PhysicalSize::new(image.width, image.height)
        } else {
            PhysicalSize::new(self.savable.window_width, self.savable.window_height)
        }
    }

    pub fn is_scalable(&self) -> bool {
        self.image.is_none()
    }
}

impl Default for LoadedSettings {
    fn default() -> Self {
        let savable = SavableSettings::default();
        let color = premultiply_alpha(savable.color);
        LoadedSettings {
            savable,
            color,
            image: None,
        }
    }
}

// process each pixel in a RGBA png
#[inline(always)]
fn rgba_to_argb(rgba_color: u32) -> u32 {
    let [r, g, b, a] = rgba_color.to_le_bytes(); // BE RGBA == LE ABGR
    let argb_color = u32::from_le_bytes([b, g, r, a]); // BE ARGB == LE BGRA
    premultiply_alpha(argb_color)
}

#[inline(always)]
#[cfg(target_os = "windows")]
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

// no-op on non-windows as this is not needed
#[inline(always)]
#[cfg(not(target_os = "windows"))]
fn premultiply_alpha(argb_color: u32) -> u32 {
    argb_color
}

fn load_png(path: &Path) -> Result<Image, String> {
    let file = File::open(path).map_err(|e| format!("error opening image \"{}\": {}", path.display(), e))?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().map_err(|e| format!("error reading PNG info \"{}\": {}", path.display(), e))?;

    const RATIO: usize = mem::size_of::<u32>() / mem::size_of::<u8>();
    let mut buf: Vec<u32> = vec![0; div_ceil(reader.output_buffer_size(), RATIO)];
    debug_assert!(buf.len() * RATIO >= reader.output_buffer_size(), "buffer was unexpectedly not large enough for image decode");

    let aligned_buf: &mut [u8] = unsafe {
        if let ([], aligned, []) = buf.align_to_mut() {
            aligned
        } else {
            panic!("couldn't align u32 buf to u8")
        }
    };

    let info = reader.next_frame(aligned_buf).map_err(|e| format!("error reading PNG frame \"{}\": {}", path.display(), e))?;

    if info.color_type != ColorType::Rgba {
        Err("Image was not in RGBA color")?;
    }

    // post-process buf
    buf.iter_mut()
        .for_each(|pixel| *pixel = rgba_to_argb(pixel.to_owned()));

    let image = Image {
        width: info.width,
        height: info.height,
        data: buf,
    };

    Ok(image)
}

const fn div_ceil(lhs: usize, rhs: usize) -> usize {
    let d = lhs / rhs;
    let r = lhs % rhs;
    if r > 0 && rhs > 0 {
        d + 1
    } else {
        d
    }
}
