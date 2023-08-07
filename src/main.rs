// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

#![windows_subsystem = "windows"] // necessary to remove the console window on Windows

use std::io;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;

use device_query::{DeviceQuery, DeviceState};
use lazy_static::lazy_static;
use native_dialog::{FileDialog, MessageDialog, MessageType};
use softbuffer::{Context, Surface};
use tray_icon::{menu::Menu, TrayIconBuilder};
use tray_icon::icon::Icon as TrayIcon;
use tray_icon::menu::{CheckMenuItem, MenuEvent, MenuItem, MenuItemExt, Submenu};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder, WindowLevel};

use crate::hotkey::HotkeyManager;
use crate::settings::Settings;

mod settings;
mod custom_serializer;
mod hotkey;
mod util;

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
    // We only need one of these per thread. As we don't use any thread pools this should be a one-time cost on application startup.
    static DIALOG_REQUEST_SENDER: mpsc::Sender<DialogRequest> = DIALOG_REQUEST_CHANNEL.0.lock().unwrap().clone();
}

/// constants generated in build.rs
mod build_constants {
    include!(env!("CONSTANTS_PATH"));
}

fn main() {
    let mut settings = Box::new(
        match Settings::load() {
            Ok(settings) => settings,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Settings::default(), // generate new settings file when it doesn't exist
            Err(e) => {
                show_warning(format!("Error loading settings file \"{}\". Resetting to default settings.\n\n{}", CONFIG_PATH.display(), e));
                Settings::default()
            }
        }
    );

    let mut hotkey_manager = Box::new(
        match HotkeyManager::new(&settings.persisted.key_bindings) {
            Ok(hotkey_manager) => hotkey_manager,
            Err(e) => {
                show_warning(format!("{e}\n\nUsing default hotkeys."));
                HotkeyManager::default()
            }
        }
    );

    // on non-linux we need this in scope
    #[cfg(not(target_os = "linux"))] let tray_menu = Menu::new();

    // windows: icon must be created on same thread as event loop
    #[cfg(target_os = "windows")] let menu_items = {
        let menu_items = MenuItems::default();
        menu_items.add_to_menu(&tray_menu);
        menu_items
    };

    // mac: icon and event loop must be created on main thread
    #[cfg(target_os = "macos")] let menu_items = {
        // on mac all menu items must be in a submenu, so just make one with no name. Hope that doesn't cause problems...
        let submenu = tray_icon::menu::Submenu::new("", true);
        tray_menu.append(&submenu);

        let menu_items = MenuItems::default();
        menu_items.add_to_menu(&submenu);
        menu_items
    };

    #[cfg(target_os = "linux")] let menu_items = {
        let menu_items = MenuItems::default();
        let menu_items_mutex = Mutex::new(menu_items.clone());

        std::thread::Builder::new()
            .name("gtk-main".to_string())
            .spawn(|| {
                gtk::init().unwrap();

                let tray_menu = Menu::new();

                let menu_items_guard = menu_items_mutex.lock().unwrap();
                menu_items_guard.add_to_menu(&tray_menu);

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

        menu_items
    };

    // keep the tray icon in an Option so we can take() it later to drop
    #[cfg(not(target_os = "linux"))] let mut tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(ICON_TOOLTIP)
            .with_icon(get_icon())
            .build()
            .unwrap()
    );

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
                    DialogRequest::Info(text) => {
                        MessageDialog::new()
                            .set_type(MessageType::Info)
                            .set_title("Simple Crosshair Overlay")
                            .set_text(&text)
                            .show_alert()
                            .unwrap();
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

    let user_event_sender = event_loop.create_proxy();
    let key_process_interval = settings.tick_interval;
    std::thread::Builder::new()
        .name("tick-sender".to_string())
        .spawn(move || {
            loop {
                let _ = user_event_sender.send_event(());
                std::thread::sleep(key_process_interval);
            }
        }).unwrap();

    let device_state = DeviceState::new();

    // unsafe note: these three structs MUST live and die together.
    // It is highly illegal to use the context or surface after the window is dropped.
    // The context only gets used right here, so that's fine.
    // As of this writing, none of these get moved. Therefore they all get dropped one after the other at the end of main(), which is safe.
    let window = init_gui(&event_loop, &settings);
    let context = unsafe { Context::new(&window) }.unwrap();
    let mut surface = unsafe { Surface::new(&context, &window) }.unwrap();

    // remember some application state that's NOT part of our saved config
    let mut window_visible = true;
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
            Event::UserEvent(_) => {
                let keys = device_state.get_keys();
                hotkey_manager.process_keys(&keys);

                if menu_items.adjust_button.is_checked() {
                    let mut window_position_dirty = false;
                    let mut window_scale_dirty = false;

                    if hotkey_manager.move_up() != 0 {
                        settings.persisted.window_dy -= hotkey_manager.move_up() as i32;
                        window_position_dirty = true;
                    }

                    if hotkey_manager.move_down() != 0 {
                        settings.persisted.window_dy += hotkey_manager.move_down() as i32;
                        window_position_dirty = true;
                    }

                    if hotkey_manager.move_left() != 0 {
                        settings.persisted.window_dx -= hotkey_manager.move_left() as i32;
                        window_position_dirty = true;
                    }

                    if hotkey_manager.move_right() != 0 {
                        settings.persisted.window_dx += hotkey_manager.move_right() as i32;
                        window_position_dirty = true;
                    }


                    if settings.is_scalable() && hotkey_manager.scale_increase() != 0 {
                        settings.persisted.window_height += hotkey_manager.scale_increase();
                        settings.persisted.window_width = settings.persisted.window_height;
                        window_scale_dirty = true;
                    }

                    if settings.is_scalable() && hotkey_manager.scale_decrease() != 0 {
                        settings.persisted.window_height = settings.persisted.window_height.checked_sub(hotkey_manager.scale_decrease()).unwrap_or(1).max(1);
                        settings.persisted.window_width = settings.persisted.window_height;
                        window_scale_dirty = true;
                    }

                    // adjust button is already checked
                    if hotkey_manager.toggle_adjust() {
                        menu_items.adjust_button.set_checked(false)
                    }

                    if window_scale_dirty {
                        on_window_size_or_position_change(&window, &settings);
                    } else if window_position_dirty {
                        on_window_position_change(&window, &settings);
                    }
                } else if hotkey_manager.toggle_adjust() {
                    // adjust button is NOT checked
                    menu_items.adjust_button.set_checked(true)
                }

                if hotkey_manager.toggle_hidden() {
                    window_visible = !window_visible;
                    window.set_visible(window_visible);
                    if !window_visible {
                        menu_items.adjust_button.set_checked(false)
                    }
                }
            }
            _ => (),
        }

        if let Ok(path) = file_path_receiver.try_recv() {
            menu_items.image_pick_button.set_enabled(true);

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
                id if id == menu_items.exit_button.id() => {
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
                id if id == menu_items.reset_button.id() => {
                    settings.reset();
                    force_redraw = true;
                    on_window_size_or_position_change(&window, &settings);
                }
                id if id == menu_items.image_pick_button.id() => {
                    menu_items.image_pick_button.set_enabled(false);
                    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::PngPath));
                }
                id if id == menu_items.about_button.id() => {
                    show_info(format!("{}\nversion {}", build_constants::APPLICATION_NAME, env!("CARGO_PKG_VERSION")));
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

fn get_icon() -> TrayIcon {
    TrayIcon::from_rgba(include_bytes!(env!("TRAY_ICON_PATH")).to_vec(), build_constants::TRAY_ICON_DIMENSION, build_constants::TRAY_ICON_DIMENSION).unwrap()
}

fn init_gui(event_loop: &EventLoop<()>, settings: &Settings) -> Window {
    let window_builder = WindowBuilder::new()
        .with_visible(false) // things get very buggy on Windows if you default the window to invisible...
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(false)
        .with_title("Simple Crosshair Overlay")
        .with_position(PhysicalPosition::new(0, 0)) // can't determine monitor size until the window is created, so just use some dummy values
        .with_inner_size(PhysicalSize::new(1, 1)); // this might flicker so make it very tiny

    #[cfg(target_os = "windows")] let window_builder = {
        use winit::platform::windows::WindowBuilderExtWindows;
        window_builder
            .with_drag_and_drop(false)
            .with_skip_taskbar(true)
    };

    let window = window_builder.build(event_loop)
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

pub fn show_info(text: String) {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Info(text)));
}

