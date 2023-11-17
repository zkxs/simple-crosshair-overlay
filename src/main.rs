// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

#![windows_subsystem = "windows"] // necessary to remove the console window on Windows

use std::io;
use std::num::NonZeroU32;
use std::rc::Rc;

use debug_print::debug_println;
use softbuffer::{Context, Surface};
use tray_icon::{Icon as TrayIcon, menu::Menu, TrayIconBuilder};
use tray_icon::menu::{CheckMenuItem, IsMenuItem, MenuEvent, MenuItem, Result as MenuResult, Submenu};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{DeviceEvents, EventLoop};
use winit::window::{CursorGrabMode, CursorIcon, Window, WindowBuilder, WindowLevel};

use simple_crosshair_overlay::platform;
use simple_crosshair_overlay::platform::HotkeyManager;
use simple_crosshair_overlay::settings::{RenderMode, Settings};
use simple_crosshair_overlay::settings::CONFIG_PATH;
use simple_crosshair_overlay::util::image;
use simple_crosshair_overlay::util::dialog;

static ICON_TOOLTIP: &str = "Simple Crosshair Overlay";

/// constants generated in build.rs
mod build_constants {
    include!(env!("CONSTANTS_PATH"));
}

fn main() {
    // settings has a decent quantity of data in it, but it never really gets moved so we can just leave it on the stack
    // the image buffer is internally boxed so don't worry about that
    let mut settings = match Settings::load() {
        Ok(settings) => settings,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Settings::default(), // generate new settings file when it doesn't exist
        Err(e) => {
            dialog::show_warning(format!("Error loading settings file \"{}\". Resetting to default settings.\n\n{}", CONFIG_PATH.display(), e));
            Settings::default()
        }
    };

    // HotkeyManager has a decent quantity of data in it, but again it never really gets moved so we can just leave it on the stack
    let mut hotkey_manager = match HotkeyManager::new(&settings.persisted.key_bindings) {
        Ok(hotkey_manager) => hotkey_manager,
        Err(e) => {
            dialog::show_warning(format!("{e}\n\nUsing default hotkeys."));
            HotkeyManager::default()
        }
    };

    let tray_menu = Menu::new();

    // on windows/linux/mac: icon must be created on same thread as event loop

    // not mac: do not use a submenu
    #[cfg(not(target_os = "macos"))] let menu_items = {
        let menu_items = MenuItems::default();
        menu_items.add_to_menu(&tray_menu);
        menu_items
    };

    // mac: there are special submenu requirements
    #[cfg(target_os = "macos")] let menu_items = {
        // on mac all menu items must be in a submenu, so just make one with no name. Hope that doesn't cause problems...
        let submenu = tray_icon::menu::Submenu::new("", true);
        tray_menu.append(&submenu).unwrap();

        let menu_items = MenuItems::default();
        menu_items.add_to_menu(&submenu);
        menu_items
    };

    #[cfg(target_os = "linux")] {
        use std::sync::{Arc, Condvar, Mutex};
        use std::time::Duration;

        let condvar_pair = Arc::new((Mutex::new(false), Condvar::new()));

        // start GTK background thread
        let condvar_pair_clone = condvar_pair.clone();
        std::thread::Builder::new()
            .name("gtk-main".to_string())
            .spawn(move || {
                gtk::init().unwrap();

                // signal that GTK init is complete
                let (lock, condvar) = &*condvar_pair_clone;
                let mut gtk_started = lock.lock().unwrap();
                *gtk_started = true;
                condvar.notify_one();

                gtk::main();
            }).unwrap();

        // wait for GTK to init
        let (lock, condvar) = &*condvar_pair;
        let mut gtk_started = lock.lock().unwrap();
        if !*gtk_started {
            let (gtk_started, timeout_result) = condvar.wait_timeout(gtk_started, Duration::from_secs(5)).unwrap();
            if !*gtk_started {
                panic!("GTK startup timed out = {}", timeout_result.timed_out());
            }
        }
    }

    // keep the tray icon in an Option so we can take() it later to drop
    let mut tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(ICON_TOOLTIP)
            .with_icon(get_icon())
            .build()
            .unwrap()
    );

    // native dialogs block a thread, so we'll spin up a single thread to loop through queued dialogs.
    // If we ever need to show multiple dialogs, they just get queued.
    let mut dialog_worker = dialog::spawn_worker();

    let menu_channel = MenuEvent::receiver();
    let event_loop = EventLoop::new().unwrap();
    event_loop.listen_device_events(DeviceEvents::Always);

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

    // unsafe note: these three structs MUST live and die together.
    // It is highly illegal to use the context or surface after the window is dropped.
    // The context only gets used right here, so that's fine.
    // As of this writing, none of these get moved. Therefore they all get dropped one after the other at the end of main(), which is safe.
    let window = Rc::new(init_window(&event_loop, &mut settings));
    let context = Context::new(window.clone()).unwrap();
    let mut surface = Surface::new(&context, window.clone()).unwrap();

    // remember some application state that's NOT part of our saved config
    let mut window_visible = true;
    let mut force_redraw = false; // if set to true, the next redraw will be forced even for known buffer contents
    let mut last_mouse_position = PhysicalPosition::default();

    let mut last_focused_window: Option<platform::WindowHandle> = None;

    // pass control to the event loop
    event_loop.run(move |event, window_target| {
        // in theory Wait is now the default ControlFlow, so the following isn't needed:
        // window_target.set_control_flow(ControlFlow::Wait);

        let mut window_position_dirty = false;
        let mut window_scale_dirty = false;

        match event {
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // failsafe to resize the window before a redraw if necessary
                // ...and of course it's fucking necessary
                settings.validate_window_size(&window, window.inner_size());
                draw_window(&mut surface, &settings, force_redraw);
                force_redraw = false;
            }
            Event::UserEvent(_) => {
                hotkey_manager.poll_keys();
                hotkey_manager.process_keys();

                let adjust_mode = menu_items.adjust_button.is_checked();
                if adjust_mode {
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

                    if hotkey_manager.cycle_monitor() {
                        settings.monitor_index = (settings.monitor_index + 1) % window.available_monitors().count();
                        window_scale_dirty = true;
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

                // only enable this hotkey if the color picker is already visible OR if adjust mode is on
                if hotkey_manager.toggle_color_picker() && (adjust_mode || settings.get_pick_color()) {
                    let color_pick = settings.toggle_pick_color();
                    menu_items.color_pick_button.set_checked(color_pick);
                    handle_color_pick(color_pick, &window, &mut last_focused_window, true);
                    window_scale_dirty = true;
                }
            }
            Event::WindowEvent { event: WindowEvent::Moved(position), .. } => {
                // incredibly, if the taskbar is at the top or left of the screen Windows will
                // (un)helpfully shift the window over by the taskbar's size. I have no idea why
                // this happens and it's terrible, but luckily Windows tells me it's done this so
                // that I can immediately detect and undo it.
                debug_println!("window position changed to {:?}", position);
                settings.validate_window_position(&window, position);
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                // See above nightmare scenario with the window position. I figure I might as well
                // do the same thing for size just in case Windows also has some arcane, evil
                // involuntary resizing behavior.
                debug_println!("window size changed to {:?}", size);
                settings.validate_window_size(&window, size);
            }
            Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                last_mouse_position = position;
            }
            Event::WindowEvent { event: WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. }, .. } => {
                let PhysicalPosition { x, y } = last_mouse_position;
                let x = x as usize;
                let y = y as usize;

                let PhysicalSize { width, height } = settings.size();
                let width = width as usize;
                let height = height as usize;

                settings.set_color(image::hue_alpha_color_from_coordinates(x, y, width, height));
                menu_items.color_pick_button.set_checked(false);
                handle_color_pick(false, &window, &mut last_focused_window, false);
                window_scale_dirty = true;
            }
            _ => ()
        }

        if let Ok(path) = dialog_worker.try_recv_file_path() {
            menu_items.image_pick_button.set_enabled(true);

            if let Some(path) = path {
                match settings.load_png(path) {
                    Ok(()) => {
                        force_redraw = true;
                        window_scale_dirty = true;
                    }
                    Err(e) => dialog::show_warning(format!("Error loading PNG.\n\n{}", e))
                }
            }
        }

        while let Ok(event) = menu_channel.try_recv() {
            match event.id {
                id if id == menu_items.exit_button.id() => {
                    // drop the tray icon, solving the funny Windows issue where it lingers after application close
                    tray_icon.take();
                    window.set_visible(false);
                    if let Err(e) = settings.save() {
                        dialog::show_warning(format!("Error saving settings to \"{}\".\n\n{}", CONFIG_PATH.display(), e));
                    }

                    // kill the dialog worker and wait for it to finish
                    // this makes the application remain open until the user has clicked through any queued dialogs
                    dialog_worker.shutdown().expect("failed to shut down dialog worker");

                    window_target.exit();
                    break;
                }
                id if id == menu_items.visible_button.id() => {
                    window.set_visible(menu_items.visible_button.is_checked());
                }
                id if id == menu_items.reset_button.id() => {
                    settings.reset();
                    force_redraw = true;
                    window_scale_dirty = true;
                }
                id if id == menu_items.color_pick_button.id() => {
                    let pick_color = menu_items.color_pick_button.is_checked();
                    settings.set_pick_color(pick_color);
                    handle_color_pick(pick_color, &window, &mut last_focused_window, false);
                    window_scale_dirty = true;
                }
                id if id == menu_items.image_pick_button.id() => {
                    menu_items.image_pick_button.set_enabled(false);
                    dialog::request_png();
                }
                id if id == menu_items.about_button.id() => {
                    dialog::show_info(format!("{}\nversion {} {}", build_constants::APPLICATION_NAME, env!("CARGO_PKG_VERSION"), env!("GIT_COMMIT_HASH")));
                }
                _ => (),
            }
        }

        if window_scale_dirty {
            on_window_size_or_position_change(&window, &mut settings);
        } else if window_position_dirty {
            on_window_position_change(&window, &mut settings);
        }
    }).unwrap();
}

