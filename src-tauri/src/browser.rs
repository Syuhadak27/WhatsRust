use cef::rc::{Rc, RcImpl};
use cef::{sys, wrap_app, wrap_browser_process_handler, wrap_client, wrap_display_handler, wrap_life_span_handler, wrap_load_handler, wrap_permission_handler, wrap_render_process_handler, wrap_request_handler, CefString, ImplApp, ImplBrowser, ImplBrowserHost, ImplCommandLine, ImplFrame, ImplLifeSpanHandler, ImplLoadHandler, ImplPermissionHandler, ImplProcessMessage, ImplRenderProcessHandler, ImplRequestHandler, ImplV8Context, ProcessId, ProcessMessage, V8Context};
use parking_lot::Mutex;
use std::sync::Arc;

use crate::app::SharedState;

// -------- Process app (all processes) --------
#[derive(Clone)]
pub struct ProcessApp { obj: std::ptr::NonNull<sys::_cef_app_t> }
impl ProcessApp {
    pub fn new() -> Self { Self { obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_app!(Self { }, render_process_handler: RenderProcessHandler::new())))) } } }
}
impl Rc for ProcessApp { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplApp for ProcessApp {
    fn get_raw(&self) -> *mut sys::_cef_app_t { self.obj.as_ptr() }
    fn on_before_command_line_processing(&self, _process_type: Option<&CefString>, command_line: Option<&mut cef::CommandLine>) {
        if let Some(cmd) = command_line {
            cmd.append_switch(Some(&"disable-background-timer-throttling".into()));
            cmd.append_switch(Some(&"disable-backgrounding-occluded-windows".into()));
            cmd.append_switch(Some(&"disable-renderer-backgrounding".into()));
            cmd.append_switch_with_value(Some(&"autoplay-policy".into()), Some(&"no-user-gesture-required".into()));
        }
    }
    fn render_process_handler(&self) -> Option<cef::RenderProcessHandler> { Some(RenderProcessHandler::new()) }
}

#[derive(Clone)]
pub struct RenderProcessHandler { obj: std::ptr::NonNull<sys::_cef_render_process_handler_t> }
impl RenderProcessHandler {
    pub fn new() -> cef::RenderProcessHandler {
        let this = Self { obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_render_process_handler!(Self { })))) } };
        unsafe { std::mem::transmute(this) }
    }
}
impl Rc for RenderProcessHandler { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplRenderProcessHandler for RenderProcessHandler {
    fn get_raw(&self) -> *mut sys::_cef_render_process_handler_t { self.obj.as_ptr() }
    fn on_context_created(&self, browser: Option<&mut cef::Browser>, frame: Option<&mut cef::Frame>, _context: Option<&mut V8Context>) {
        let Some(frame) = frame else { return };
        if !frame.is_main() { return; }
        // Expose a simple sender; actual handling is in browser process via process message.
        let shim = r#"
window.whatRust = window.whatRust || {};
window.whatRust.__send = function(payload){
  try { native function whatrustSend(); whatrustSend(payload); } catch(e) {
    window.postMessage({__wr:true,payload:payload}, '*');
  }
};
window.__TAURI__ = window.__TAURI__ || { core:{ invoke: (c,a)=>window.whatRust.invoke(c,a) }, event:{ listen: async ()=>()=>{} } };
"#;
        frame.execute_java_script(Some(&shim.into()), Some(&"https://web.whatsapp.com".into()), 0);
        let _ = browser;
    }
}

// -------- Browser process app --------
#[derive(Clone)]
pub struct BrowserApp { obj: std::ptr::NonNull<sys::_cef_app_t>, state: Arc<Mutex<SharedState>> }
impl BrowserApp {
    pub fn new(state: Arc<Mutex<SharedState>>) -> Self {
        Self { state, obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_app!(Self { state: Arc<Mutex<SharedState>> }, browser_process_handler: BrowserProcessHandler { state: state.clone() })))) } }
    }
}
impl Rc for BrowserApp { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplApp for BrowserApp {
    fn get_raw(&self) -> *mut sys::_cef_app_t { self.obj.as_ptr() }
    fn browser_process_handler(&self) -> Option<cef::BrowserProcessHandler> { Some(BrowserProcessHandler::new(self.state.clone())) }
    fn on_before_command_line_processing(&self, _process_type: Option<&CefString>, command_line: Option<&mut cef::CommandLine>) {
        if let Some(cmd) = command_line {
            cmd.append_switch(Some(&"disable-background-timer-throttling".into()));
            cmd.append_switch(Some(&"disable-backgrounding-occluded-windows".into()));
            cmd.append_switch(Some(&"disable-renderer-backgrounding".into()));
            cmd.append_switch_with_value(Some(&"autoplay-policy".into()), Some(&"no-user-gesture-required".into()));
        }
    }
}

#[derive(Clone)]
pub struct BrowserProcessHandler { obj: std::ptr::NonNull<sys::_cef_browser_process_handler_t>, state: Arc<Mutex<SharedState>> }
impl BrowserProcessHandler {
    pub fn new(state: Arc<Mutex<SharedState>>) -> cef::BrowserProcessHandler {
        let this = Self { state, obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_browser_process_handler!(Self { state: Arc<Mutex<SharedState>> })))) } };
        unsafe { std::mem::transmute(this) }
    }
}
impl Rc for BrowserProcessHandler { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplBrowserProcessHandler for BrowserProcessHandler { fn get_raw(&self) -> *mut sys::_cef_browser_process_handler_t { self.obj.as_ptr() } }

// -------- Client --------
#[derive(Clone)]
pub struct Client { obj: std::ptr::NonNull<sys::_cef_client_t>, state: Arc<Mutex<SharedState>> }
impl Client {
    pub fn new(state: Arc<Mutex<SharedState>>) -> Self {
        let st = state.clone();
        Self {
            state,
            obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_client!(Self { state: Arc<Mutex<SharedState>> },
                display_handler: DisplayHandler { state: st.clone() },
                life_span_handler: LifeSpanHandler { state: st.clone() },
                request_handler: RequestHandler { state: st.clone() },
                load_handler: LoadHandler { state: st.clone() },
                permission_handler: PermissionHandler { state: st.clone() },
            )))) }
        }
    }
}
impl Rc for Client { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplClient for Client {
    fn get_raw(&self) -> *mut sys::_cef_client_t { self.obj.as_ptr() }
    fn display_handler(&self) -> Option<cef::DisplayHandler> { Some(DisplayHandler::new(self.state.clone())) }
    fn life_span_handler(&self) -> Option<cef::LifeSpanHandler> { Some(LifeSpanHandler::new(self.state.clone())) }
    fn request_handler(&self) -> Option<cef::RequestHandler> { Some(RequestHandler::new(self.state.clone())) }
    fn load_handler(&self) -> Option<cef::LoadHandler> { Some(LoadHandler::new(self.state.clone())) }
    fn permission_handler(&self) -> Option<cef::PermissionHandler> { Some(PermissionHandler::new(self.state.clone())) }

