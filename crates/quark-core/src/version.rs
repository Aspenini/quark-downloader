pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = "Quark Downloader";

pub fn window_title() -> String {
    format!("{APP_NAME} {VERSION}")
}

pub fn settings_window_title() -> String {
    format!("{} Settings", window_title())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!VERSION.is_empty());
        assert!(window_title().contains(APP_NAME));
        assert!(window_title().contains(VERSION));
    }
}
