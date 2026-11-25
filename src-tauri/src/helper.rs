#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

//! CEF sub-process entry point.
//!
//! On macOS every helper process is a separate app bundle
//! (`whatRust Helper*.app`) that re-executes this binary with the CEF
//! `--type=` switch. The framework must be loaded before `execute_process`.
//! On Windows/Linux the same binary handles sub-processes in `main.rs`,
//! but keeping this target lets one codebase build all platforms.

use cef::{args::Args, *};

fn main() {
    let args = Args::new();

    #[cfg(target_os = "macos")]
    let _loader = {
        // `true` = helper bundle: framework is three levels up from
        // `.../Helper.app/Contents/MacOS/<helper>`.
        let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), true);
        assert!(loader.load(), "failed to load Chromium Embedded Framework");
        loader
    };

    // Initialize the CEF API version table before any CEF call.
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    // A non-browser process never calls `initialize`.
    execute_process(
        Some(args.as_main_args()),
        None::<&mut App>,
        std::ptr::null_mut(),
    );
}
