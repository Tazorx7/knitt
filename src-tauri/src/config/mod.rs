pub mod autostart;
pub mod crash;
pub mod diagnostics;
pub mod paths;
pub mod settings;
pub mod ssh;

/// Tauri's application identifier, and the reverse-DNS name the app uses in
/// AppStream and desktop metadata. It also prefixes the keyring service and the
/// HTTP user agent, so it lives in exactly one place here and is mirrored in
/// `tauri.conf.json`, which JSON cannot import.
pub const APP_ID: &str = "io.github.tazorx7.knitt";
