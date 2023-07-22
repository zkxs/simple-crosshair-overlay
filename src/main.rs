// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

#![windows_subsystem = "windows"] // necessary to remove the console window on Windows

use std::fs;
use std::num::NonZeroU32;
use std::path::PathBuf;

use lazy_static::lazy_static;
use softbuffer::{Context, Surface};
use tray_icon::{menu::Menu, TrayIconBuilder};
use tray_icon::icon::Icon;
use tray_icon::menu::{CheckMenuItem, MenuEvent, MenuItem};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Event, VirtualKeyCode};
use winit::event::DeviceEvent::Key;
use winit::event_loop::{DeviceEventFilter, EventLoop};
use winit::platform::windows::WindowBuilderExtWindows;
use winit::window::{Window, WindowBuilder, WindowLevel};

use crate::settings::{LoadedSettings, SavableSettings};

mod settings;
mod custom_serializer;

const ICON_DIMENSION: u32 = 32;
const ICON_DIMENSION_SQUARED: u32 = ICON_DIMENSION * ICON_DIMENSION;
const ICON_SIZE: usize = (ICON_DIMENSION_SQUARED * 4) as usize;

static ICON_TOOLTIP: &str = "Simple Crosshair Overlay";

lazy_static! {
    static ref CONFIG_PATH: PathBuf = directories::ProjectDirs::from("dev.zkxs", "", "simple-crosshair-overlay").unwrap().config_dir().join("config.toml");
}

