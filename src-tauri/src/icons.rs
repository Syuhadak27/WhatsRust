pub const TRAY_NORMAL: &[u8] = include_bytes!("../icons/tray.png");
pub const TRAY_UNREAD: &[u8] = include_bytes!("../icons/tray-unread.png");
#[cfg(target_os = "macos")]
pub const TRAY_NORMAL_MAC: &[u8] = include_bytes!("../icons/tray-macos-template.png");
#[cfg(target_os = "macos")]
pub const TRAY_UNREAD_MAC: &[u8] = include_bytes!("../icons/tray-macos-template-unread.png");
pub const APP_ICON: &[u8] = include_bytes!("../icons/icon.png");

pub fn tray_icon(unread: bool) -> &'static [u8] {
    #[cfg(target_os = "macos")]
    { if unread { TRAY_UNREAD_MAC } else { TRAY_NORMAL_MAC } }
    #[cfg(not(target_os = "macos"))]
    { if unread { TRAY_UNREAD } else { TRAY_NORMAL } }
}

pub fn load_rgba(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}
