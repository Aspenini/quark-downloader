//! Single source of truth for dropdown values. Frontends must not keep a second copy.

pub use quark_core::media::{AUDIO_FORMATS, VIDEO_FORMATS};

pub const SPACES: &[&str] = &["keep", "underscore", "dash", "remove"];
pub const MODES: &[&str] = &["progress", "external_cli"];
pub const THEMES: &[&str] = &["system", "light", "dark"];
pub const TOOL_SOURCES: &[&str] = &["auto", "path", "bundled"];

pub fn formats_for(media: quark_core::MediaType) -> &'static [&'static str] {
    quark_core::Format::choices(media)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_are_stable() {
        assert_eq!(THEMES, &["system", "light", "dark"]);
        assert_eq!(SPACES, &["keep", "underscore", "dash", "remove"]);
        assert_eq!(MODES, &["progress", "external_cli"]);
    }
}