    fn on_process_message_received(&self, _browser: Option<&mut cef::Browser>, _frame: Option<&mut cef::Frame>, _source: ProcessId, message: Option<&mut ProcessMessage>) -> sys::c_int {
        let Some(msg) = message else { return 0; };
        let name = CefString::from(&msg.name()).to_string();
        if name != "whatrust" { return 0; }
        let args = msg.argument_list().unwrap();
        let payload = CefString::from(&args.string(0)).to_string();
        if let Ok(parsed) = serde_json::from_str::<crate::bridge::BridgeMessage>(&payload) {
            let reply = crate::bridge::handle_message(&parsed);
            if parsed.cmd == "set_unread" {
                // tray update happens in app loop via shared state
            }
            let _ = reply;
        }
        1
    }
}

// -------- Display handler (title/unread) --------
#[derive(Clone)]
pub struct DisplayHandler { obj: std::ptr::NonNull<sys::_cef_display_handler_t>, state: Arc<Mutex<SharedState>> }
impl DisplayHandler {
    pub fn new(state: Arc<Mutex<SharedState>>) -> cef::DisplayHandler {
        let this = Self { state, obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_display_handler!(Self { state: Arc<Mutex<SharedState>> })))) } };
        unsafe { std::mem::transmute(this) }
    }
}
impl Rc for DisplayHandler { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplDisplayHandler for DisplayHandler {
    fn get_raw(&self) -> *mut sys::_cef_display_handler_t { self.obj.as_ptr() }
    fn on_title_change(&self, _browser: Option<&mut cef::Browser>, title: Option<&CefString>) {
        let title = title.map(|t| t.to_string()).unwrap_or_default();
        let count = crate::window_state::parse_unread_from_title(&title);
        self.state.lock().unread = count;
    }
    fn on_console_message(&self, _browser: Option<&mut cef::Browser>, _level: cef::LogSeverity, message: Option<&CefString>, _source: Option<&CefString>, _line: i32) -> sys::c_int {
        if let Some(msg) = message {
            let s = msg.to_string();
            if let Some(payload) = s.strip_prefix("__WR_CMD__:") {
                if let Ok(parsed) = serde_json::from_str::<crate::bridge::BridgeMessage>(payload) {
                    let _ = crate::bridge::handle_message(&parsed);
                }
                return 1;
            }
        }
        0
    }
}

// -------- Life span handler --------
#[derive(Clone)]
pub struct LifeSpanHandler { obj: std::ptr::NonNull<sys::_cef_life_span_handler_t>, state: Arc<Mutex<SharedState>> }
impl LifeSpanHandler {
    pub fn new(state: Arc<Mutex<SharedState>>) -> cef::LifeSpanHandler {
        let this = Self { state, obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_life_span_handler!(Self { state: Arc<Mutex<SharedState>> })))) } };
        unsafe { std::mem::transmute(this) }
    }
}
impl Rc for LifeSpanHandler { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplLifeSpanHandler for LifeSpanHandler {
    fn get_raw(&self) -> *mut sys::_cef_life_span_handler_t { self.obj.as_ptr() }
    fn on_before_popup(&self, _browser: Option<&mut cef::Browser>, _frame: Option<&mut cef::Frame>, _popup_id: i32, target_url: Option<&CefString>, _target_frame_name: Option<&CefString>, _target_disposition: cef::WindowOpenDisposition, _user_gesture: sys::c_int, _popup_features: Option<&cef::PopupFeatures>, _window_info: Option<&mut cef::WindowInfo>, _client: Option<&mut Option<cef::Client>>, _settings: Option<&mut cef::BrowserSettings>, _extra_info: Option<&mut Option<cef::DictionaryValue>>, _no_javascript_access: Option<&mut sys::c_int>) -> sys::c_int {
        if let Some(url) = target_url {
            let u = url.to_string();
            if u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:") {
                let _ = open::that(&u);
            }
        }
        1 // cancel popup, we opened externally
    }
}

