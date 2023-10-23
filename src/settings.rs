// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Relating to the settings file loaded on app start and persisted on app close

use std::{fs, io};
use std::path::{Path, PathBuf};
use std::time::Duration;

use debug_print::debug_println;
use serde::{Deserialize, Serialize};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;

use crosshair_lib::hotkey::KeyBindings;
use crosshair_lib::util::image::{self, Image};
use crosshair_lib::util::numeric::fps_to_tick_interval;

use crate::{CONFIG_PATH, show_warning};

const DEFAULT_OFFSET_X: i32 = 0;
const DEFAULT_OFFSET_Y: i32 = 0;
const DEFAULT_SIZE: u32 = 16;
const DEFAULT_FPS: u32 = 60;
const DEFAULT_MONITOR_INDEX: usize = 0;
const DEFAULT_MONITOR: u32 = (DEFAULT_MONITOR_INDEX as u32) + 1;
const DEFAULT_COLOR: u32 = 0xB2FF0000; // 70% alpha red;

// needed for serde, as it can't read constants directly
const fn default_fps() -> u32 {
    DEFAULT_FPS
}

const fn default_monitor() -> u32 {
    DEFAULT_MONITOR
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
    /// 1-indexed monitor to render the overlay to
    #[serde(default = "default_monitor")]
    monitor: u32,
}

