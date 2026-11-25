use tray_icon::{menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem}, TrayIcon, TrayIconBuilder, Icon};

pub struct Tray {
    pub tray: TrayIcon,
    pub show_item: MenuItem,
    pub reload_item: MenuItem,
    pub quit_item: MenuItem,
}

pub enum TrayCommand {
    Show,
    Reload,
    Quit,
}

impl Tray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let show_item = MenuItem::new("Show / Hide", true, None);
        let reload_item = MenuItem::new("Reload", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        menu.append(&show_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&reload_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        let (rgba, w, h) = crate::icons::load_rgba(crate::icons::tray_icon(false)).ok_or("icon")?;
        let icon = Icon::from_rgba(rgba, w, h)?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("whatRust")
            .with_icon(icon)
            .with_icon_as_template(cfg!(target_os = "macos"))
            .build()?;

        Ok(Self { tray, show_item, reload_item, quit_item })
    }

    pub fn set_unread(&self, unread: bool) {
        if let Some((rgba, w, h)) = crate::icons::load_rgba(crate::icons::tray_icon(unread)) {
            if let Ok(icon) = Icon::from_rgba(rgba, w, h) {
                let _ = self.tray.set_icon(Some(icon));
                let _ = self.tray.set_icon_as_template(cfg!(target_os = "macos"));
            }
        }
        let _ = self.tray.set_tooltip(Some(if unread { "whatRust - unread messages" } else { "whatRust" }));
    }

    pub fn poll_event(&self) -> Option<TrayCommand> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_item.id() { return Some(TrayCommand::Show); }
            if event.id == self.reload_item.id() { return Some(TrayCommand::Reload); }
            if event.id == self.quit_item.id() { return Some(TrayCommand::Quit); }
        }
        None
    }
}
