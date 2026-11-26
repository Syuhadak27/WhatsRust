use crate::{config::{Settings, WindowState}, tray::{Tray, TrayCommand}};
use cef::*;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Default)]
pub struct SharedState {
    pub unread: u32,
    pub visible: bool,
    pub should_quit: bool,
    pub reload: bool,
    /// The main browser, handed to us by `LifeSpanHandler::on_after_created`.
    /// Kept here because CEF creates it asynchronously on macOS (see
    /// `create_browser`), so there is no return value to hold onto.
    pub browser: Option<Browser>,
    /// Latest client-area size in **physical** pixels, recorded by winit.
    /// Used for `WindowInfo.bounds` at creation and kept as a reference for
    /// size/scale notifications driven by the event loop.
    pub window_size: Option<(u32, u32)>,
    /// Set by `Resized`; consumed outside the winit handler by `apply_resize`.
    pub pending_resize: bool,
    /// Set by `ScaleFactorChanged`; consumed by `apply_resize`.
    pub scale_changed: bool,
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
    /// Last badge state pushed to the tray, so we don't rebuild the icon on
    /// every event-loop pump.
    tray_unread: bool,
    window_state: WindowState,
    settings: Settings,
}

impl WhatRustApp {
    pub fn new() -> Self {
        let _ = crate::paths::ensure_dirs();
        Self {
            window: None,
            tray: None,
            tray_unread: false,
            window_state: WindowState::load(),
            settings: Settings::load(),
        }
    }

