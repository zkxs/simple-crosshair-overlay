// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

#![windows_subsystem = "windows"] // necessary to remove the console window on Windows

use std::io;
use std::num::NonZeroU32;
use std::rc::Rc;

use debug_print::debug_println;
use softbuffer::{Context, Surface};
use tray_icon::{menu::Menu, TrayIcon, TrayIconBuilder};
use tray_icon::menu::{CheckMenuItem, IsMenuItem, MenuEvent, MenuEventReceiver, MenuItem, Result as MenuResult, Submenu};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, DeviceEvents, EventLoop};
use winit::window::{CursorGrabMode, CursorIcon, Window, WindowId, WindowLevel};

use simple_crosshair_overlay::platform;
use simple_crosshair_overlay::platform::HotkeyManager;
use simple_crosshair_overlay::settings::{RenderMode, Settings};
use simple_crosshair_overlay::settings::CONFIG_PATH;
use simple_crosshair_overlay::util::dialog;
use simple_crosshair_overlay::util::dialog::DialogWorker;
use simple_crosshair_overlay::util::image;

#[cfg(target_os = "linux")]
mod linux;

static ICON_TOOLTIP: &str = "Simple Crosshair Overlay";

/// constants generated in build.rs
mod build_constants {
    include!(env!("CONSTANTS_PATH"));
}

fn main() {
    // Initialize Eventloop before everything
    let event_loop: EventLoop<UserEvent> = EventLoop::new().unwrap();
    // in theory Wait is now the default ControlFlow, so the following isn't needed:
    // event_loop.set_control_flow(ControlFlow::Wait);

    // settings has a decent quantity of data in it, but it never really gets moved so we can just leave it on the stack
    // the image buffer is internally boxed so don't worry about that
    let settings = match Settings::load() {
        Ok(settings) => settings,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Settings::default(), // generate new settings file when it doesn't exist
        Err(e) => {
            dialog::show_warning(format!("Error loading settings file \"{}\". Resetting to default settings.\n\n{}", CONFIG_PATH.display(), e));
            Settings::default()
        }
    };

    //TODO: is this needed?
    event_loop.listen_device_events(DeviceEvents::Always);

    // start sending tick events
    start_tick_sender(&settings, &event_loop);

    // create the winit application
    let mut window_state = WindowState::new(settings, &event_loop);

    // pass control to the event loop
    event_loop.run_app(&mut window_state).unwrap();
}

type UserEvent = ();

struct WindowState<'a> {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    settings: Settings,
    hotkey_manager: HotkeyManager,
    /// native dialogs block a thread, so we'll spin up a single thread to loop through queued dialogs.
    /// If we ever need to show multiple dialogs, they just get queued.
    dialog_worker: DialogWorker,
    /// we keep the tray icon in an Option so that we can take() it later to drop
    tray_icon: Option<TrayIcon>,
    menu_items: MenuItems,
    last_focused_window: Option<platform::WindowHandle>,
    last_mouse_position: PhysicalPosition<f64>,
    menu_channel: &'a MenuEventReceiver,
    /// if set to true, the next redraw will be forced even for known buffer contents
    force_redraw: bool,
    window_position_dirty: bool,
    window_scale_dirty: bool,
    window_visible: bool,
}

impl <'a> WindowState<'a> {
    fn new(mut settings: Settings, event_loop: &EventLoop<UserEvent>) -> Self {
        // HotkeyManager has a decent quantity of data in it, but again it never really gets moved so we can just leave it on the stack
        let hotkey_manager: HotkeyManager = HotkeyManager::new(&settings.persisted.key_bindings).unwrap_or_else(|e| {
            dialog::show_warning(format!("{e}\n\nUsing default hotkeys."));
            HotkeyManager::default()
        });

        let (menu_items, tray_icon) = build_tray_icon();

        // unsafe note: these three structs MUST live and die together.
        // It is highly illegal to use the context or surface after the window is dropped.
        // The context only gets used right here, so that's fine.
        // As of this writing, none of these get moved. Therefore they all get dropped one after the other at the end of main(), which is safe.
        let window = Rc::new(init_window(event_loop, &mut settings));
        let context = Context::new(window.clone()).unwrap();
        let surface: Surface<Rc<Window>, Rc<Window>> = Surface::new(&context, window.clone()).unwrap();

        WindowState {
            window,
            surface,
            settings,
            hotkey_manager,
            dialog_worker: dialog::spawn_worker(),
            tray_icon: Some(tray_icon),
            menu_items,
            last_focused_window: None,
            last_mouse_position: Default::default(),
            menu_channel: MenuEvent::receiver(),
            force_redraw: false,
            window_position_dirty: false,
            window_scale_dirty: false,
            window_visible: true,
        }
    }

