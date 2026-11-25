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

use cef::{args::Args, execute_process, CefApp, MainArgs};

fn main() {
    let _ = paths::ensure_dirs();

    let args = Args::new();
    let main_args = MainArgs::new(&args);

    let process_app = CefApp::new(browser::ProcessApp::new());
    let exit_code = execute_process(Some(main_args.as_ref()), Some(&process_app), std::ptr::null_mut());
    if exit_code >= 0 {
        std::process::exit(exit_code);
    }

    let cache = paths::cef_cache_dir();
    let cache_str = cache.to_string_lossy().to_string();
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(|| std::path::PathBuf::from("."));
    let res = exe_dir.to_string_lossy().to_string();
    let locales = exe_dir.join("locales").to_string_lossy().to_string();
    let cef_settings = cef::Settings {
        no_sandbox: !cfg!(feature = "sandbox") as _,
        windowless_rendering_enabled: 0,
        external_message_pump: 0,
        multi_threaded_message_loop: 0,
        log_severity: cef::LogSeverity::WARNING,
        root_cache_path: cache_str.as_str().into(),
        cache_path: cache_str.as_str().into(),
        resources_dir_path: res.as_str().into(),
        locales_dir_path: locales.as_str().into(),
        persist_session_cookies: 1,
        ..Default::default()
    };

    let mut whatrust = app::WhatRustApp::new();
    let browser_app = CefApp::new(browser::BrowserApp::new(app::state()));
    whatrust.run(main_args, cef_settings, browser_app);
}
