// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023-2024 Michael Ripley

#[cfg(target_os = "linux")]
use debug_print::debug_println;
use tray_icon::{menu::Menu, TrayIcon, TrayIconBuilder};
use tray_icon::menu::{CheckMenuItem, IsMenuItem, MenuItem, Result as MenuResult, Submenu};

use crate::{build_constants, ICON_TOOLTIP};

pub fn build_tray_icon() -> (MenuItems, TrayIcon) {
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

/// Load a tray icon graphic.
fn get_icon() -> tray_icon::Icon {
    // simply grab the static byte array that's embedded in the application, which was generated in build.rs
    tray_icon::Icon::from_rgba(include_bytes!(env!("TRAY_ICON_PATH")).to_vec(), build_constants::TRAY_ICON_DIMENSION, build_constants::TRAY_ICON_DIMENSION).unwrap()
}

/// Contains the menu items in our tray menu
#[derive(Clone)]
pub struct MenuItems {
    pub visible_button: CheckMenuItem,
    pub adjust_button: CheckMenuItem,
    pub color_pick_button: CheckMenuItem,
    pub image_pick_button: MenuItem,
    pub reset_button: MenuItem,
    pub about_button: MenuItem,
    pub exit_button: MenuItem,
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
    fn add_to_menu<T>(&self, menu: &T)
    where
        T: AppendableMenu,
    {
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
