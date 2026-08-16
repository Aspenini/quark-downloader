//! Single source of truth for dropdown values. Frontends must not keep a second copy.

pub use quark_core::media::{AUDIO_FORMATS, VIDEO_FORMATS};

pub const SPACES: &[&str] = &["keep", "underscore", "dash", "remove"];
pub const MODES: &[&str] = &["progress", "external_cli"];
pub const THEMES: &[&str] = &["light", "dark"];
pub const TOOL_SOURCES: &[&str] = &["auto", "path", "bundled"];

/// Every settings UI must offer this list (plus `auto`).
pub const ALL_FRONTEND_IDS: &[&str] = &["gtk", "cosmic", "kirigami", "win32", "appkit"];

pub fn formats_for(media: quark_core::MediaType) -> &'static [&'static str] {
    quark_core::Format::choices(media)
}

/// Frontends this host can reasonably run. Helpers that are not installed
/// still appear so the user can pick them and get a clear install error.
pub fn supported_frontends() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &["auto", "win32", "gtk", "cosmic", "kirigami"]
    }
    #[cfg(target_os = "macos")]
    {
        &["auto", "appkit", "gtk", "cosmic", "kirigami"]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        &["auto", "gtk", "cosmic", "kirigami"]
    }
    #[cfg(not(any(windows, unix)))]
    {
        &["auto"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_list_starts_with_auto() {
        let list = supported_frontends();
        assert_eq!(list.first().copied(), Some("auto"));
        assert!(list.contains(&"gtk"));
        assert!(list.contains(&"cosmic"));
        assert!(list.contains(&"kirigami"));
    }
}