    fn post_event_work(&mut self, active_event_loop: &ActiveEventLoop) {
        if let Ok(path) = self.dialog_worker.try_recv_file_path() {
            self.menu_items.image_pick_button.set_enabled(true);

            if let Some(path) = path {
                match self.settings.load_png(path) {
                    Ok(()) => {
                        self.force_redraw = true;
                        self.window_scale_dirty = true;
                    }
                    Err(e) => dialog::show_warning(format!("Error loading PNG.\n\n{}", e))
                }
            }
        }

        while let Ok(event) = self.menu_channel.try_recv() {
            match event.id {
                id if id == self.menu_items.exit_button.id() => {
                    // drop the tray icon, solving the funny Windows issue where it lingers after application close
                    #[cfg(not(target_os = "linux"))] self.tray_icon.take();
                    self. window.set_visible(false);
                    if let Err(e) = self.settings.save() {
                        dialog::show_warning(format!("Error saving settings to \"{}\".\n\n{}", CONFIG_PATH.display(), e));
                    }

                    // kill the dialog worker and wait for it to finish
                    // this makes the application remain open until the user has clicked through any queued dialogs
                    self.dialog_worker.shutdown().expect("failed to shut down dialog worker");

                    active_event_loop.exit();
                    break;
                }
                id if id == self.menu_items.visible_button.id() => {
                    self.window.set_visible(self.menu_items.visible_button.is_checked());
                }
                id if id == self.menu_items.reset_button.id() => {
                    self.settings.reset();
                    self.force_redraw = true;
                    self.window_scale_dirty = true;
                }
                id if id == self.menu_items.color_pick_button.id() => {
                    let pick_color = self.menu_items.color_pick_button.is_checked();
                    self.settings.set_pick_color(pick_color);
                    handle_color_pick(pick_color, &self.window, &mut self.last_focused_window, false);
                    self.window_scale_dirty = true;
                }
                id if id == self.menu_items.image_pick_button.id() => {
                    self.menu_items.image_pick_button.set_enabled(false);
                    dialog::request_png();
                }
                id if id == self.menu_items.about_button.id() => {
                    dialog::show_info(format!("{}\nversion {} {}", build_constants::APPLICATION_NAME, env!("CARGO_PKG_VERSION"), env!("GIT_COMMIT_HASH")));
                }
                _ => (),
            }
        }

        if self.window_scale_dirty {
            on_window_size_or_position_change(&self.window, &mut self.settings);
            self.window_scale_dirty = false;
            self.window_position_dirty = false;
        } else if self.window_position_dirty {
            on_window_position_change(&self.window, &mut self.settings);
            self.window_position_dirty = false;
        }
    }
}

impl <'a> ApplicationHandler<UserEvent> for WindowState<'a> {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: UserEvent) {
        self.hotkey_manager.poll_keys();
        self.hotkey_manager.process_keys();

