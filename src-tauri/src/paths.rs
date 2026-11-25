use std::path::PathBuf;

pub fn app_data_dir() -> PathBuf {
    let base = dirs::data_local_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from("."));
    base.join("whatRust")
}

pub fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("whatRust")
}

pub fn cache_dir() -> PathBuf { app_data_dir().join("cache") }
pub fn cef_cache_dir() -> PathBuf { app_data_dir().join("cef") }
pub fn profiles_dir() -> PathBuf { app_data_dir().join("profiles") }
pub fn settings_file() -> PathBuf { config_dir().join("settings.json") }
pub fn window_state_file() -> PathBuf { config_dir().join("window-state.json") }

pub fn ensure_dirs() -> std::io::Result<()> {
    for d in [config_dir(), app_data_dir(), cache_dir(), cef_cache_dir(), profiles_dir()] {
        std::fs::create_dir_all(d)?;
    }
    Ok(())
}
