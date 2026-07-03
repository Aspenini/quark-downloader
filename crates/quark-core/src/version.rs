//! Application version and window-title strings.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = "Quark Downloader";

pub fn window_title() -> String {
    format!("{APP_NAME} {VERSION}")
}

pub fn settings_window_title() -> String {
    format!("{} Settings", window_title())
}
