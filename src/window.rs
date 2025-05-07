// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023-2024 Michael Ripley

use std::num::NonZeroU32;
use std::rc::Rc;

use debug_print::debug_println;
use tray_icon::dpi::{PhysicalPosition, PhysicalSize};
use tray_icon::menu::{MenuEvent, MenuEventReceiver};
use tray_icon::TrayIcon;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{CursorIcon, Window, WindowId, WindowLevel};

use simple_crosshair_overlay::private::platform;
use simple_crosshair_overlay::private::platform::HotkeyManager;
use simple_crosshair_overlay::private::settings::{RenderMode, Settings, CONFIG_PATH};
use simple_crosshair_overlay::private::util::dialog::DialogWorker;
use simple_crosshair_overlay::private::util::{dialog, image};

use crate::tray::MenuItems;
use crate::{build_constants, handle_color_pick, tray};

pub type UserEvent = ();
type Surface = softbuffer::Surface<Rc<Window>, Rc<Window>>;

pub struct State<'a> {
    context: Option<Context>,
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

/// Window context
struct Context {
    window: Rc<Window>,
    surface: Surface,
}

impl Context {
    fn new(active_event_loop: &ActiveEventLoop, settings: &mut Settings) -> Self {
        // unsafe note: these three structs MUST live and die together.
        // It is highly illegal to use the context or surface after the window is dropped.
        // The context only gets used right here, so that's fine.
        // As of this writing, none of these get moved out of this struct. Therefore, they all get dropped at the same time, which is safe.
        let window = Rc::new(init_window(active_event_loop, settings));
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface: Surface = Surface::new(&context, window.clone()).unwrap();
        Context { window, surface }
    }
}

