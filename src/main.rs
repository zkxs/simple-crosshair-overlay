// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

#![windows_subsystem = "windows"] // necessary to remove the console window on Windows

use std::io;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;

use lazy_static::lazy_static;
use native_dialog::{FileDialog, MessageDialog, MessageType};
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

use crate::settings::Settings;

mod settings;
mod custom_serializer;

const ICON_DIMENSION: u32 = 32;
const ICON_DIMENSION_SQUARED: u32 = ICON_DIMENSION * ICON_DIMENSION;
const ICON_SIZE: usize = (ICON_DIMENSION_SQUARED * 4) as usize;

static ICON_TOOLTIP: &str = "Simple Crosshair Overlay";

lazy_static! {
    pub static ref CONFIG_PATH: PathBuf = directories::ProjectDirs::from("dev.zkxs", "", "simple-crosshair-overlay").unwrap().config_dir().join("config.toml");

    // this is some arcane bullshit to get a global mpsc
    // the sender can be cloned, and we'll do that via a thread_local later
    // the receiver can't be cloned, so just shove it in an Option so we can take() it later.
    static ref DIALOG_REQUEST_CHANNEL: (Mutex<mpsc::Sender<DialogRequest>>, Mutex<Option<mpsc::Receiver<DialogRequest>>>) = {
        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(sender);
        let receiver = Mutex::new(Some(receiver));
        (sender, receiver)
    };
}

thread_local! {
    static DIALOG_REQUEST_SENDER: mpsc::Sender<DialogRequest> = DIALOG_REQUEST_CHANNEL.0.lock().unwrap().clone();
}

