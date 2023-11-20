use tray_icon::menu::MenuId;

use crate::MenuItems;

pub trait MenuItemWithId {
    fn id(&self) -> &MenuId;
}

#[derive(Clone)]
pub struct LinuxMenuItem {
    id: MenuId,
}

impl LinuxCheckMenuItem {
    pub fn new(id: &MenuId) -> LinuxMenuItem {
        LinuxMenuItem {
            id: id.to_owned(),
        }
    }
}

impl MenuItemWithId for LinuxMenuItem {
    fn id(&self) -> &MenuId {
        &self.id
    }
}

pub trait MenuItemWithCheckbox {
    fn is_checked(&self) -> bool;
    fn set_checked(&mut self, checked: bool);
    fn set_enabled(&mut self, enabled: bool);
    fn is_dirty(&self) -> bool;
    fn reset_dirty(&mut self);
}

#[derive(Clone)]
pub struct LinuxCheckMenuItem {
    id: MenuId,
    enabled: bool,
    checked: bool,
    dirty: bool,
}

impl LinuxCheckMenuItem {
    pub fn new(id: &MenuId, checked: bool) -> LinuxCheckMenuItem {
        LinuxCheckMenuItem {
            id: id.to_owned(),
            enabled: true,
            checked,
            dirty: false,
        }
    }
}

impl MenuItemWithId for LinuxCheckMenuItem {
    fn id(&self) -> &MenuId {
        &self.id
    }
}

impl MenuItemWithCheckbox for LinuxCheckMenuItem {
    fn is_checked(&self) -> bool {
        self.checked
    }

    fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
        self.dirty = true;
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn reset_dirty(&mut self) {
        self.dirty = false;
    }
}

#[derive(Clone)]
struct LinuxMenuItems {
    visible_button: LinuxCheckMenuItem,
    adjust_button: LinuxCheckMenuItem,
    color_pick_button: LinuxCheckMenuItem,
    image_pick_button: LinuxMenuItem,
    reset_button: LinuxMenuItem,
    about_button: LinuxMenuItem,
    exit_button: LinuxMenuItem,
}

impl LinuxMenuItems {
    fn new(menu_items: &MenuItems) -> Self {
        LinuxMenuItems {
            visible_button: LinuxCheckMenuItem::new(menu_items.visible_button.id(), menu_items.visible_button.is_checked()),
            adjust_button: LinuxCheckMenuItem::new(menu_items.adjust_button.id(), menu_items.adjust_button.is_checked()),
            color_pick_button: LinuxCheckMenuItem::new(menu_items.adjust_button.id(), menu_items.adjust_button.is_checked()),
            image_pick_button: LinuxMenuItem::new(menu_items.image_pick_button.id()),
            reset_button: LinuxMenuItem::new(menu_items.reset_button.id()),
            about_button: LinuxMenuItem::new(menu_items.about_button.id()),
            exit_button: LinuxMenuItem::new(menu_items.exit_button.id()),
        }
    }
}
