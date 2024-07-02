// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

#![windows_subsystem = "windows"] // necessary to remove the console window on Windows

use std::io;

use debug_print::debug_println;
use winit::event_loop::{DeviceEvents, EventLoop};
use winit::window::{CursorGrabMode, Window};

use simple_crosshair_overlay::platform;
use simple_crosshair_overlay::settings::CONFIG_PATH;
use simple_crosshair_overlay::settings::Settings;
use simple_crosshair_overlay::util::dialog;

mod window;
mod tray;

static ICON_TOOLTIP: &str = "Simple Crosshair Overlay";

/// constants generated in build.rs
mod build_constants {
    include!(env!("CONSTANTS_PATH"));
}

fn main() {
    // Initialize Eventloop before everything
    let event_loop: EventLoop<window::UserEvent> = EventLoop::new().unwrap();
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

    // only functional on Linux targets
    event_loop.listen_device_events(DeviceEvents::Never);

    // start sending tick events
    start_tick_sender(&settings, &event_loop);

    // create the winit application
    let mut window_state = window::State::new(settings, &event_loop);

    // pass control to the event loop
    event_loop.run_app(&mut window_state).unwrap();
}

fn start_tick_sender(settings: &Settings, event_loop: &EventLoop<window::UserEvent>) {
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