    pub fn run(&mut self) {
        let mut event_loop = EventLoop::new().expect("event loop");
        // With `pump_app_events` the explicit timeout drives the loop; using
        // `Poll` here would make winit wake immediately and spin the CPU.
        #[cfg(target_os = "macos")]
        event_loop.set_control_flow(ControlFlow::Wait);
        #[cfg(not(target_os = "macos"))]
        event_loop.set_control_flow(ControlFlow::Poll);

        #[cfg(target_os = "macos")]
        {
            // macOS: winit defers events that arrive while its `ApplicationHandler`
            // is borrowed via `CFRunLoopPerformBlock`. CEF's `DoMessageLoopWork`
            // enters a nested `NSRunLoop`, which executes those deferred blocks
            // *re-entrantly* and makes winit panic at
            // `event_handler.rs:135` ("tried to handle event while another event
            // is currently being handled") -> abort.
            //
            // Driving winit with `pump_app_events` unsets the handler as soon as
            // each pump returns. We then let CEF do its work while no winit
            // callback is active, so any event CEF's nested loop queues is
            // delivered on the next pump instead of re-entering the handler.
            use std::time::Duration;
            use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};

            loop {
                let hidden = !state().lock().map(|s| s.visible).unwrap_or(true);
                let tick = if hidden { 20 } else { 4 };
                let status = event_loop.pump_app_events(Some(Duration::from_millis(tick)), self);
                // Apply resize/scale notifications before pumping CEF so the
                // browser paints at the correct size from the very first frame.
                self.apply_resize();
                // A single CefDoMessageLoopWork() call is only one iteration.
                // Pump a few iterations per slice so loading/rendering stays
                // responsive during startup without spinning an external loop.
                for _ in 0..(if hidden { 1 } else { 3 }) {
                    do_message_loop_work();
                }
                if matches!(status, PumpStatus::Exit(_)) {
                    break;
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        event_loop.run_app(self).expect("run app");
    }

    fn create_browser(&mut self, event_loop: &ActiveEventLoop) {
        let logical_size = winit::dpi::LogicalSize::new(
            self.window_state.width.max(800),
            self.window_state.height.max(600),
        );
        let mut attrs = Window::default_attributes()
            .with_title("whatRust")
            .with_inner_size(logical_size);
        if let (Some(x), Some(y)) = (self.window_state.x, self.window_state.y) {
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x, y));
        }
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        if self.settings.start_minimized {
            window.set_visible(false);
            if let Ok(mut s) = state().lock() { s.visible = false; }
        } else if let Ok(mut s) = state().lock() { s.visible = true; }
        if self.window_state.maximized { window.set_maximized(true); }

        // Remember the real client-area size (physical px) before CEF is
        // created. Both WindowInfo.bounds and the initial was_resized() need
        // it; relying on CEF to pick up the NSView size misses DPI scaling and
        // any size change caused by set_maximized().
        let size = window.inner_size();
        if let Ok(mut s) = state().lock() {
            s.window_size = Some((size.width, size.height));
            s.pending_resize = true;
        }

        let window_info = window_info_for(&window, size, window.scale_factor());
        let browser_settings = BrowserSettings::default();
        let mut client = crate::browser::ClientImpl::new(state());
        let url = CefString::from("https://web.whatsapp.com");

        // On macOS the synchronous creation API spins a nested `NSRunLoop`
        // while waiting for `OnAfterCreated`. `resumed()` is dispatched from
        // inside winit's handler, so that nested loop can run one of winit's
        // deferred events re-entrantly and abort the process. The asynchronous
        // variant does not block, and `LifeSpanHandler::on_after_created`
        // stores the resulting browser in `SharedState` for us.
        #[cfg(target_os = "macos")]
        browser_host_create_browser(
            Some(&window_info),
            Some(&mut client),
            Some(&url),
            Some(&browser_settings),
            None,
            None,
        );
        #[cfg(not(target_os = "macos"))]
        let browser = browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&url),
            Some(&browser_settings),
            None,
            None,
        );
        #[cfg(not(target_os = "macos"))]
        if let Ok(mut s) = state().lock() { s.browser = browser; }

        self.window = Some(window);
    }

    fn toggle_visible(&mut self) {
        let visible = if let Ok(mut s) = state().lock() { s.visible = !s.visible; s.visible } else { true };
        self.apply_visibility(visible);
    }

    /// Show/hide the native window AND tell CEF. Without `was_hidden`, the
    /// renderer keeps painting into a hidden NSView (RAM/CPU stay high), and
    /// after `set_visible(true)` the compositor never restarts -> black screen.
    fn apply_visibility(&mut self, visible: bool) {
        if let Ok(mut s) = state().lock() { s.visible = visible; }
        let browser = state().lock().ok().and_then(|s| s.browser.clone());
        if !visible {
            if let Some(w) = &self.window { w.set_visible(false); }
            if let Some(host) = browser.and_then(|b| b.host()) { host.was_hidden(1); }
        } else {
            if let Some(w) = &self.window {
                w.set_visible(true);
                w.set_minimized(false);
                w.focus_window();
            }
            if let Some(host) = browser.and_then(|b| b.host()) {
                host.was_hidden(0);
                host.was_resized();
                host.notify_screen_info_changed();
                host.set_focus(1);
            }
        }
    }

    /// Tell CEF about size/scale changes. All CEF calls happen here with the
    /// state mutex released, so CEF callbacks (which also lock the state) can
    /// never deadlock against us.
    fn apply_resize(&mut self) {
        let (browser, pending_resize, scale_changed) = {
            // Bind the Arc first: `state().lock()` alone would create a
            // temporary Arc that is dropped while the MutexGuard still
            // borrows from it (E0716).
            let shared = state();
            let Ok(s) = shared.lock() else { return };
            if !s.pending_resize && !s.scale_changed { return; }
            // Clone the handle out, then release the lock *before* calling
            // CEF: CEF handlers also lock this state and would deadlock.
            (s.browser.clone(), s.pending_resize, s.scale_changed)
        };

        if let Some(browser) = browser {
            if let Some(host) = browser.host() {
                if pending_resize {
                    host.was_resized();
                }
                if scale_changed {
                    host.notify_screen_info_changed();
                }
            }
            if let Ok(mut s) = state().lock() {
                s.pending_resize = false;
                s.scale_changed = false;
            }
        }
        // If the browser is not ready yet, leave the flags set; the next
        // loop iteration will apply them once `on_after_created` has stored
        // the handle.
    }

    /// Clone the browser out of shared state so CEF calls never happen while
    /// the state mutex is held (CEF handlers may synchronously call back into
    /// the same state).
    fn reload_browser() {
        let browser = state().lock().ok().and_then(|s| s.browser.clone());
        if let Some(b) = browser { b.reload(); }
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
                    self.apply_visibility(false);
                } else {
                    if let Ok(mut s) = state().lock() { s.should_quit = true; }
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Ok(mut s) = state().lock() {
                    s.window_size = Some((size.width, size.height));
                    s.pending_resize = true;
                }
                self.persist_window_state();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Ok(mut s) = state().lock() {
                    s.scale_changed = true;
                    s.pending_resize = true;
                }
            }
            WindowEvent::Moved(_) => {
                self.persist_window_state();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // On non-macOS there is no external pump loop, so drive CEF and the
        // deferred resize notifications from here.
        #[cfg(not(target_os = "macos"))]
        {
            do_message_loop_work();
            self.apply_resize();
        }

        if let Some(tray) = &self.tray {
            match tray.poll_event() {
                Some(TrayCommand::Show) => self.toggle_visible(),
                Some(TrayCommand::Reload) => Self::reload_browser(),
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
        if reload { Self::reload_browser(); }
        let unread = unread > 0;
        if unread != self.tray_unread {
            self.tray_unread = unread;
            if let Some(tray) = &self.tray { tray.set_unread(unread); }
        }
    }
}

fn window_info_for(
    window: &Arc<Window>,
    size: PhysicalSize<u32>,
    scale_factor: f64,
) -> WindowInfo {
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
    // WindowInfo::default() has a 0x0 bounds rect. Hand CEF the real client
    // area so the browser view is created at full size instead of a tiny
    // 1x1 view in the top-left corner. CEF's macOS API takes DIP (points),
    // while Windows/X11 take device pixels.
    #[cfg(target_os = "macos")]
    let (width, height) = {
        let scale = if scale_factor > 0.0 { scale_factor } else { 1.0 };
        (
            (size.width as f64 / scale).round() as i32,
            (size.height as f64 / scale).round() as i32,
        )
    };
    #[cfg(not(target_os = "macos"))]
    let (width, height) = {
        let _ = scale_factor;
        (size.width as i32, size.height as i32)
    };

    info.bounds = Rect { x: 0, y: 0, width, height };
    info
}