        let adjust_mode = self.menu_items.adjust_button.is_checked();
        if adjust_mode {
            if self.hotkey_manager.move_up() != 0 {
                self.settings.persisted.window_dy -= self.hotkey_manager.move_up() as i32;
                self.window_position_dirty = true;
            }

            if self.hotkey_manager.move_down() != 0 {
                self.settings.persisted.window_dy += self.hotkey_manager.move_down() as i32;
                self.window_position_dirty = true;
            }

            if self.hotkey_manager.move_left() != 0 {
                self.settings.persisted.window_dx -= self.hotkey_manager.move_left() as i32;
                self.window_position_dirty = true;
            }

            if self.hotkey_manager.move_right() != 0 {
                self.settings.persisted.window_dx += self.hotkey_manager.move_right() as i32;
                self.window_position_dirty = true;
            }

            if self.hotkey_manager.cycle_monitor() {
                self.settings.monitor_index = (self.settings.monitor_index + 1) % self.window.available_monitors().count();
                self.window_scale_dirty = true;
            }

            if self.settings.is_scalable() && self.hotkey_manager.scale_increase() != 0 {
                self.settings.persisted.window_height += self.hotkey_manager.scale_increase();
                self.settings.persisted.window_width = self.settings.persisted.window_height;
                self.window_scale_dirty = true;
            }

            if self.settings.is_scalable() && self.hotkey_manager.scale_decrease() != 0 {
                self.settings.persisted.window_height = self.settings.persisted.window_height.checked_sub(self.hotkey_manager.scale_decrease()).unwrap_or(1).max(1);
                self.settings.persisted.window_width = self.settings.persisted.window_height;
                self.window_scale_dirty = true;
            }

            // adjust button is already checked
            if self.hotkey_manager.toggle_adjust() {
                self.menu_items.adjust_button.set_checked(false)
            }
        } else if self.hotkey_manager.toggle_adjust() {
            // adjust button is NOT checked
            self.menu_items.adjust_button.set_checked(true)
        }

        if self.hotkey_manager.toggle_hidden() {
            self.window_visible = !self.window_visible;
            self.window.set_visible(self.window_visible);
            if !self.window_visible {
                self.menu_items.adjust_button.set_checked(false)
            }
        }

        // only enable this hotkey if the color picker is already visible OR if adjust mode is on
        if self.hotkey_manager.toggle_color_picker() && (adjust_mode || self.settings.get_pick_color()) {
            let color_pick = self.settings.toggle_pick_color();
            self.menu_items.color_pick_button.set_checked(color_pick);
            handle_color_pick(color_pick, &self.window, &mut self.last_focused_window, true);
            self.window_scale_dirty = true;
        }

        self.post_event_work(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                // failsafe to resize the window before a redraw if necessary
                // ...and of course it's fucking necessary
                self.settings.validate_window_size(&self.window, self.window.inner_size());
                draw_window(&mut self.surface, &self.settings, self.force_redraw);
                self.force_redraw = false;
            }
            WindowEvent::Moved(position) => {
                // incredibly, if the taskbar is at the top or left of the screen Windows will
                // (un)helpfully shift the window over by the taskbar's size. I have no idea why
                // this happens and it's terrible, but luckily Windows tells me it's done this so
                // that I can immediately detect and undo it.
                debug_println!("window position changed to {:?}", position);
                self.settings.validate_window_position(&self.window, position);
            }
            WindowEvent::Resized(size) => {
                // See above nightmare scenario with the window position. I figure I might as well
                // do the same thing for size just in case Windows also has some arcane, evil
                // involuntary resizing behavior.
                debug_println!("window size changed to {:?}", size);
                self.settings.validate_window_size(&self.window, size);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.last_mouse_position = position;
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let PhysicalPosition { x, y } = self.last_mouse_position;
                let x = x as usize;
                let y = y as usize;

                let PhysicalSize { width, height } = self.settings.size();
                let width = width as usize;
                let height = height as usize;

                self.settings.set_color(image::hue_alpha_color_from_coordinates(x, y, width, height));
                self.menu_items.color_pick_button.set_checked(false);
                handle_color_pick(false, &self.window, &mut self.last_focused_window, false);
                self.window_scale_dirty = true;
            }
            _ => {}
        }

        self.post_event_work(event_loop);
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, _event: DeviceEvent) {
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
    }

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {
    }
}

