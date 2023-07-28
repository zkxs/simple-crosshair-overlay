// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

use std::{fs, io, mem};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use png::ColorType;
use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalSize;

use crate::{CONFIG_PATH, show_warning};
use crate::hotkey::KeyBindings;
use crate::util::numeric::DivCeil;

const DEFAULT_OFFSET_X: i32 = 0;
const DEFAULT_OFFSET_Y: i32 = 0;
const DEFAULT_SIZE: u32 = 4;
const DEFAULT_FPS: u32 = 60;

// needed for serde, as it can't read constants directly
const fn default_fps() -> u32 {
    DEFAULT_FPS
}

#[derive(Deserialize, Serialize)]
pub struct PersistedSettings {
    pub window_dx: i32,
    pub window_dy: i32,
    pub window_width: u32,
    pub window_height: u32,
    #[serde(with = "crate::custom_serializer::argb_color")]
    color: u32,
    #[serde(default = "default_fps")]
    fps: u32,
    image_path: Option<PathBuf>,
    #[serde(default)]
    key_bindings: KeyBindings,
}

impl PersistedSettings {
    fn load(self) -> Settings {
        let color = premultiply_alpha(self.color);

        // make sure that if the user manually put an empty string in their config we don't explode
        let filtered_image_path = self.image_path.as_ref()
            .filter(|path| !path.as_os_str().is_empty());

        let image = if let Some(image_path) = filtered_image_path {
            match load_png(image_path.as_path()) {
                Ok(image) => Some(image),
                Err(e) => {
                    show_warning(format!("Failed loading saved image_path \"{}\".\n\n{}", image_path.display(), e));
                    None
                }
            }
        } else {
            None
        };

        let tick_interval = fps_to_tick_interval(self.fps);

        Settings {
            persisted: self,
            color,
            image,
            tick_interval,
        }
    }
}

impl Default for PersistedSettings {
    fn default() -> Self {
        PersistedSettings {
            window_dx: DEFAULT_OFFSET_X,
            window_dy: DEFAULT_OFFSET_Y,
            window_width: DEFAULT_SIZE,
            window_height: DEFAULT_SIZE,
            color: 0xB2FF0000, // 70% alpha red
            fps: DEFAULT_FPS,
            image_path: None,
            key_bindings: KeyBindings::default(),
        }
    }
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u32>,
}

pub struct Settings {
    pub persisted: PersistedSettings,
    pub color: u32,
    pub image: Option<Image>,
    pub tick_interval: Duration,
}

impl Settings {
    pub fn size(&self) -> PhysicalSize<u32> {
        if let Some(image) = &self.image {
            PhysicalSize::new(image.width, image.height)
        } else {
            PhysicalSize::new(self.persisted.window_width, self.persisted.window_height)
        }
    }

    pub fn is_scalable(&self) -> bool {
        self.image.is_none()
    }

    /// only reset the settings the user can actually edit in-app. If they've manually edited "secret settings" in their config that should stick.
    pub fn reset(&mut self) {
        self.persisted.window_dx = DEFAULT_OFFSET_X;
        self.persisted.window_dy = DEFAULT_OFFSET_Y;
        self.persisted.window_width = DEFAULT_SIZE;
        self.persisted.window_height = DEFAULT_SIZE;
        self.persisted.image_path = None;
        self.image = None;
    }

    /// load a new PNG at runtime
    pub fn load_png(&mut self, path: PathBuf) -> io::Result<()> {
        let image = load_png(path.as_path())?;
        self.persisted.image_path = Some(path);
        self.image = Some(image);
        Ok(())
    }

    pub fn load() -> io::Result<Settings> {
        fs::create_dir_all(CONFIG_PATH.as_path().parent().unwrap())?;
        fs::read_to_string(CONFIG_PATH.as_path())
            .and_then(|string| toml::from_str::<PersistedSettings>(&string).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))
            .map(|settings| settings.load())
    }

    pub fn save(&self) -> Result<(), String> {
        let serialized_config = toml::to_string(&self.persisted).expect("failed to serialize settings");
        fs::write(CONFIG_PATH.as_path(), serialized_config).map_err(|e| format!("{e:?}"))
    }
}

impl Default for Settings {
    fn default() -> Self {
        let savable = PersistedSettings::default();
        let color = premultiply_alpha(savable.color);
        Settings {
            persisted: savable,
            color,
            image: None,
            tick_interval: fps_to_tick_interval(DEFAULT_FPS),
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

fn fps_to_tick_interval(fps: u32) -> Duration {
    let millis = 1000.div_ceil_placeholder(fps);
    Duration::from_millis(millis as u64)
}

fn load_png(path: &Path) -> io::Result<Image> {
    let file = File::open(path)?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info()?;

    // make a buffer of the correct size to hold the reader's data, but as u32's instead of u8's
    const RATIO: usize = mem::size_of::<u32>() / mem::size_of::<u8>();
    let mut buf: Vec<u32> = Vec::with_capacity(reader.output_buffer_size().div_ceil_placeholder(RATIO));
    #[allow(clippy::uninit_vec)]
    unsafe {
        // there is no requirement I send a zeroed buffer to the PNG decoding library.
        buf.set_len(buf.capacity());
    }

    // a little check to make sure div_ceil isn't fucked up. Which it's definitely not, because I eyeballed it really sternly.
    debug_assert!(buf.len() * RATIO >= reader.output_buffer_size(), "buffer was unexpectedly not large enough for image decode");

    // I'm just transmuting color data between u32 and [u8; 4] packing. No risk.
    let aligned_buf: &mut [u8] = unsafe {
        if let ([], aligned, []) = buf.align_to_mut() {
            aligned
        } else {
            panic!("couldn't align u32 buf to u8")
        }
    };

    let info = reader.next_frame(aligned_buf)?;

    if info.color_type != ColorType::Rgba {
        Err(io::Error::new(io::ErrorKind::InvalidInput, format!("PNG was in {:?} format. Only {:?} format is supported. Please re-save your PNG in the required format.", info.color_type, ColorType::Rgba)))?;
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
