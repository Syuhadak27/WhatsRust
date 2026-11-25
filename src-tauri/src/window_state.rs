pub use crate::config::WindowState;

pub fn parse_unread_from_title(title: &str) -> u32 {
    let t = title.trim();
    if let Some(rest) = t.strip_prefix('(') {
        if let Some(end) = rest.find(')') {
            if let Ok(n) = rest[..end].replace('+', "").trim().parse::<u32>() {
                return n;
            }
        }
    }
    0
}
