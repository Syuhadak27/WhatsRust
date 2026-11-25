#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod bridge;
mod browser;
mod config;
mod icons;
mod notifications;
mod paths;
mod tray;
mod window_state;
#[cfg(target_os = "macos")]
mod mac;

use cef::*;

fn main() {
    let _ = paths::ensure_dirs();
    let _library = browser::load_cef();

    let args = args::Args::new();
    let Some(cmd_line) = args.as_cmd_line() else {
        eprintln!("Failed to parse command line arguments");
        std::process::exit(1);
    };

    let switch = CefString::from("type");
    let is_browser_process = cmd_line.has_switch(Some(&switch)) != 1;
    let ret = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
    if is_browser_process {
        assert_eq!(ret, -1, "cannot execute browser process");
    } else {
        assert!(ret >= 0, "cannot execute non-browser process");
        return;
    }

    let cache = paths::cef_cache_dir();
    let cache_str = cache.to_string_lossy().to_string();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let res = exe_dir.to_string_lossy().to_string();
    let locales = exe_dir.join("locales").to_string_lossy().to_string();

    let mut settings = Settings {
        no_sandbox: !cfg!(feature = "sandbox") as _,
        windowless_rendering_enabled: 0,
        external_message_pump: 0,
        multi_threaded_message_loop: 0,
        log_severity: LogSeverity::WARNING,
        root_cache_path: cache_str.as_str().into(),
        cache_path: cache_str.as_str().into(),
        persist_session_cookies: 1,
        ..Default::default()
    };

    // macOS resolves resources/locales from inside
    // `Chromium Embedded Framework.framework/Resources` automatically and
    // finds the helper apps in `Contents/Frameworks`. Pointing these at
    // `Contents/MacOS` would prevent CEF from starting.
    #[cfg(not(target_os = "macos"))]
    {
        settings.resources_dir_path = res.as_str().into();
        settings.locales_dir_path = locales.as_str().into();
    }
    #[cfg(target_os = "macos")]
    let _ = (res, locales);

    let mut app = browser::BrowserApp::new(app::state());
    assert_eq!(
        initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            std::ptr::null_mut(),
        ),
        1
    );

    let mut whatrust = app::WhatRustApp::new();
    whatrust.run();

    shutdown();
}