impl<'a> State<'a> {
    pub fn new(settings: Settings, _event_loop: &EventLoop<UserEvent>) -> Self {
        // HotkeyManager has a decent quantity of data in it, but again it never really gets moved so we can just leave it on the stack
        let hotkey_manager: HotkeyManager = HotkeyManager::new(&settings.persisted.key_bindings)
            .unwrap_or_else(|e| {
                dialog::show_warning(format!("{e}\n\nUsing default hotkeys."));
                HotkeyManager::default()
            });

        let (menu_items, tray_icon) = tray::build_tray_icon();
        State {
            context: None,
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
        let window: &Window = &self.context.as_ref().unwrap().window;

        if let Ok(path) = self.dialog_worker.try_recv_file_path() {
            self.menu_items.image_pick_button.set_enabled(true);

            if let Some(path) = path {
                match self.settings.load_png(path) {
                    Ok(()) => {
                        self.force_redraw = true;
                        self.window_scale_dirty = true;
                    }
                    Err(e) => dialog::show_warning(format!("Error loading PNG.\n\n{}", e)),
                }
            }
        }

        while let Ok(event) = self.menu_channel.try_recv() {
            match event.id {
                id if id == self.menu_items.exit_button.id() => {
                    // drop the tray icon, solving the funny Windows issue where it lingers after application close
                    #[cfg(not(target_os = "linux"))]
                    self.tray_icon.take();
                    window.set_visible(false);
                    if let Err(e) = self.settings.save() {
                        dialog::show_warning(format!(
                            "Error saving settings to \"{}\".\n\n{}",
                            CONFIG_PATH.display(),
                            e
                        ));
                    }

                    // kill the dialog worker and wait for it to finish
                    // this makes the application remain open until the user has clicked through any queued dialogs
                    self.dialog_worker
                        .shutdown()
                        .expect("failed to shut down dialog worker");

                    active_event_loop.exit();
                    break;
                }
                id if id == self.menu_items.visible_button.id() => {
                    window.set_visible(self.menu_items.visible_button.is_checked());
                }
                id if id == self.menu_items.reset_button.id() => {
                    self.settings.reset();
                    self.force_redraw = true;
                    self.window_scale_dirty = true;
                }
                id if id == self.menu_items.color_pick_button.id() => {
                    let pick_color = self.menu_items.color_pick_button.is_checked();
                    self.settings.set_pick_color(pick_color);
                    handle_color_pick(pick_color, window, &mut self.last_focused_window, false);
                    self.window_scale_dirty = true;
                }
                id if id == self.menu_items.image_pick_button.id() => {
                    self.menu_items.image_pick_button.set_enabled(false);
                    dialog::request_png();
                }
                id if id == self.menu_items.about_button.id() => {
                    dialog::show_info(format!(
                        "{}\nversion {} {}",
                        build_constants::APPLICATION_NAME,
                        env!("CARGO_PKG_VERSION"),
                        env!("GIT_COMMIT_HASH")
                    ));
                }
                _ => (),
            }
        }

        if self.window_scale_dirty {
            on_window_size_or_position_change(window, &mut self.settings);
            self.window_scale_dirty = false;
            self.window_position_dirty = false;
        } else if self.window_position_dirty {
            on_window_position_change(window, &mut self.settings);
            self.window_position_dirty = false;
        }
    }
}

impl<'a> ApplicationHandler<UserEvent> for State<'a> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            self.context = Some(Context::new(event_loop, &mut self.settings))
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // only used on iOS/Android/Web
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: UserEvent) {
        let window: &Window = &self.context.as_ref().unwrap().window;

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
                let next_monitor = (self.settings.monitor_index + 1) % window.available_monitors().count();
                self.settings.set_monitor(next_monitor);
                self.window_scale_dirty = true;
            }

            if self.settings.is_scalable() && self.hotkey_manager.scale_increase() != 0 {
                self.settings.persisted.window_height += self.hotkey_manager.scale_increase();
                self.settings.persisted.window_width = self.settings.persisted.window_height;
                self.window_scale_dirty = true;
            }

            if self.settings.is_scalable() && self.hotkey_manager.scale_decrease() != 0 {
                self.settings.persisted.window_height = self
                    .settings
                    .persisted
                    .window_height
                    .checked_sub(self.hotkey_manager.scale_decrease())
                    .unwrap_or(1)
                    .max(1);
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
            window.set_visible(self.window_visible);
            if !self.window_visible {
                self.menu_items.adjust_button.set_checked(false)
            }
        }

        // only enable this hotkey if the color picker is already visible OR if adjust mode is on
        if self.hotkey_manager.toggle_color_picker()
            && (adjust_mode || self.settings.get_pick_color())
        {
            let color_pick = self.settings.toggle_pick_color();
            self.menu_items.color_pick_button.set_checked(color_pick);
            handle_color_pick(color_pick, window, &mut self.last_focused_window, true);
            self.window_scale_dirty = true;
        }

        self.post_event_work(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let context: &mut Context = self.context.as_mut().unwrap();

        match event {
            WindowEvent::RedrawRequested => {
                // failsafe to resize the window before a redraw if necessary
                // ...and of course it's fucking necessary
                self.settings
                    .validate_window_size(&context.window, context.window.inner_size());
                draw_window(&mut context.surface, &self.settings, self.force_redraw);
                self.force_redraw = false;
            }
            WindowEvent::Moved(position) => {
                // incredibly, if the taskbar is at the top or left of the screen Windows will
                // (un)helpfully shift the window over by the taskbar's size. I have no idea why
                // this happens and it's terrible, but luckily Windows tells me it's done this so
                // that I can immediately detect and undo it.
                debug_println!("window position changed to {:?}", position);
                self.settings
                    .validate_window_position(&context.window, position);
            }
            WindowEvent::Resized(size) => {
                // See above nightmare scenario with the window position. I figure I might as well
                // do the same thing for size just in case Windows also has some arcane, evil
                // involuntary resizing behavior.
                debug_println!("window size changed to {:?}", size);
                self.settings.validate_window_size(&context.window, size);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.last_mouse_position = position;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let PhysicalPosition { x, y } = self.last_mouse_position;
                let x = x as usize;
                let y = y as usize;

                let PhysicalSize { width, height } = self.settings.size();
                let width = width as usize;
                let height = height as usize;

                self.settings
                    .set_color(image::hue_alpha_color_from_coordinates(x, y, width, height));
                self.menu_items.color_pick_button.set_checked(false);
                handle_color_pick(false, &context.window, &mut self.last_focused_window, false);
                self.window_scale_dirty = true;
            }
            _ => {}
        }

        self.post_event_work(event_loop);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        _event: DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // only used on iOS/Android/Web
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {}

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {
        // only used on iOS/Android
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
fn draw_window(surface: &mut Surface, settings: &Settings, force: bool) {
    let PhysicalSize {
        width: window_width,
        height: window_height,
    } = settings.size();
    surface
        .resize(
            NonZeroU32::new(window_width).unwrap(),
            NonZeroU32::new(window_height).unwrap(),
        )
        .unwrap();

    let width = window_width as usize;
    let height = window_height as usize;

    let mut buffer = surface.buffer_mut().unwrap();

    if force || buffer.age() == 0 {
        // only redraw if the buffer is uninitialized OR redraw is being forced
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

/// Initialize the window. This gives a transparent, borderless window that's always on top and can be clicked through.
fn init_window(active_event_loop: &ActiveEventLoop, settings: &mut Settings) -> Window {
    let window_attributes = Window::default_attributes()
        .with_visible(false) // things get very buggy on Windows if you default the window to invisible...
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(false)
        .with_title("Simple Crosshair Overlay")
        .with_position(PhysicalPosition::new(0, 0)) // can't determine monitor size until the window is created, so just use some dummy values
        .with_inner_size(PhysicalSize::new(1, 1)) // this might flicker so make it very tiny
        .with_active(false);

    #[cfg(target_os = "windows")]
    let window_attributes = {
        use winit::platform::windows::WindowAttributesExtWindows;
        window_attributes
            .with_drag_and_drop(false)
            .with_skip_taskbar(true)
    };

    #[cfg(target_os = "macos")]
    let window_attributes = {
        use winit::platform::macos::WindowAttributesExtMacOS;
        window_attributes
            .with_title_hidden(true)
            .with_titlebar_hidden(true)
            .with_has_shadow(false)
    };

    let window = active_event_loop.create_window(window_attributes).unwrap();

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
