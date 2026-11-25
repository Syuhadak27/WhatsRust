#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "macos")]
    {
        use cef::args::Args;
        use cef::{execute_process, App, CefApp, MainArgs, sys, rc::{Rc, RcImpl}, wrap_app, ImplApp};

        #[derive(Clone)]
        struct HelperApp { obj: std::ptr::NonNull<sys::_cef_app_t> }
        impl HelperApp {
            fn new() -> Self { Self { obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_app!(Self {})))) } } }
        }
        impl Rc for HelperApp { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
        impl ImplApp for HelperApp { fn get_raw(&self) -> *mut sys::_cef_app_t { self.obj.as_ptr() } }

        let args = Args::new();
        let main_args = MainArgs::new(&args);
        let mut app = CefApp::new(HelperApp::new());
        let code = execute_process(Some(main_args.as_ref()), Some(&app), std::ptr::null_mut());
        std::process::exit(code);
    }
    #[cfg(not(target_os = "macos"))]
    {
        // On Windows/Linux the main binary handles sub-processes via execute_process.
        println!("whatrust_helper is used on macOS packaging.");
    }
}
