// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Relating to the settings file loaded on app start and persisted on app close

use std::{fs, io};
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalSize;

use crate::{CONFIG_PATH, show_warning};
use crate::hotkey::KeyBindings;
use crate::util::image::{Image, load_png, premultiply_alpha};
use crate::util::numeric::fps_to_tick_interval;

const DEFAULT_OFFSET_X: i32 = 0;
const DEFAULT_OFFSET_Y: i32 = 0;
const DEFAULT_SIZE: u32 = 4;
const DEFAULT_FPS: u32 = 60;

// needed for serde, as it can't read constants directly
const fn default_fps() -> u32 {
    DEFAULT_FPS
}

/// The actual persisted settings struct
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
    pub key_bindings: KeyBindings,
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

/// A wrapper around the persisted settings providing additional derived values
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