pub fn show_warning(text: String) {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Warning(text)));
}

pub fn terminate_dialog_worker() {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Terminate));
}

#[derive(Clone)]
struct MenuItems {
    visible_button: CheckMenuItem,
    adjust_button: CheckMenuItem,
    image_pick_button: MenuItem,
    reset_button: MenuItem,
    about_button: MenuItem,
    exit_button: MenuItem,
}

impl Default for MenuItems {
    fn default() -> Self {
        let visible_button = CheckMenuItem::new("Visible", true, true, None);
        let adjust_button = CheckMenuItem::new("Adjust", true, false, None);
        let image_pick_button = MenuItem::new("Load Image", true, None);
        let reset_button = MenuItem::new("Reset Overlay", true, None);
        let about_button = MenuItem::new("About", true, None);
        let exit_button = MenuItem::new("Exit", true, None);

        MenuItems {
            visible_button,
            adjust_button,
            image_pick_button,
            reset_button,
            about_button,
            exit_button,
        }
    }
}

impl MenuItems {
    fn add_to_menu<T>(&self, menu: &T) where T: AppendableMenu {
        menu.append(&self.visible_button);
        menu.append(&self.adjust_button);
        menu.append(&self.image_pick_button);
        menu.append(&self.reset_button);
        menu.append(&self.about_button);
        menu.append(&self.exit_button);
    }
}

trait AppendableMenu {
    fn append(&self, item: &dyn MenuItemExt);
}

impl AppendableMenu for Menu {
    fn append(&self, item: &dyn MenuItemExt) {
        self.append(item);
    }
}

impl AppendableMenu for Submenu {
    fn append(&self, item: &dyn MenuItemExt) {
        self.append(item);
    }
}

enum DialogRequest {
    PngPath,
    Info(String),
    Warning(String),
    Terminate,
}
