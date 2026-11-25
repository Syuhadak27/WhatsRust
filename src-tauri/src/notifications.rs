use crate::config::Settings;

pub fn show(title: &str, body: &str) {
    let settings = Settings::load();
    if !settings.notifications {
        return;
    }
    let mut n = notify_rust::Notification::new();
    n.summary(title).body(body).appname("whatRust");
    #[cfg(not(target_os = "macos"))]
    n.icon("whatrust");
    let _ = n.show();
}
