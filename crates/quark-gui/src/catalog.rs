//! Single source of truth for dropdown values. Frontends must not keep a second copy.

pub use quark_core::media::{AUDIO_FORMATS, VIDEO_FORMATS};

pub const SPACES: &[&str] = &["keep", "underscore", "dash", "remove"];
pub const MODES: &[&str] = &["progress", "external_cli"];
pub const THEMES: &[&str] = &["light", "dark"];
pub const LINUX_FRONTENDS: &[&str] = &["auto", "gtk"];
pub const TOOL_SOURCES: &[&str] = &["auto", "path", "bundled"];

pub fn formats_for(media: quark_core::MediaType) -> &'static [&'static str] {
    quark_core::Format::choices(media)
}
