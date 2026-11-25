use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub notifications: bool,
    pub notification_sound: bool,
    pub hide_app_banner: bool,
    pub page_overlays: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_minimized: false,
            notifications: true,
            notification_sound: true,
            hide_app_banner: true,
            page_overlays: true,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = crate::paths::settings_file();
        std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = crate::paths::settings_file().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = crate::paths::settings_file().with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap_or_default())?;
        std::fs::rename(tmp, crate::paths::settings_file())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WindowState {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl WindowState {
    pub fn load() -> Self {
        let path = crate::paths::window_state_file();
        std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(Self { width: 1200, height: 800, ..Default::default() })
    }
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = crate::paths::window_state_file().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = crate::paths::window_state_file().with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap_or_default())?;
        std::fs::rename(tmp, crate::paths::window_state_file())
    }
}
