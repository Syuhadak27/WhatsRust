use serde::{Deserialize, Serialize};

pub const MSG_NOTIFY: &str = "notify";
pub const MSG_SET_UNREAD: &str = "set_unread";
pub const MSG_OPEN_EXTERNAL: &str = "open_external";

#[derive(Debug, Deserialize)]
pub struct BridgeMessage {
    pub cmd: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct BridgeReply {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn handle_message(msg: &BridgeMessage) -> BridgeReply {
    match msg.cmd.as_str() {
        MSG_NOTIFY => {
            crate::notifications::show(&msg.title, &msg.body);
            BridgeReply { ok: true, error: None }
        }
        MSG_SET_UNREAD => {
            if let Ok(mut st) = crate::app::state().lock() { st.unread = msg.count; }
            BridgeReply { ok: true, error: None }
        }
        MSG_OPEN_EXTERNAL => {
            if msg.url.starts_with("http://") || msg.url.starts_with("https://") || msg.url.starts_with("mailto:") {
                let _ = open::that(&msg.url);
            }
            BridgeReply { ok: true, error: None }
        }
        _ => BridgeReply { ok: false, error: Some("unknown command".into()) },
    }
}
