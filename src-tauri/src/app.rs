use crate::{config::{Settings, WindowState}, tray::{Tray, TrayCommand}};
use cef::*;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Default)]
pub struct SharedState {
    pub unread: u32,
    pub visible: bool,
    pub should_quit: bool,
    pub reload: bool,
}

static STATE: std::sync::OnceLock<Arc<Mutex<SharedState>>> = std::sync::OnceLock::new();
pub fn state() -> Arc<Mutex<SharedState>> {
    STATE
        .get_or_init(|| Arc::new(Mutex::new(SharedState { visible: true, ..Default::default() })))
        .clone()
}

pub struct WhatRustApp {
    window: Option<Arc<Window>>,
    tray: Option<Tray>,
    browser: Option<Browser>,
    window_state: WindowState,
    settings: Settings,
}

impl WhatRustApp {
    pub fn new() -> Self {
        let _ = crate::paths::ensure_dirs();
        Self {
            window: None,
            tray: None,
            browser: None,
            window_state: WindowState::load(),
            settings: Settings::load(),
        }
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().expect("event loop");
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).expect("run app");
    }

    fn create_browser(&mut self, event_loop: &ActiveEventLoop) {
        let mut attrs = Window::default_attributes()
            .with_title("whatRust")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.window_state.width.max(800),
                self.window_state.height.max(600),
            ));
        if let (Some(x), Some(y)) = (self.window_state.x, self.window_state.y) {
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x, y));
        }
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        if self.settings.start_minimized {
            window.set_visible(false);
            if let Ok(mut s) = state().lock() { s.visible = false; }
        } else if let Ok(mut s) = state().lock() { s.visible = true; }
        if self.window_state.maximized { window.set_maximized(true); }

        let window_info = window_info_for(&window);
        let browser_settings = BrowserSettings::default();
        let mut client = crate::browser::ClientImpl::new(state());
        let url = CefString::from("https://web.whatsapp.com");
        let browser = browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&url),
            Some(&browser_settings),
            None,
            None,
        );
        self.browser = browser;
        self.window = Some(window);
    }

    fn toggle_visible(&mut self) {
        let visible = if let Ok(mut s) = state().lock() { s.visible = !s.visible; s.visible } else { true };
        if let Some(w) = &self.window { w.set_visible(visible); }
    }

    fn persist_window_state(&mut self) {
        let Some(w) = &self.window else { return };
        let size = w.inner_size();
        let pos = w.outer_position().ok();
        self.window_state.width = size.width;
        self.window_state.height = size.height;
        if let Some(p) = pos { self.window_state.x = Some(p.x); self.window_state.y = Some(p.y); }
        self.window_state.maximized = w.is_maximized();
        let _ = self.window_state.save();
    }
}

impl ApplicationHandler for WhatRustApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            match Tray::new() {
                Ok(t) => self.tray = Some(t),
                Err(e) => eprintln!("tray init failed: {e}"),
            }
            self.create_browser(event_loop);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.persist_window_state();
                if self.settings.close_to_tray {
                    if let Some(w) = &self.window { w.set_visible(false); }
                    if let Ok(mut s) = state().lock() { s.visible = false; }
                } else {
                    if let Ok(mut s) = state().lock() { s.should_quit = true; }
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                self.persist_window_state();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        do_message_loop_work();

        if let Some(tray) = &self.tray {
            match tray.poll_event() {
                Some(TrayCommand::Show) => self.toggle_visible(),
                Some(TrayCommand::Reload) => { if let Some(b) = &self.browser { b.reload(); } }
                Some(TrayCommand::Quit) => {
                    self.persist_window_state();
                    if let Ok(mut s) = state().lock() { s.should_quit = true; }
                    event_loop.exit();
                }
                None => {}
            }
        }

        let (unread, reload, quit) = if let Ok(mut s) = state().lock() {
            let r = (s.unread, s.reload, s.should_quit);
            s.reload = false;
            r
        } else { (0, false, false) };

        if quit { self.persist_window_state(); event_loop.exit(); }
        if reload { if let Some(b) = &self.browser { b.reload(); } }
        if let Some(tray) = &self.tray { tray.set_unread(unread > 0); }
    }
}

fn window_info_for(window: &Arc<Window>) -> WindowInfo {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let mut info = WindowInfo::default();
    let handle = window.window_handle().expect("window handle").as_raw();
    match handle {
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(h) => { info.window = h.hwnd.get() as _; }
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(h) => { info.parent_view = h.ns_view.as_ptr() as _; }
        #[cfg(target_os = "linux")]
        RawWindowHandle::Xlib(h) => { info.parent_window = h.window as _; }
        #[cfg(target_os = "linux")]
        RawWindowHandle::Xcb(h) => { info.parent_window = h.window.get() as _; }
        _ => {}
    }
    info
}