/// Updates the window state after entering or exiting color picker mode
///
/// If `save_focused` is `true`, this will make a best-effort to restore the previously focused window next time we exit color pick mode.
fn handle_color_pick(color_pick: bool, window: &Window, last_focused_window: &mut Option<platform::WindowHandle>, save_focused: bool) {
    if color_pick {
        *last_focused_window = if save_focused {
            // back up the last-focused window right before we focus ourself
            platform::get_foreground_window()
        } else {
            // make sure we don't have some weird old window handle saved if we shouldn't be saving focus
            None
        };
        window.set_cursor_hittest(true).unwrap();
        window.focus_window();
        window.set_cursor_grab(CursorGrabMode::Confined).unwrap(); // if we do this after the window is focused, it'll move the cursor to the window for us.
    } else {
        window.set_cursor_grab(CursorGrabMode::None).unwrap();
        window.set_cursor_hittest(false).unwrap();
        if let Some(last_focused_window) = *last_focused_window {
            let _success = platform::set_foreground_window(last_focused_window);
            debug_println!("focus previous window {last_focused_window:?} {_success}");
        }
    }
}

/// Handles both window size and position change side effects.
fn on_window_size_or_position_change(window: &Window, settings: &mut Settings) {
    settings.set_window_size(window);
    settings.set_window_position(window);
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
fn on_window_position_change(window: &Window, settings: &mut Settings) {
    settings.set_window_position(window);
}

/// Draws a crosshair image, or a simple red crosshair if no image is set. Normally this only
/// redraws the buffer if it's uninitialized, but redraw can be forced by setting the `force`
/// parameter to `true`.
fn draw_window(surface: &mut Surface<Rc<Window>, Rc<Window>>, settings: &Settings, force: bool) {
    let PhysicalSize { width: window_width, height: window_height } = settings.size();
    surface.resize(
        NonZeroU32::new(window_width).unwrap(),
        NonZeroU32::new(window_height).unwrap(),
    ).unwrap();

    let width = window_width as usize;
    let height = window_height as usize;

    let mut buffer = surface.buffer_mut().unwrap();

    if force || buffer.age() == 0 { // only redraw if the buffer is uninitialized OR redraw is being forced
        match settings.render_mode {
            RenderMode::Image => {
                // draw our image
                buffer.copy_from_slice(settings.image().unwrap().data.as_slice());
            }
            RenderMode::Crosshair => {
                // draw a generated crosshair

                const FULL_ALPHA: u32 = 0x00000000;

                if width <= 2 || height <= 2 {
                    // edge case where there simply aren't enough pixels to draw a crosshair, so we just fall back to a dot
                    buffer.fill(settings.color);
                } else {
                    // draw a simple crosshair. Think a `+` shape.
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
            RenderMode::ColorPicker => {
                image::draw_color_picker(&mut buffer);
            }
        }
    }

    buffer.present().unwrap();
}

/// Load a tray icon graphic.
fn get_icon() -> TrayIcon {
    // simply grab the static byte array that's embedded in the application, which was generated in build.rs
    TrayIcon::from_rgba(include_bytes!(env!("TRAY_ICON_PATH")).to_vec(), build_constants::TRAY_ICON_DIMENSION, build_constants::TRAY_ICON_DIMENSION).unwrap()
}

/// Initialize the window. This gives a transparent, borderless window that's always on top and can be clicked through.
fn init_window(event_loop: &EventLoop<()>, settings: &mut Settings) -> Window {
    let window_builder = WindowBuilder::new()
        .with_visible(false) // things get very buggy on Windows if you default the window to invisible...
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(false)
        .with_title("Simple Crosshair Overlay")
        .with_position(PhysicalPosition::new(0, 0)) // can't determine monitor size until the window is created, so just use some dummy values
        .with_inner_size(PhysicalSize::new(1, 1)) // this might flicker so make it very tiny
        .with_active(false);

    #[cfg(target_os = "windows")] let window_builder = {
        use winit::platform::windows::WindowBuilderExtWindows;
        window_builder
            .with_drag_and_drop(false)
            .with_skip_taskbar(true)
    };

    let window = window_builder.build(event_loop)
        .unwrap();

    // contrary to all my expectations this call appears to work reliably
    settings.set_window_position(&window);

    // this call is very fragile (read: shit) and sometimes simply doesn't do anything.
    // There's a fallback call up in the event loop that saves us when this fails.
    settings.set_window_size(&window);

    // once the window is ready, show it
    window.set_visible(true);

    // set these weirder settings AFTER the window is visible to avoid even more buggy Windows behavior
    // Windows particularly hates if you unset cursor_hittest while the window is hidden
    window.set_cursor_hittest(false).unwrap();
    window.set_window_level(WindowLevel::AlwaysOnTop);
    window.set_cursor_icon(CursorIcon::Crosshair); // Yo Dawg, I herd you like crosshairs so I put a crosshair in your crosshair so you can aim while you aim.

    window
}

/// Contains the menu items in our tray menu
#[derive(Clone)]
struct MenuItems {
    visible_button: CheckMenuItem,
    adjust_button: CheckMenuItem,
    color_pick_button: CheckMenuItem,
    image_pick_button: MenuItem,
    reset_button: MenuItem,
    about_button: MenuItem,
    exit_button: MenuItem,
}

impl Default for MenuItems {
    fn default() -> Self {
        let visible_button = CheckMenuItem::new("Visible", true, true, None);
        let adjust_button = CheckMenuItem::new("Adjust", true, false, None);
        let color_pick_button = CheckMenuItem::new("Pick Color", true, false, None);
        let image_pick_button = MenuItem::new("Load Image", true, None);
        let reset_button = MenuItem::new("Reset Overlay", true, None);
        let about_button = MenuItem::new("About", true, None);
        let exit_button = MenuItem::new("Exit", true, None);

        MenuItems {
            visible_button,
            adjust_button,
            color_pick_button,
            image_pick_button,
            reset_button,
            about_button,
            exit_button,
        }
    }
}

impl MenuItems {
    /// Append all the menu items into the provided `menu`.
    fn add_to_menu<T>(&self, menu: &T) where T: AppendableMenu {
        menu.append(&self.visible_button).unwrap();
        menu.append(&self.adjust_button).unwrap();
        menu.append(&self.color_pick_button).unwrap();
        menu.append(&self.image_pick_button).unwrap();
        menu.append(&self.reset_button).unwrap();
        menu.append(&self.about_button).unwrap();
        menu.append(&self.exit_button).unwrap();
    }
}

/// Surprisingly tray-icon doesn't provide a trait for the Menu.append() behavior several structs
/// have, so I have to build it myself for the structs I'm actually using.
trait AppendableMenu {
    /// Add a menu item to the end of this menu.
    fn append(&self, item: &dyn IsMenuItem) -> MenuResult<()>;
}

impl AppendableMenu for Menu {
    fn append(&self, item: &dyn IsMenuItem) -> MenuResult<()> {
        self.append(item)
    }
}

impl AppendableMenu for Submenu {
    fn append(&self, item: &dyn IsMenuItem) -> MenuResult<()> {
        self.append(item)
    }
}
