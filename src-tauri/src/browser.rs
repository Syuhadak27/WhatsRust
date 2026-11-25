use cef::*;
use std::sync::{Arc, Mutex};

use crate::app::SharedState;

/// Keeps the CEF framework loaded for the lifetime of the process.
/// Dropping the loader on macOS unloads libcef, so the main binary must
/// hold this value until after `shutdown()`.
#[cfg(target_os = "macos")]
pub struct Library(library_loader::LibraryLoader);
#[cfg(not(target_os = "macos"))]
pub struct Library;

pub fn load_cef() -> Library {
    #[cfg(target_os = "macos")]
    let library = {
        let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), false);
        assert!(loader.load(), "failed to load Chromium Embedded Framework");
        Library(loader)
    };
    #[cfg(not(target_os = "macos"))]
    let library = Library;

    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    library
}

wrap_app! {
    pub struct BrowserApp {
        state: Arc<Mutex<SharedState>>,
    }

    impl App {
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            if let Some(cmd) = command_line {
                let switches = [
                    "disable-background-timer-throttling",
                    "disable-backgrounding-occluded-windows",
                    "disable-renderer-backgrounding",
                ];
                for s in switches {
                    cmd.append_switch(Some(&CefString::from(s)));
                }
                cmd.append_switch_with_value(
                    Some(&CefString::from("autoplay-policy")),
                    Some(&CefString::from("no-user-gesture-required")),
                );
            }
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(BrowserProcessHandlerImpl::new(self.state.clone()))
        }
    }
}

wrap_browser_process_handler! {
    struct BrowserProcessHandlerImpl {
        state: Arc<Mutex<SharedState>>,
    }

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            // Browser is created by winit/app flow in our app; nothing to do here.
        }
    }
}

wrap_client! {
    pub struct ClientImpl {
        state: Arc<Mutex<SharedState>>,
    }

    impl Client {
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(DisplayHandlerImpl::new(self.state.clone()))
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(LifeSpanHandlerImpl::new(self.state.clone()))
        }

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(LoadHandlerImpl::new(self.state.clone()))
        }

        fn permission_handler(&self) -> Option<PermissionHandler> {
            Some(PermissionHandlerImpl::new(self.state.clone()))
        }

        fn on_process_message_received(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _source_process: ProcessId,
            message: Option<&mut ProcessMessage>,
        ) -> i32 {
            let Some(msg) = message else { return 0 };
            let name = CefString::from(&msg.name()).to_string();
            if name != "whatrust" {
                return 0;
            }
            let args = msg.argument_list();
            let Some(args) = args else { return 0 };
            let payload = CefString::from(&args.string(0)).to_string();
            if let Ok(parsed) = serde_json::from_str::<crate::bridge::BridgeMessage>(&payload) {
                let _ = crate::bridge::handle_message(&parsed);
            }
            1
        }
    }
}

wrap_display_handler! {
    struct DisplayHandlerImpl {
        state: Arc<Mutex<SharedState>>,
    }

    impl DisplayHandler {
        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            let title = title.map(CefString::to_string).unwrap_or_default();
            let count = crate::window_state::parse_unread_from_title(&title);
            if let Ok(mut st) = self.state.lock() {
                st.unread = count;
            }
        }

        fn on_console_message(
            &self,
            _browser: Option<&mut Browser>,
            _level: LogSeverity,
            message: Option<&CefString>,
            _source: Option<&CefString>,
            _line: ::std::os::raw::c_int,
        ) -> i32 {
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
}

wrap_life_span_handler! {
    struct LifeSpanHandlerImpl {
        state: Arc<Mutex<SharedState>>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            // The browser is created asynchronously on macOS, so this is the
            // only place we can get a handle to it. Ignore popups/devtools and
            // keep the main browser in shared state for reload/quit handling.
            let Some(browser) = browser else { return };
            if browser.is_popup() != 0 { return; }
            if let Ok(mut s) = self.state.lock() {
                s.browser = Some((*browser).clone());
            }
        }

        fn on_before_popup(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _popup_id: ::std::os::raw::c_int,
            target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            _target_disposition: WindowOpenDisposition,
            _user_gesture: ::std::os::raw::c_int,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut ::std::os::raw::c_int>,
        ) -> i32 {
            if let Some(url) = target_url {
                let u = url.to_string();
                if u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:") {
                    let _ = open::that(&u);
                }
            }
            1
        }
    }
}

wrap_permission_handler! {
    struct PermissionHandlerImpl {
        state: Arc<Mutex<SharedState>>,
    }

    impl PermissionHandler {
        fn on_request_media_access_permission(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            requesting_origin: Option<&CefString>,
            requested_permissions: u32,
            callback: Option<&mut MediaAccessCallback>,
        ) -> i32 {
            let origin = requesting_origin.map(CefString::to_string).unwrap_or_default();
            let allow = origin.starts_with("https://web.whatsapp.com");
            if let Some(cb) = callback {
                if allow {
                    cb.cont(requested_permissions);
                } else {
                    cb.cont(0);
                }
            }
            1
        }
    }
}

wrap_load_handler! {
    struct LoadHandlerImpl {
        state: Arc<Mutex<SharedState>>,
    }

    impl LoadHandler {
        fn on_load_end(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            http_status_code: ::std::os::raw::c_int,
        ) {
            let Some(frame) = frame else { return };
            if frame.is_main() == 0 { return; }
            if http_status_code == 0 { return; }

            let settings = crate::config::Settings::load();
            let shim = r#"(function(){
  if(window.__WR_CEF__) return; window.__WR_CEF__=true;
  window.whatRust = window.whatRust || {};
  window.whatRust.invoke = function(cmd, args){
    args = args || {};
    var payload = JSON.stringify(Object.assign({cmd: cmd}, args));
    console.log("__WR_CMD__:" + payload);
    return Promise.resolve({ok:true});
  };
  window.__TAURI__ = window.__TAURI__ || { core:{ invoke: (c,a)=>window.whatRust.invoke(c,a) }, event:{ listen: async ()=>()=>{} } };
})();"#;
            frame.execute_java_script(
                Some(&CefString::from(shim)),
                Some(&CefString::from("https://web.whatsapp.com")),
                0,
            );

            for src in [
                include_str!("../resources/bridge.js"),
                include_str!("../resources/media.js"),
                include_str!("../resources/privacy.js"),
                include_str!("../resources/integration.js"),
            ] {
                frame.execute_java_script(
                    Some(&CefString::from(src)),
                    Some(&CefString::from("https://web.whatsapp.com")),
                    0,
                );
            }
            if settings.hide_app_banner {
                let upsell = include_str!("../resources/upsell.js");
                frame.execute_java_script(
                    Some(&CefString::from(upsell)),
                    Some(&CefString::from("https://web.whatsapp.com")),
                    0,
                );
            }
        }
    }
}