impl PersistedSettings {
    fn load(self) -> Settings {
        let color = image::premultiply_alpha(self.color);

        // make sure that if the user manually put an empty string in their config we don't explode
        let filtered_image_path = self.image_path.as_ref()
            .filter(|path| !path.as_os_str().is_empty());

        let image = if let Some(image_path) = filtered_image_path {
            match image::load_png(image_path.as_path()) {
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
        let monitor_index = usize::try_from(self.monitor.checked_sub(1).unwrap()).unwrap();
        let render_mode = RenderMode::from(&image);

        Settings {
            persisted: self,
            color,
            image,
            tick_interval,
            monitor_index,
            desired_window_position: PhysicalPosition::default(),
            desired_window_size: PhysicalSize::default(),
            render_mode,
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
            color: DEFAULT_COLOR,
            fps: DEFAULT_FPS,
            image_path: None,
            key_bindings: KeyBindings::default(),
            monitor: DEFAULT_MONITOR,
        }
    }
}

/// A wrapper around the persisted settings providing additional derived values
pub struct Settings {
    pub persisted: PersistedSettings,
    pub color: u32,
    image: Option<Box<Image>>,
    pub tick_interval: Duration,
    /// 0-indexed monitor to render the overlay to
    pub monitor_index: usize,
    pub desired_window_position: PhysicalPosition<i32>,
    pub desired_window_size: PhysicalSize<u32>,
    pub render_mode: RenderMode,
}

impl Settings {
    pub fn size(&self) -> PhysicalSize<u32> {
        match self.render_mode {
            RenderMode::Image => {
                let image = self.image.as_ref().unwrap();
                PhysicalSize::new(image.width, image.height)
            }
            RenderMode::Crosshair => {
                PhysicalSize::new(self.persisted.window_width, self.persisted.window_height)
            }
            RenderMode::ColorPicker => {
                PhysicalSize::new(image::COLOR_PICKER_SIZE as u32, image::COLOR_PICKER_SIZE as u32)
            }
        }
    }

    pub fn image(&self) -> Option<&Image> {
        self.image.as_ref().map(|b| b.as_ref())
    }

    /// Toggle color picker mode on or off. Returns `true` if color picker mode is now enabled, `false` otherwise.
    pub fn toggle_pick_color(&mut self) -> bool {
        let (render_mode, enabled) = if self.render_mode == RenderMode::ColorPicker {
            (RenderMode::from(&self.image), false)
        } else {
            (RenderMode::ColorPicker, true)
        };
        self.render_mode = render_mode;
        enabled
    }

    pub fn set_pick_color(&mut self, pick_color: bool) {
        self.render_mode = if pick_color {
            RenderMode::ColorPicker
        } else {
            RenderMode::from(&self.image)
        }
    }

    /// Returns `true` if color picker mode is now enabled, `false` otherwise.
    pub fn get_pick_color(&self) -> bool {
        self.render_mode == RenderMode::ColorPicker
    }

    /// Set the color of the generated crosshair. The provided `color` must not have premultiplied alpha (yet)
    pub fn set_color(&mut self, color: u32) {
        debug_println!("set color to {color:08X}");
        self.persisted.color = color;
        self.color = image::premultiply_alpha(color);
        self.image = None; // unload image
        self.persisted.image_path = None;
        self.render_mode = RenderMode::Crosshair;
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
        self.persisted.color = DEFAULT_COLOR;
        self.color = image::premultiply_alpha(DEFAULT_COLOR);
        self.persisted.image_path = None;
        if self.render_mode == RenderMode::Image {
            self.render_mode = RenderMode::Crosshair;
        }
        self.image = None;
    }

    /// load a new PNG at runtime
    pub fn load_png(&mut self, path: PathBuf) -> io::Result<()> {
        let image = image::load_png(path.as_path())?;
        self.persisted.image_path = Some(path);
        self.image = Some(image);
        self.render_mode = RenderMode::Image;
        Ok(())
    }

    pub fn load() -> io::Result<Settings> {
        fs::create_dir_all(CONFIG_PATH.as_path().parent().unwrap())?;
        Settings::load_from_path(CONFIG_PATH.as_path())
    }

    #[inline(always)]
    fn load_from_path<T>(path: T) -> io::Result<Settings> where T: AsRef<Path> {
        fs::read_to_string(path)
            .and_then(|string| toml::from_str::<PersistedSettings>(&string).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))
            .map(|settings| settings.load())
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to_path(CONFIG_PATH.as_path())
    }

    #[inline(always)]
    fn save_to_path<T>(&self, path: T) -> Result<(), String> where T: AsRef<Path> {
        let serialized_config = toml::to_string(&self.persisted).expect("failed to serialize settings");
        fs::write(path, serialized_config).map_err(|e| format!("{e:?}"))
    }

    pub fn set_window_position(&mut self, window: &Window) {
        let position = self.compute_window_coordinates(window);
        self.desired_window_position = position;
        window.set_outer_position(position);
    }

    fn reset_window_position(&self, window: &Window) {
        window.set_outer_position(self.desired_window_position);
    }

    pub fn validate_window_position(&self, window: &Window, position: PhysicalPosition<i32>) {
        if position != self.desired_window_position {
            debug_println!("resetting window position");
            self.reset_window_position(window);
        }
    }

    pub fn set_window_size(&self, window: &Window) {
        let _ = window.request_inner_size(self.size());
    }

    pub fn validate_window_size(&self, window: &Window, size: PhysicalSize<u32>) {
        if size != self.size() {
            debug_println!("resetting window size");
            self.set_window_size(window);
        }
    }

    /// Compute the correct coordinates of the top-left of the window in order to center the crosshair in the selected monitor
    fn compute_window_coordinates(&self, window: &Window) -> PhysicalPosition<i32> {
        // fall back to primary monitor if the desired monitor index is invalid
        let monitor = window.available_monitors().nth(self.monitor_index)
            .unwrap_or_else(|| window.primary_monitor().unwrap());

        // grab a bunch of coordinates/sizes and convert them to i32s, as we have some signed math to do
        let PhysicalPosition { x: monitor_x, y: monitor_y } = monitor.position();
        let PhysicalSize { width: monitor_width, height: monitor_height } = monitor.size();
        let monitor_width = i32::try_from(monitor_width).unwrap();
        let monitor_height = i32::try_from(monitor_height).unwrap();
        let PhysicalSize { width: window_width, height: window_height } = self.size();
        let window_width = i32::try_from(window_width).unwrap();
        let window_height = i32::try_from(window_height).unwrap();

        // calculate the coordinates of the center of the monitor, rounding down
        let (monitor_center_x, monitor_center_y) = image::rectangle_center(monitor_x, monitor_y, monitor_width, monitor_height);

        // adjust by half our window size, as we want the coordinates at which to place the top-left corner of the window
        let window_x = monitor_center_x - (window_width / 2) + self.persisted.window_dx;
        let window_y = monitor_center_y - (window_height / 2) + self.persisted.window_dy;

        debug_println!("placing window at {}, {}", window_x, window_y);
        PhysicalPosition::new(window_x, window_y)
    }
}

impl Default for Settings {
    fn default() -> Self {
        let savable = PersistedSettings::default();
        let color = image::premultiply_alpha(savable.color);
        Settings {
            persisted: savable,
            color,
            image: None,
            tick_interval: fps_to_tick_interval(DEFAULT_FPS),
            monitor_index: DEFAULT_MONITOR_INDEX,
            desired_window_position: PhysicalPosition::default(),
            desired_window_size: PhysicalSize::default(),
            render_mode: RenderMode::Crosshair,
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum RenderMode {
    Image,
    Crosshair,
    ColorPicker,
}

impl<T> From<&Option<T>> for RenderMode where T: AsRef<Image> {
    fn from(value: &Option<T>) -> Self {
        if value.is_some() {
            RenderMode::Image
        } else {
            RenderMode::Crosshair
        }
    }
}

#[cfg(test)]
mod test_config_load {
    use super::*;

    /// typical config
    #[test]
    fn test_load_settings() {
        Settings::load_from_path("tests/resources/test_config.toml").unwrap();
    }

    /// config with an image set
    #[test]
    fn test_load_settings_with_image() {
        Settings::load_from_path("tests/resources/test_config_image.toml").unwrap();
    }

    /// config with minimum possible values set
    #[test]
    fn test_load_settings_old() {
        Settings::load_from_path("tests/resources/test_config_old.toml").unwrap();
    }

    /// load a PNG into a config
    #[test]
    fn test_load_png() {
        let mut settings = Settings::load_from_path("tests/resources/test_config.toml").unwrap();
        settings.load_png("tests/resources/test.png".into()).unwrap();
    }

    /// save config to disk
    #[test]
    fn test_save_config() {
        let settings = Settings::load_from_path("tests/resources/test_config.toml").unwrap();

        let mut path = std::env::temp_dir();
        path.push("DELETEME_simple-crosshair-overlay-test-config.toml");

        settings.save_to_path(&path).expect("save failed");
        fs::remove_file(&path).expect("cleanup failed");
    }
}