fn build_tray_icon() -> (MenuItems, TrayIcon) {
    // on linux we have to do this in a completely different way
    #[cfg(not(target_os = "linux"))] let tray_menu = Menu::new();

    let menu_items = MenuItems::default();

    // windows: do not use a submenu
    #[cfg(target_os = "windows")] {
        menu_items.add_to_menu(&tray_menu);
    }

    // mac: there are special submenu requirements
    #[cfg(target_os = "macos")] {
        // on mac all menu items must be in a submenu, so just make one with no name. Hope that doesn't cause problems...
        let submenu = tray_icon::menu::Submenu::new("", true);
        tray_menu.append(&submenu).unwrap();
        menu_items.add_to_menu(&submenu);
    }

    // on Linux this MUST be called on the GTK thread, so we have to do some weird hijinks to pass things around
    #[cfg(not(target_os = "linux"))] let tray_icon: TrayIcon = {
        let tray_icon_builder = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(ICON_TOOLTIP)
            .with_icon(get_icon());
        tray_icon_builder.build().unwrap()
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
                debug_println!("starting GTK background thread");
                gtk::init().unwrap();
                debug_println!("GTK init complete");

                // initialize the tray icon
                let tray_menu = Menu::new();
                menu_items.add_to_menu(&tray_menu);

                let tray_icon_builder = TrayIconBuilder::new()
                    .with_menu(Box::new(tray_menu))
                    .with_tooltip(ICON_TOOLTIP)
                    .with_icon(get_icon());
                let mut tray_icon = Some(tray_icon_builder.build().unwrap());

                // signal that GTK init is complete
                {
                    let (lock, condvar) = &*condvar_pair_clone;
                    let mut gtk_started = lock.lock().unwrap();
                    *gtk_started = true;
                    condvar.notify_one();
                } // this block is actually necessary so that the lock gets released!

                debug_println!("GTK init signal sent. Starting GTK main loop.");
                loop {
                    gtk::main_iteration_do(false);
                    //TODO: channel MenuItem state around?
                    std::thread::yield_now();
                }
                debug_println!("GTK main loop returned!? Weird.");
            }).unwrap();
        debug_println!("spawned GTK background thread");

        // wait for GTK to init
        let (lock, condvar) = &*condvar_pair;
        let gtk_started = lock.lock().unwrap();
        debug_println!("acquired GTK lock");
        if !*gtk_started {
            debug_println!("waiting for GTK init signal");
            let (gtk_started, timeout_result) = condvar.wait_timeout(gtk_started, Duration::from_secs(5)).unwrap();
            if !*gtk_started {
                panic!("GTK startup timed out = {}", timeout_result.timed_out());
            }
        }

        debug_println!("GTK startup complete");
    }

    (menu_items, tray_icon)
}

fn start_tick_sender(settings: &Settings, event_loop: &EventLoop<UserEvent>) {
    let user_event_sender = event_loop.create_proxy();
    let key_process_interval = settings.tick_interval;
    std::thread::Builder::new()
        .name("tick-sender".to_string())
        .spawn(move || {
            loop {
                let _ = user_event_sender.send_event(());
                std::thread::sleep(key_process_interval);
            }
        }).unwrap(); // if we fail to spawn a thread something is super wrong and we ought to panic
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
        window.set_cursor_hittest(true).unwrap(); // fails on non Windows/Mac/Linux platforms
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
fn get_icon() -> tray_icon::Icon {
    // simply grab the static byte array that's embedded in the application, which was generated in build.rs
    tray_icon::Icon::from_rgba(include_bytes!(env!("TRAY_ICON_PATH")).to_vec(), build_constants::TRAY_ICON_DIMENSION, build_constants::TRAY_ICON_DIMENSION).unwrap()
}

/// Initialize the window. This gives a transparent, borderless window that's always on top and can be clicked through.
fn init_window(event_loop: &EventLoop<()>, settings: &mut Settings) -> Window {
    let window_attributes = Window::default_attributes()
        .with_visible(false) // things get very buggy on Windows if you default the window to invisible...
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(false)
        .with_title("Simple Crosshair Overlay")
        .with_position(PhysicalPosition::new(0, 0)) // can't determine monitor size until the window is created, so just use some dummy values
        .with_inner_size(PhysicalSize::new(1, 1)) // this might flicker so make it very tiny
        .with_active(false);

    #[cfg(target_os = "windows")] let window_attributes = {
        use winit::platform::windows::WindowAttributesExtWindows;
        window_attributes
            .with_drag_and_drop(false)
            .with_skip_taskbar(true)
    };

    let window = event_loop.create_window(window_attributes)
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
    window.set_cursor(CursorIcon::Crosshair); // Yo Dawg, I herd you like crosshairs so I put a crosshair in your crosshair so you can aim while you aim.

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