// -------- Request handler --------
#[derive(Clone)]
pub struct RequestHandler { obj: std::ptr::NonNull<sys::_cef_request_handler_t>, state: Arc<Mutex<SharedState>> }
impl RequestHandler {
    pub fn new(state: Arc<Mutex<SharedState>>) -> cef::RequestHandler {
        let this = Self { state, obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_request_handler!(Self { state: Arc<Mutex<SharedState>> })))) } };
        unsafe { std::mem::transmute(this) }
    }
}
impl Rc for RequestHandler { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplRequestHandler for RequestHandler {
    fn get_raw(&self) -> *mut sys::_cef_request_handler_t { self.obj.as_ptr() }
    fn permission_handler(&self) -> Option<cef::PermissionHandler> { Some(PermissionHandler::new(self.state.clone())) }
}

// -------- Permission handler (camera/mic for WebRTC) --------
#[derive(Clone)]
pub struct PermissionHandler { obj: std::ptr::NonNull<sys::_cef_permission_handler_t>, state: Arc<Mutex<SharedState>> }
impl PermissionHandler {
    pub fn new(state: Arc<Mutex<SharedState>>) -> cef::PermissionHandler {
        let this = Self { state, obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_permission_handler!(Self { state: Arc<Mutex<SharedState>> })))) } };
        unsafe { std::mem::transmute(this) }
    }
}
impl Rc for PermissionHandler { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplPermissionHandler for PermissionHandler {
    fn get_raw(&self) -> *mut sys::_cef_permission_handler_t { self.obj.as_ptr() }
    fn on_request_media_access_permission(&self, _browser: Option<&mut cef::Browser>, _frame: Option<&mut cef::Frame>, requesting_origin: Option<&CefString>, requested_permissions: u32, callback: Option<&mut cef::MediaAccessCallback>) -> sys::c_int {
        let origin = requesting_origin.map(|o| o.to_string()).unwrap_or_default();
        let allow = origin.starts_with("https://web.whatsapp.com");
        if let Some(cb) = callback {
            if allow { cb.continue_(requested_permissions); } else { cb.continue_(0); }
        }
        1
    }
}

// -------- Load handler (inject bridge) --------
#[derive(Clone)]
pub struct LoadHandler { obj: std::ptr::NonNull<sys::_cef_load_handler_t>, state: Arc<Mutex<SharedState>> }
impl LoadHandler {
    pub fn new(state: Arc<Mutex<SharedState>>) -> cef::LoadHandler {
        let this = Self { state, obj: unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(wrap_load_handler!(Self { state: Arc<Mutex<SharedState>> })))) } };
        unsafe { std::mem::transmute(this) }
    }
}
impl Rc for LoadHandler { fn as_base(&self) -> &sys::cef_base_ref_counted_t { unsafe { &(*self.obj.as_ptr()).base } } }
impl ImplLoadHandler for LoadHandler {
    fn get_raw(&self) -> *mut sys::_cef_load_handler_t { self.obj.as_ptr() }
    fn on_load_end(&self, _browser: Option<&mut cef::Browser>, frame: Option<&mut cef::Frame>, http_status_code: i32) {
        let Some(frame) = frame else { return };
        if !frame.is_main() { return; }
        if http_status_code == 0 { return; }
        let settings = crate::config::Settings::load();
        let shim = r#"
(function(){
  if(window.__WR_CEF__) return; window.__WR_CEF__=true;
  window.whatRust = window.whatRust || {};
  window.whatRust.invoke = function(cmd, args){
    args = args || {};
    var payload = JSON.stringify(Object.assign({cmd: cmd}, args));
    console.log("__WR_CMD__:" + payload);
    return Promise.resolve({ok:true});
  };
  window.__TAURI__ = window.__TAURI__ || { core:{ invoke: (c,a)=>window.whatRust.invoke(c,a) }, event:{ listen: async ()=>()=>{} } };
})();
"#;
        frame.execute_java_script(Some(&shim.into()), Some(&"https://web.whatsapp.com".into()), 0);
        // Inject project bridge scripts
        let bridge = include_str!("../resources/bridge.js");
        let media = include_str!("../resources/media.js");
        let privacy = include_str!("../resources/privacy.js");
        let integration = include_str!("../resources/integration.js");
        let upsell = include_str!("../resources/upsell.js");
        for (name, src) in [("bridge", bridge), ("media", media), ("privacy", privacy), ("integration", integration)] {
            let _ = name;
            frame.execute_java_script(Some(&src.into()), Some(&"https://web.whatsapp.com".into()), 0);
        }
        if settings.hide_app_banner {
            frame.execute_java_script(Some(&upsell.into()), Some(&"https://web.whatsapp.com".into()), 0);
        }
    }
}