fn main() {
    let settings = match Settings::load() {
        Ok(settings) => settings,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Settings::default(), // generate new settings file when it doesn't exist
        Err(e) => {
            show_warning(format!("Error loading settings file \"{}\". Resetting to default settings.\n\n{}", CONFIG_PATH.display(), e));
            Settings::default()
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
    let image_pick_button = MenuItem::new("Load Image", true, None);
    let reset_button = MenuItem::new("Reset", true, None);
    let exit_button = MenuItem::new("Exit", true, None);
    root_menu.append(&visible_button);
    root_menu.append(&adjust_button);
    root_menu.append(&image_pick_button);
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
    std::thread::Builder::new()
        .name("gtk-main".to_string())
        .spawn(|| {
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
        }).unwrap();

    let (file_path_sender, file_path_receiver) = mpsc::channel();
    let dialog_request_receiver = DIALOG_REQUEST_CHANNEL.1.lock().unwrap().take().unwrap();

    // native dialogs block a thread, so we'll spin up a single thread to loop through queued dialogs.
    // If we ever need to show multiple dialogs, they just get queued.
    let dialog_worker_join_handle = std::thread::Builder::new()
        .name("dialog-worker".to_string())
        .spawn(move || {
            loop {
                // block waiting for a file read request
                match dialog_request_receiver.recv().unwrap() {
                    DialogRequest::PngPath => {
                        let path = FileDialog::new()
                            .add_filter("PNG Image", &["png"])
                            .show_open_single_file()
                            .ok()
                            .flatten();

                        let _ = file_path_sender.send(path);
                    }
                    DialogRequest::Warning(text) => {
                        MessageDialog::new()
                            .set_type(MessageType::Warning)
                            .set_title("Simple Crosshair Overlay")
                            .set_text(&text)
                            .show_alert()
                            .unwrap();
                    }
                    DialogRequest::Terminate => break,
                }
            }
        }).unwrap();
    let mut dialog_worker_join_handle = Some(dialog_worker_join_handle); // we take() from this later

    let menu_channel = MenuEvent::receiver();
    let event_loop = EventLoop::new();
    event_loop.set_device_event_filter(DeviceEventFilter::Never); // allow key capture even when the window is unfocused

    let window = init_gui(&event_loop, &settings);
    let context = unsafe { Context::new(&window) }.unwrap();
    let mut surface = unsafe { Surface::new(&context, &window) }.unwrap();

    // remember some application state that's NOT part of our saved config
    let mut window_visible = true;
    let mut control_pressed = false;
    let mut held_count: u32 = 0; // a really terrible count of how many "frames" we've been holding a button. But it's not frames, and it's not accurate.
    let mut force_redraw = false; // if set to true, the next redraw will be forced even for known buffer contents

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

                draw_window(&mut surface, &settings, force_redraw);
                force_redraw = false;
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
                                    settings.persisted.window_dy -= speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::Down => {
                                if keyboard_input.state == ElementState::Pressed {
                                    settings.persisted.window_dy += speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::Left => {
                                if keyboard_input.state == ElementState::Pressed {
                                    settings.persisted.window_dx -= speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::Right => {
                                if keyboard_input.state == ElementState::Pressed {
                                    settings.persisted.window_dx += speed_ramp(held_count) as i32;
                                    on_window_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::PageUp => {
                                if settings.is_scalable() && keyboard_input.state == ElementState::Pressed {
                                    settings.persisted.window_height += speed_ramp(held_count);
                                    settings.persisted.window_width = settings.persisted.window_height;
                                    on_window_size_or_position_change(&window, &settings);
                                    held_count += 1;
                                } else {
                                    held_count = 0;
                                }
                            }
                            VirtualKeyCode::PageDown => {
                                if settings.is_scalable() && keyboard_input.state == ElementState::Pressed {
                                    settings.persisted.window_height = settings.persisted.window_height.checked_sub(speed_ramp(held_count)).unwrap_or(1).max(1);
                                    settings.persisted.window_width = settings.persisted.window_height;
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

        if let Ok(path) = file_path_receiver.try_recv() {
            image_pick_button.set_enabled(true);

            if let Some(path) = path {
                match settings.load_png(path) {
                    Ok(()) => {
                        force_redraw = true;
                        on_window_size_or_position_change(&window, &settings);
                    }
                    Err(e) => show_warning(format!("Error loading PNG.\n\n{}", e))
                }
            }
        }

        while let Ok(event) = menu_channel.try_recv() {
            match event.id {
                id if id == exit_button.id() => {
                    // drop the tray icon, solving the funny Windows issue where it lingers after application close
                    #[cfg(not(target_os = "linux"))]
                    tray_icon.take(); // yeah this is simply impossible on Linux, so good luck dropping this at the correct time :)
                    window.set_visible(false);
                    if let Err(e) = settings.save() {
                        show_warning(format!("Error saving settings to \"{}\".\n\n{}", CONFIG_PATH.display(), e));
                    }

                    // kill the dialog worker and wait for it to finish
                    // this makes the application remain open until the user has clicked through any queued dialogs
                    terminate_dialog_worker();
                    if let Some(handle) = dialog_worker_join_handle.take() {
                        handle.join().unwrap();
                    }

                    control_flow.set_exit();
                    break;
                }
                id if id == reset_button.id() => {
                    settings.reset();
                    force_redraw = true;
                    on_window_size_or_position_change(&window, &settings);
                }
                id if id == image_pick_button.id() => {
                    image_pick_button.set_enabled(false);
                    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::PngPath));
                }
                _ => (),
            }
        }
    });
}

/// Handles both window size and position change side effects.
fn on_window_size_or_position_change(window: &Window, settings: &Settings) {
    window.set_inner_size(settings.size());
    window.set_outer_position(compute_window_coordinates(window, settings));
    window.request_redraw(); // needed in case the window size didn't change but the image was replaced

    /*
    TODO: scaling jitter problem
        When the application is scaled really quickly via key-repeat spam it struggles to scale, move, and redraw the window in perfect sync.
        To fix this I'd have to completely rearchitect how scaling works. Ideas:
        1. Temporarily size the window to full screen, thereby eliminating all but the redraws
        2. Stop relying on key repeat and instead remember key state and use ticks for your update intervals
    */
}

/// Slightly cheaper special case that can only handle window position changes. Do not use this if the window size may have changed.
fn on_window_position_change(window: &Window, settings: &Settings) {
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
fn compute_window_coordinates(window: &Window, settings: &Settings) -> PhysicalPosition<i32> {
    let monitor = window.primary_monitor().unwrap();
    let PhysicalPosition { x: monitor_x, y: monitor_y } = monitor.position();
    let PhysicalSize { width: monitor_width, height: monitor_height } = monitor.size();
    let monitor_width = i32::try_from(monitor_width).unwrap();
    let monitor_height = i32::try_from(monitor_height).unwrap();
    let PhysicalSize { width: window_width, height: window_height } = settings.size();
    let window_width = window_width as i32;
    let window_height = window_height as i32;
    let monitor_center_x = (monitor_width - monitor_x) / 2;
    let monitor_center_y = (monitor_height - monitor_y) / 2;
    let window_x = monitor_center_x - (window_width / 2) + settings.persisted.window_dx;
    let window_y = monitor_center_y - (window_height / 2) + settings.persisted.window_dy;
    PhysicalPosition::new(window_x, window_y)
}

/// draws a crosshair image, or a simple red crosshair if no image is set
fn draw_window(surface: &mut Surface, settings: &Settings, force: bool) {
    let PhysicalSize { width: window_width, height: window_height } = settings.size();
    surface.resize(
        NonZeroU32::new(window_width).unwrap(),
        NonZeroU32::new(window_height).unwrap(),
    ).unwrap();

    let mut buffer = surface.buffer_mut().unwrap();

    if force || buffer.age() == 0 {
        if let Some(image) = &settings.image {
            // draw our image
            buffer.copy_from_slice(image.data.as_slice());
        } else {
            // draw a generated crosshair

            const FULL_ALPHA: u32 = 0x00000000;

            let width = settings.persisted.window_width as usize;
            let height = settings.persisted.window_height as usize;

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

fn init_gui(event_loop: &EventLoop<()>, settings: &Settings) -> Window {
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

enum DialogRequest {
    PngPath,
    Warning(String),
    Terminate,
}

pub fn show_warning(text: String) {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Warning(text)));
}

pub fn terminate_dialog_worker() {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Terminate));
}