fn main() {
    let settings = match load_settings() {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Error loading settings at {}: {}", CONFIG_PATH.display(), e);
            LoadedSettings::default()
        }
    };
    let mut settings = Box::new(settings);

    let tray_menu = Menu::new();

    // on non-mac just append directly to the menu itself
    #[cfg(not(target_os = "macos"))] let root_menu = &tray_menu;
    // on mac all menu items must be in a submenu, so just make one with no name. Hope that doesn't cause problems...
    #[cfg(target_os = "macos")] let root_menu = {
        let submenu = tray_icon::menu::Submenu::new("", true);
        tray_menu.append(&submenu);
        submenu
    };

    let visible_button = CheckMenuItem::new("Visible", true, true, None);
    let adjust_button = CheckMenuItem::new("Adjust", true, false, None);
    let reset_button = MenuItem::new("Reset", true, None);
    let exit_button = MenuItem::new("Exit", true, None);
    root_menu.append(&visible_button);
    root_menu.append(&adjust_button);
    root_menu.append(&reset_button);
    root_menu.append(&exit_button);

    // keep the tray icon in an Option so we can take() it later to drop
    // windows: icon must be created on same thread as event loop
    // mac: icon and event loop must be created on main thread
    #[cfg(not(target_os = "linux"))] let mut tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(ICON_TOOLTIP)
            .with_icon(get_icon())
            .build()
            .unwrap()
    );

    #[cfg(target_os = "linux")]
    std::thread::spawn(|| {
        gtk::init().unwrap();

        // linux: icon must be created on same thread as gtk main loop,
        // and therefore can NOT be on the same thread as the event loop despite the tray-icon docs saying otherwise.
        // This means it's impossible to have it in scope for dropping later from the event loop
        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(ICON_TOOLTIP)
            .with_icon(get_icon())
            .build()
            .unwrap();

        gtk::main();
    });

    let menu_channel = MenuEvent::receiver();
    let event_loop = EventLoop::new();
    event_loop.set_device_event_filter(DeviceEventFilter::Never); // allow key capture even when the window is unfocused

    let window = init_gui(&event_loop, &settings);
    let context = unsafe { Context::new(&window) }.unwrap();
    let mut surface = unsafe { Surface::new(&context, &window) }.unwrap();

    // remember some application state that's NOT part of our saved config
    let mut window_visible = true;
    let mut control_pressed = false;
    let mut held_count: u32 = 0;

    // pass control to the event loop
    event_loop.run(move |event, _, control_flow| {
        control_flow.set_wait();

        match event {
            Event::RedrawRequested(_) => {
                // failsafe to resize the window before a redraw if necessary
                // ...and of course it's fucking necessary
                if window.inner_size() != settings.size() {
                    window.set_inner_size(settings.size());
                }

                draw_window(&mut surface, &settings)
            }
            Event::DeviceEvent { event: Key(keyboard_input), device_id: _device_id } => {
                if let Some(keycode) = keyboard_input.virtual_keycode {
                    // remember some select modifier keys
                    match keycode {
                        VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                            control_pressed = keyboard_input.state == ElementState::Pressed;
                        }
                        _ => ()
                    }

                    if adjust_button.is_checked() {
                        // adjust button IS checked
                        match keycode {
                            VirtualKeyCode::Up => {
                                if keyboard_input.state == ElementState::Pressed {
                                    settings.savable.window_dy -= speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::Down => {
                                if keyboard_input.state == ElementState::Pressed {
                                    settings.savable.window_dy += speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::Left => {
                                if keyboard_input.state == ElementState::Pressed {
                                    settings.savable.window_dx -= speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::Right => {
                                if keyboard_input.state == ElementState::Pressed {
                                    settings.savable.window_dx += speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::PageUp => {
                                if settings.is_scalable() && keyboard_input.state == ElementState::Pressed {
                                    settings.savable.window_height += speed_ramp(held_count);
                                    settings.savable.window_width = settings.savable.window_height;
                                    on_window_size_or_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::PageDown => {
                                if settings.is_scalable() && keyboard_input.state == ElementState::Pressed {
                                    settings.savable.window_height = settings.savable.window_height.checked_sub(speed_ramp(held_count)).unwrap_or(1).max(1);
                                    settings.savable.window_width = settings.savable.window_height;
                                    on_window_size_or_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::H => {
                                if control_pressed && keyboard_input.state == ElementState::Pressed {
                                    window_visible = !window_visible;
                                    window.set_visible(window_visible);
                                    if !window_visible {
                                        adjust_button.set_checked(false)
                                    }
                                }
                            }
                            VirtualKeyCode::J => {
                                if control_pressed && keyboard_input.state == ElementState::Pressed {
                                    adjust_button.set_checked(false)
                                }
                            }
                            _ => (),
                        }
                    } else {
                        // adjust button is NOT checked
                        match keycode {
                            VirtualKeyCode::H => {
                                if control_pressed && keyboard_input.state == ElementState::Pressed {
                                    window_visible = !window_visible;
                                    window.set_visible(window_visible);
                                    if !window_visible {
                                        adjust_button.set_checked(false)
                                    }
                                }
                            }
                            VirtualKeyCode::J => {
                                if control_pressed && keyboard_input.state == ElementState::Pressed && window_visible {
                                    adjust_button.set_checked(true)
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => ()
        }

        while let Ok(event) = menu_channel.try_recv() {
            match event.id {
                id if id == exit_button.id() => {
                    // drop the tray icon, solving the funny Windows issue where it lingers after application close
                    #[cfg(not(target_os = "linux"))]
                    tray_icon.take(); // yeah this is simply impossible on Linux, so good luck dropping this at the correct time :)
                    window.set_visible(false);
                    if let Err(e) = save_settings(&settings) {
                        eprintln!("Error saving settings at {}: {}", CONFIG_PATH.display(), e);
                    }

                    control_flow.set_exit();
                    break;
                }
                id if id == reset_button.id() => {
                    *settings = LoadedSettings::default();
                    on_window_size_or_position_change(&window, &settings);
                }
                _ => (),
            }
        }
    });
}

/// Handles both window size and position change side effects.
fn on_window_size_or_position_change(window: &Window, settings: &LoadedSettings) {
    window.set_inner_size(settings.size());
    window.set_outer_position(compute_window_coordinates(window, settings));

    /*
    TODO: scaling jitter problem
        When the application is scaled really quickly via key-repeat spam it struggles to scale, move, and redraw the window in perfect sync.
        To fix this I'd have to completely rearchitect how scaling works. Ideas:
        1. Temporarily size the window to full screen, thereby eliminating all but the redraws
        2. Stop relying on key repeat and instead remember key state and use ticks for your update intervals
    */
}

/// Slightly cheaper special case that can only handle window position changes. Do not use this if the window size may have changed.
fn on_window_position_change(window: &Window, settings: &LoadedSettings) {
    window.set_outer_position(compute_window_coordinates(window, settings));
}

fn speed_ramp(held_count: u32) -> u32 {
    if held_count < 10 {
        // 0-9
        1
    } else if held_count < 20 {
        // 10-19
        4
    } else if held_count < 40 {
        // 20-39
        16
    } else if held_count < 60 {
        // 40-59
        32
    } else {
        // 60+
        64
    }
}

/// Compute the correct coordinates of the top-left of the window in order to center the crosshair in the primary monitor
fn compute_window_coordinates(window: &Window, settings: &LoadedSettings) -> PhysicalPosition<i32> {
    let monitor = window.primary_monitor().unwrap();
    let PhysicalPosition { x: monitor_x, y: monitor_y } = monitor.position();
    let PhysicalSize { width: monitor_width, height: monitor_height } = monitor.size();
    let monitor_width = i32::try_from(monitor_width).unwrap();
    let monitor_height = i32::try_from(monitor_height).unwrap();
    let window_width = settings.savable.window_width as i32;
    let window_height = settings.savable.window_height as i32;
    let monitor_center_x = (monitor_width - monitor_x) / 2;
    let monitor_center_y = (monitor_height - monitor_y) / 2;
    let window_x = monitor_center_x - (window_width / 2) + settings.savable.window_dx;
    let window_y = monitor_center_y - (window_height / 2) + settings.savable.window_dy;
    PhysicalPosition::new(window_x, window_y)
}

/// draws a crosshair image, or a simple red crosshair if no image is set
fn draw_window(surface: &mut Surface, settings: &LoadedSettings) {
    let PhysicalSize { width: window_width, height: window_height } = settings.size();
    surface.resize(
        NonZeroU32::new(window_width).unwrap(),
        NonZeroU32::new(window_height).unwrap(),
    ).unwrap();

    let mut buffer = surface.buffer_mut().unwrap();

    if buffer.age() == 0 {
        if let Some(image) = &settings.image {
            // draw our image
            buffer.copy_from_slice(image.data.as_slice());
        } else {
            // draw a generated crosshair

            const FULL_ALPHA: u32 = 0x00000000;

            let width = settings.savable.window_width as usize;
            let height = settings.savable.window_height as usize;

            if width <= 2 || height <= 2 {
                // edge case where there simply aren't enough pixels to draw a crosshair, so we just fall back to a dot
                buffer.fill(settings.color);
            } else {
                buffer.fill(FULL_ALPHA);

                // horizontal line
                let start = width * (height / 2);
                for x in start..start + width {
                    buffer[x] = settings.color;
                }

                // second horizontal line (if size is even we need this for centering)
                if height % 2 == 0 {
                    let start = start - width;
                    for x in start..start + width {
                        buffer[x] = settings.color;
                    }
                }

                // vertical line
                for y in 0..height {
                    buffer[width * y + width / 2] = settings.color;
                }

                // second vertical line (if size is even we need this for centering)
                if width % 2 == 0 {
                    for y in 0..height {
                        buffer[width * y + width / 2 - 1] = settings.color;
                    }
                }
            }
        }
    }

    buffer.present().unwrap();
}

fn load_settings() -> Result<LoadedSettings, String> {
    fs::create_dir_all(CONFIG_PATH.as_path().parent().unwrap()).map_err(|e| format!("{e:?}"))?;
    fs::read_to_string(CONFIG_PATH.as_path()).map_err(|e| format!("{e:?}"))
        .and_then(|string| toml::from_str::<SavableSettings>(&string).map_err(|e| format!("{e:?}")))
        .and_then(|settings| settings.load())
}

fn save_settings(settings: &LoadedSettings) -> Result<(), String> {
    let serialized_config = toml::to_string(&settings.savable).expect("failed to serialize settings");
    fs::write(CONFIG_PATH.as_path(), serialized_config).map_err(|e| format!("{e:?}"))
}

fn get_icon() -> Icon {
    Icon::from_rgba(get_icon_rgba(), ICON_DIMENSION, ICON_DIMENSION).unwrap()
}

//TODO: use an actual graphic and not just a generated placeholder
fn get_icon_rgba() -> Vec<u8> {
    // some silly math to make a colored circle
    let mut icon_rgba: Vec<u8> = Vec::with_capacity(ICON_SIZE);
    #[allow(clippy::uninit_vec)]
    unsafe {
        icon_rgba.set_len(icon_rgba.capacity());
    }
    for x in 0..ICON_DIMENSION {
        for y in 0..ICON_DIMENSION {
            let x_term = ((x as i32) * 2 - (ICON_DIMENSION as i32) + 1) / 2;
            let y_term = ((y as i32) * 2 - (ICON_DIMENSION as i32) + 1) / 2;
            let distance_squared = x_term * x_term + y_term * y_term;
            let color: u8 = if distance_squared < ICON_DIMENSION_SQUARED as i32 / 4 {
                255
            } else {
                0
            };
            let icon_offset: usize = (x as usize * ICON_DIMENSION as usize + y as usize) * 4;
            icon_rgba[icon_offset] = color; // set red
            icon_rgba[icon_offset + 1] = (x * 4) as u8; // set green
            icon_rgba[icon_offset + 2] = (y * 4) as u8; // set blue
            icon_rgba[icon_offset + 3] = color; // set alpha
        }
    }
    icon_rgba
}

fn init_gui(event_loop: &EventLoop<()>, settings: &LoadedSettings) -> Window {
    let window = WindowBuilder::new()
        .with_visible(false) // things get very buggy on Windows if you default the window to invisible...
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(false)
        .with_drag_and_drop(false)
        .with_skip_taskbar(true)
        .with_title("Simple Crosshair Overlay")
        .with_position(PhysicalPosition::new(0, 0)) // can't determine monitor size until the window is created, so just use some dummy values
        .with_inner_size(PhysicalSize::new(1, 1)) // this might flicker so make it very tiny
        .build(event_loop)
        .unwrap();

    // contrary to all my expectations this call appears to work reliably
    window.set_outer_position(compute_window_coordinates(&window, settings));

    // this call is very fragile (read: shit) and sometimes simply doesn't do anything.
    // There's a fallback call up in the event loop that saves us when this fails.
    window.set_inner_size(settings.size());

    // once the window is ready, show it
    window.set_visible(true);

    // set these weirder settings AFTER the window is visible to avoid even more buggy Windows behavior
    // Windows particularly hates if you unset cursor_hittest while the window is hidden
    window.set_cursor_hittest(false).unwrap();
    window.set_window_level(WindowLevel::AlwaysOnTop);

    window
}

