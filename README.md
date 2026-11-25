# whatRust — Klien WhatsApp Web Desktop (Rust + CEF)

[![Platform: macOS Intel Ventura](https://img.shields.io/badge/platform-macOS%20Intel%20Ventura-informational)](https://github.com/Syuhadak27/WhatsRust/releases/latest)

---

## Tentang whatRust

**whatRust** adalah klien desktop WhatsApp Web yang ringan dan open-source. Aplikasi ini dibangun dengan **Ratural murni + CEF (Chromium Embedded Framework)** dan tidak menggunakan Tauri atau Electron. Aplikasi ini memuat `web.whatsapp.com` melalui webview native sistem dan menambahkan kemudahan desktop seperti tray, notifikasi native, sesi yang tetap ada, dan akses mikrofon/ekstra untuk panggilan suara.

whatRust adalah **tidak resmi dan mandiri** — tidak terasosiasi dengan WhatsApp atau Meta. Aplikasi ini hanya memuat antarmuka resmi `web.whatsapp.com` dalam webview native dan tidak memodifikasi atau mengganggu layanan WhatsApp.

### Alur pembangunan
- **Dulu:** Tauri + WebView (WebKitGTK / WebView2 / WKWebView)
- **Sekarang:** Rust + CEF 151.8.1, tanpa engine browser yang dibundel
- **Auto-updater:** Dihapus total
- **Target runtime:** macOS 13 Ventura (Skylake/Intel x86_64) dengan CEF framework di `Contents/Frameworks/`

---

## Fitur

- **Tray (sistem tray)** — ikon di bilah sistem, close-to-tray, badge tidak terbaca
- **Notifikasi native** — notifikasi OS untuk pesan masuk
- **Akses kamera/mikrofon** — untuk WhatsApp voice calls dan voice notes (diberikan otomatis via CEF handler `OnRequestMediaAccessPermission`)
- **Sesi persistensi** — data login disimpan di direktori OS app data (multi-account isolasi jika disimpan)
- **JS bridge injection** — `bridge.js`, `media.js`, `privacy.js`, `integration.js` disuntikkan ke frame WhatsApp untuk notifikasi/link luar/privacy/media
- **Penghilang latar (privacy blur)** — pengaturan untuk memblur bagian chat
- **Penyesuaian Ventura** — `MACOSX_DEPLOYMENT_TARGET=13.0` dan `LSMinimumSystemVersion=13.0`
- **Tidak ada updater otomatis** — kode updater (`updater.rs`, `update_service.rs`, `settings-ui/updates.js` dan script terkait) telah dihapus total

---

## Bangunan (Build) — bagi pengembang

Proyek ini telah dideploy ulang dari Tauri ke **Rust murni + CEF**. Struktur utama:

| File | Deskripsi |
|---|---|
| `src-tauri/src/main.rs` | Entry point CEF, Settings, inisialisasi `BrowserApp` |
| `src-tauri/src/browser.rs` | CEF handlers: anti-throttling, media permission, title/unread, JS bridge, console |
| `src-tauri/src/app.rs` | Event loop winit (macOS: `pump_app_events` agar CEF tidak re-entrant), window state, hide-to-tray, perintah tray |
| `src-tauri/src/bridge.rs` | Handler pesan dari JavaScript (`notify`, `set_unread`, `open_external`) |
| `src-tauri/src/helper.rs` | Sub-proses CEF macOS — helper binary dengan `Info.plist` dan entitlements |
| `src-tauri/src/main.rs` | Loop event, CEF settings, cache paths, startup |
| `src-tauri/src/config.rs` | Setelan: close_to_tray, start_minimized, notifications, dsb |
| `src-tauri/src/paths.rs` | Direktori data OS, cache, profil, setting, state jendela |
| `src-tauri/src/window_state.rs` | State jendela: lebar, tinggi, posisi, maximized |
| `src-tauri/src/tray.rs` | Ikon tray, menu, badge notifikasi |
| `src-tauri/src/notifications.rs` | Notifikasi native via `notify-rust` |
| `src-tauri/src/icons.rs` | Ikon tray: normal & tidak terbaca (PNG/SVG) |
| `src-tauri/src/mac.rs` | Stub modul macOS |

### Workflow CI (GitHub Actions)

**`.github/workflows/release.yml`** — hanya membangun DMG macOS Intel Ventura:

1. Download CEF 151.3.24 dari Spotify CDN (via script Python dengan retry + `archive.json`)
2. `cargo build --release --target x86_64-apple-darwin`
3. Paket aplikasi `whatRust.app` beserta 5 helper bundle (Helper, Helper (GPU), Helper (Renderer), Helper (Plugin), Helper (Alerts))
4. Ad-hoc codesign untuk framework bawaan dan helper
5. `hdiutil create -format UDZO` → `whatRust-macos-x64-ventura.dmg`
6. `softprops/action-gh-release` — auto-release pada tag `v*` dan push ke `main` (`prerelease: true`, `overwrite_files: true`)

**`.github/workflows/check.yml`** — `cargo check` untuk target Linux, Windows, macOS (dilakukan untuk memastikan tidak ada error kompilasi).

### Cargo.toml

```toml
[package]
name = "whatrust"
version = "0.7.0"
edition = "2021"

[dependencies]
cef = { version = "151.8.1", features = ["sandbox"] }
cef-dll-sys = "151.8.1"
winit = "0.30"
tray-icon = "0.21"
muda = "0.17"
image = { version = "0.25", default-features = false, features = ["png"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
crossbeam-channel = "0.5"
parking_lot = "0.12"
dirs = "6"
open = "5"
url = "2"
percent-encoding = "2"
notify-rust = "4"

[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = ["NSApplication", "NSRunningApplication", "NSWindow", "NSResponder"] }
objc2-foundation = { version = "0.3", features = ["NSString", "NSProcessInfo", "NSNotification", "NSThread"] }

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.62", features = [
  "Win32_Foundation",
  "Win32_UI_WindowsAndMessaging",
  "Win32_Graphics_Gdi",
  "Win32_System_LibraryLoader",
] }

[target.'cfg(target_os = "linux")'.dependencies]
gtk = "0.18"

[build-dependencies]
```

### Arsitekturnya

- `cef` / `cef-dll-sys` v151.8.1 — serasi dengan binary CEF `151.3.24+g2384915+chromium-151.0.7922.174`
- `wrap_*` macros dari `cef-rs` — gaya API modern (tidak seperti pendekatan lama dengan `NonNull<sys:_cef_*_t>`)
- `library_loader::LibraryLoader` — memastikan `libcef` tetap dimuat selama proses hidup (di macOS, melepas loader akan me-unload framework)
- `execute_process` — subprocess CEF di awal `main()`, dengan `--disable-background-timer-throttling`, `--disable-backgrounding-occluded-windows`, `--disable-renderer-backgrounding` agar WhatsApp WebSocket tetap aktif saat tersembunyi
- `OnRequestMediaAccessPermission` — mengizinkan kamera/mikrofon secara otomatis untuk WhatsApp WebRTC (hanya origin `https://web.whatsapp.com`)
- `wrap_permission_handler!` — `cb.cont(...)` bukan `continue_` (sesuai signature CEF)
- Tidak ada dependensi `tauri`, `tauri-build`, `wry`, `tao`
- `parking_lot` digantikan `std::sync::Mutex`

---

## Instalasi

### DMG (macOS Ventura Intel)
Unduh dari [releases terbaru](https://github.com/Syuhadak27/WhatsRust/releases/latest):
- `whatRust-macos-x64-ventura.dmg`

Setelah diunduh:
1. Seret `whatRust.app` ke `Applications`
2. Buka Terminal dan jalankan sekali:
   ```bash
   xattr -dr com.apple.quarantine /Applications/whatRust.app
   ```
3. Jalankan `whatRust` dari Launchpad atau Applications

### Dari source (jika ingin build sendiri)

```bash
# Butuh Rust 1.90+ dan Cargo
# Install dependencies on macOS:
brew install wget libpng jpeg_turbo libtiff giflib libwebp

# Clone dan build
git clone git@github.com:Syuhadak27/WhatsRust.git
cd WhatRust
./scripts/build.sh  # atau gunakan GitHub Actions workflow
```

---

## Struktur direktori yang relevan

```
src-tauri/
├── Cargo.toml          # Dependencies CEF 151.8.1
├── main.rs             # Entry point + CEF init + event loop
├── app.rs              # winit event loop, window state, tray
├── browser.rs          # CEF handlers, anti-throttling, media permission, JS bridge
├── bridge.rs           # Pesan dari JS (notify, set_unread, open_external)
├── helper.rs           # Sub-proses CEF macOS
├── config.rs           # Setelan aplikasi
├── icons.rs            # Ikon tray
├── mac.rs              # Stub modul macOS
├── notifications.rs    # Notifikasi native
├── paths.rs            # Direktori data OS
├── tray.rs             # Menu tray, badge
├── window_state.rs     # State jendela
├── resources/
│   ├── bridge.js       # JS bridge untuk invoke native
│   ├── media.js        # Media handling
│   ├── privacy.js      # Privacy/blur settings
│   └── integration.js  # Integrasi fitur
└── workflows/
    ├── release.yml     # CI build DMG Ventura Intel
    └── check.yml       # CI cargo check
```

---

## Lisensi

**MIT License** — lihat file `LICENSE` untuk detail.

---

## Penulis

Dibangun oleh **Syuhadak27** — proyek open-source untuk mempertahankan fitur WhatsApp Desktop yang ringan tanpa Electron.

*Proyek ini tidak terasosiasi dengan WhatsApp atau Meta.*