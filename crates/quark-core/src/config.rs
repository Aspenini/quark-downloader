//! Settings: a TOML model with a one-time migration from the legacy
//! `quark-downloader.conf` key=value format.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::paths;

pub const CONFIG_NAME: &str = "quark-downloader.toml";
pub const LEGACY_CONFIG_NAME: &str = "quark-downloader.conf";
pub const DEFAULT_DOWNLOAD_DIR: &str = "~/Downloads";

// ---- enums ---------------------------------------------------------------

macro_rules! str_enum {
    ($name:ident { $($variant:ident => $canon:literal $(| $alias:literal)* ),+ $(,)? } default $def:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $canon),+ }
            }
            /// Parse leniently. Unknown values return `(default, false)`.
            pub fn parse_lenient(value: &str) -> (Self, bool) {
                match value.trim().to_ascii_lowercase().as_str() {
                    $( $canon $(| $alias)* => (Self::$variant, true), )+
                    _ => (Self::$def, false),
                }
            }
        }
        impl Default for $name {
            fn default() -> Self { Self::$def }
        }
    };
}

str_enum!(ToolSource {
    Auto => "auto",
    Path => "path",
    Bundled => "bundled",
} default Auto);

str_enum!(GuiDownloadMode {
    Progress => "progress" | "gui",
    ExternalCli => "external_cli" | "cli" | "terminal",
} default Progress);

str_enum!(GuiTheme {
    Light => "light",
    Dark => "dark",
} default Light);

str_enum!(FilenameSpaces {
    Keep => "keep" | "space" | "spaces",
    Underscore => "underscore" | "underscores",
    Dash => "dash" | "dashes" | "hyphen",
    Remove => "remove" | "none",
} default Keep);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiBackend {
    Win32,
    Cocoa,
    Gtk,
    Kirigami,
}

impl GuiBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Win32 => "win32",
            Self::Cocoa => "cocoa",
            Self::Gtk => "gtk",
            Self::Kirigami => "kirigami",
        }
    }

    /// Parse leniently. Unknown values return the platform default.
    pub fn parse_lenient(value: &str) -> (Self, bool) {
        match value.trim().to_ascii_lowercase().as_str() {
            "win32" | "winui" => (Self::Win32, true),
            "cocoa" => (Self::Cocoa, true),
            "gtk" => (Self::Gtk, true),
            "kirigami" => (Self::Kirigami, true),
            _ => (Self::default(), false),
        }
    }

    /// Whether this backend is meaningful on the current platform.
    pub fn supported_here(&self) -> bool {
        match self {
            GuiBackend::Win32 => cfg!(windows),
            GuiBackend::Cocoa => cfg!(target_os = "macos"),
            GuiBackend::Gtk => cfg!(any(target_os = "macos", target_os = "linux")),
            GuiBackend::Kirigami => cfg!(any(windows, target_os = "macos", target_os = "linux")),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for GuiBackend {
    fn default() -> Self {
        #[cfg(windows)]
        {
            Self::Win32
        }
        #[cfg(target_os = "macos")]
        {
            Self::Cocoa
        }
        #[cfg(target_os = "linux")]
        {
            Self::Gtk
        }
        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        {
            Self::Kirigami
        }
    }
}

impl crate::download::naming::SpacesPolicy {
    pub fn from_setting(value: FilenameSpaces) -> Self {
        match value {
            FilenameSpaces::Keep => Self::Keep,
            FilenameSpaces::Underscore => Self::Underscore,
            FilenameSpaces::Dash => Self::Dash,
            FilenameSpaces::Remove => Self::Remove,
        }
    }
}

// ---- settings ------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub download_dir: String,
    pub yt_dlp: ToolSource,
    pub ffmpeg: ToolSource,
    pub gui_backend: GuiBackend,
    pub gui_download_mode: GuiDownloadMode,
    pub download_logs: bool,
    pub gui_theme: GuiTheme,
    pub strip_video_ids: bool,
    pub sanitize_filenames: bool,
    pub filename_spaces: FilenameSpaces,
    pub playlist_folders: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            download_dir: DEFAULT_DOWNLOAD_DIR.to_string(),
            yt_dlp: ToolSource::Auto,
            ffmpeg: ToolSource::Auto,
            gui_backend: GuiBackend::default(),
            gui_download_mode: GuiDownloadMode::Progress,
            download_logs: true,
            gui_theme: GuiTheme::Light,
            strip_video_ids: true,
            sanitize_filenames: true,
            filename_spaces: FilenameSpaces::Keep,
            playlist_folders: true,
        }
    }
}

impl Settings {
    pub fn spaces_policy(&self) -> crate::download::naming::SpacesPolicy {
        crate::download::naming::SpacesPolicy::from_setting(self.filename_spaces)
    }

    /// Resolve the configured download directory, falling back when unset.
    pub fn resolved_download_dir(&self, fallback: &Path) -> PathBuf {
        if self.download_dir.trim().is_empty() {
            fallback.to_path_buf()
        } else {
            paths::expand_path(&self.download_dir)
        }
    }

    /// Render the canonical, fully-commented TOML file.
    pub fn render(&self) -> String {
        format!(
            "# Quark Downloader configuration\n\
             # Save and restart to apply changes.\n\
             \n\
             # Default folder offered at the output prompt (~ = your home directory)\n\
             download_dir = {dir:?}\n\
             \n\
             # Download naming\n\
             #   strip_video_ids    - drop the trailing \" [VIDEOID]\" from filenames\n\
             #   sanitize_filenames - make filenames mostly ASCII-safe on all platforms\n\
             #   filename_spaces    - keep | underscore | dash | remove\n\
             #   playlist_folders   - put playlist downloads in a folder named after the playlist\n\
             strip_video_ids = {strip}\n\
             sanitize_filenames = {san}\n\
             filename_spaces = {spaces:?}\n\
             playlist_folders = {plf}\n\
             \n\
             # How to locate yt-dlp and ffmpeg: auto | path | bundled\n\
             yt_dlp = {ytdlp:?}\n\
             ffmpeg = {ffmpeg:?}\n\
             \n\
             # GUI backend toolkit: win32 | cocoa | gtk | kirigami\n\
             # Availability and the default depend on the operating system.\n\
             gui_backend = {backend:?}\n\
             \n\
             # GUI download behavior: progress | external_cli\n\
             gui_download_mode = {mode:?}\n\
             \n\
             # Create rotated logs for CLI and GUI downloads\n\
             download_logs = {logs}\n\
             \n\
             # GUI appearance: light | dark\n\
             gui_theme = {theme:?}\n",
            dir = self.download_dir,
            strip = self.strip_video_ids,
            san = self.sanitize_filenames,
            spaces = self.filename_spaces.as_str(),
            plf = self.playlist_folders,
            ytdlp = self.yt_dlp.as_str(),
            ffmpeg = self.ffmpeg.as_str(),
            backend = self.gui_backend.as_str(),
            mode = self.gui_download_mode.as_str(),
            logs = self.download_logs,
            theme = self.gui_theme.as_str(),
        )
    }
}

// ---- raw deserialization (TOML or migrated .conf) ------------------------

#[derive(Default, Deserialize)]
struct RawSettings {
    download_dir: Option<String>,
    yt_dlp: Option<String>,
    ffmpeg: Option<String>,
    gui_backend: Option<String>,
    gui_download_mode: Option<String>,
    download_logs: Option<bool>,
    gui_theme: Option<String>,
    strip_video_ids: Option<bool>,
    sanitize_filenames: Option<bool>,
    filename_spaces: Option<String>,
    playlist_folders: Option<bool>,
}

impl RawSettings {
    /// Convert to validated [`Settings`]. Returns the settings, whether any key
    /// was missing (so the file should be rewritten with defaults), and any
    /// warnings about invalid values.
    fn into_settings(self) -> (Settings, bool, Vec<String>) {
        let d = Settings::default();
        let mut missing = false;
        let mut warnings = Vec::new();

        macro_rules! enum_field {
            ($opt:expr, $ty:ty, $default:expr, $key:literal) => {
                match $opt {
                    Some(v) => {
                        let (parsed, ok) = <$ty>::parse_lenient(&v);
                        if !ok {
                            missing = true;
                            warnings.push(format!(
                                "invalid {} value {:?}, using {}",
                                $key,
                                v,
                                $default.as_str()
                            ));
                        }
                        parsed
                    }
                    None => {
                        missing = true;
                        $default
                    }
                }
            };
        }
        macro_rules! bool_field {
            ($opt:expr, $default:expr) => {
                match $opt {
                    Some(v) => v,
                    None => {
                        missing = true;
                        $default
                    }
                }
            };
        }

        let download_dir = match self.download_dir {
            Some(v) => v,
            None => {
                missing = true;
                d.download_dir.clone()
            }
        };

        let settings = Settings {
            download_dir,
            yt_dlp: enum_field!(self.yt_dlp, ToolSource, d.yt_dlp, "yt_dlp"),
            ffmpeg: enum_field!(self.ffmpeg, ToolSource, d.ffmpeg, "ffmpeg"),
            gui_backend: enum_field!(self.gui_backend, GuiBackend, d.gui_backend, "gui_backend"),
            gui_download_mode: enum_field!(
                self.gui_download_mode,
                GuiDownloadMode,
                d.gui_download_mode,
                "gui_download_mode"
            ),
            download_logs: bool_field!(self.download_logs, d.download_logs),
            gui_theme: enum_field!(self.gui_theme, GuiTheme, d.gui_theme, "gui_theme"),
            strip_video_ids: bool_field!(self.strip_video_ids, d.strip_video_ids),
            sanitize_filenames: bool_field!(self.sanitize_filenames, d.sanitize_filenames),
            filename_spaces: enum_field!(
                self.filename_spaces,
                FilenameSpaces,
                d.filename_spaces,
                "filename_spaces"
            ),
            playlist_folders: bool_field!(self.playlist_folders, d.playlist_folders),
        };
        (settings, missing, warnings)
    }
}

// ---- store ---------------------------------------------------------------

pub fn config_path() -> PathBuf {
    paths::config_dir().join(CONFIG_NAME)
}

pub fn legacy_config_path() -> PathBuf {
    paths::config_dir().join(LEGACY_CONFIG_NAME)
}

/// Load settings, creating defaults on first run and migrating any legacy
/// `.conf`. Missing keys are filled and written back. Warnings about invalid
/// values are printed to stderr unless `quiet`.
pub fn load(quiet: bool) -> std::io::Result<Settings> {
    paths::ensure_config_dir()?;
    let path = config_path();

    if !path.exists() {
        if let Some(migrated) = try_migrate_legacy(quiet)? {
            return Ok(migrated);
        }
        let settings = Settings::default();
        std::fs::write(&path, settings.render())?;
        if !quiet {
            eprintln!("Created config: {}", path.display());
        }
        return Ok(settings);
    }

    let text = std::fs::read_to_string(&path)?;
    let raw: RawSettings = match basic_toml::from_str(&text) {
        Ok(raw) => raw,
        Err(err) => {
            if !quiet {
                eprintln!("Warning: could not parse {}: {err}", path.display());
            }
            RawSettings::default()
        }
    };
    let (settings, missing, warnings) = raw.into_settings();
    if !quiet {
        for w in &warnings {
            eprintln!("Warning: {w} in {}", path.display());
        }
    }
    if missing {
        std::fs::write(&path, settings.render())?;
    }
    Ok(settings)
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    paths::ensure_config_dir()?;
    std::fs::write(config_path(), settings.render())
}

/// If a legacy `.conf` exists, convert it to TOML, back up the original, and
/// return the migrated settings.
fn try_migrate_legacy(quiet: bool) -> std::io::Result<Option<Settings>> {
    let legacy = legacy_config_path();
    if !legacy.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&legacy)?;
    let (settings, warnings) = parse_legacy(&text);
    std::fs::write(config_path(), settings.render())?;
    let backup = legacy.with_extension("conf.bak");
    let _ = std::fs::rename(&legacy, &backup);
    if !quiet {
        eprintln!("Migrated {} -> {}", LEGACY_CONFIG_NAME, CONFIG_NAME);
        for w in &warnings {
            eprintln!("Warning: {w}");
        }
    }
    Ok(Some(settings))
}

/// Parse the legacy key=value format into settings (+ warnings).
pub fn parse_legacy(text: &str) -> (Settings, Vec<String>) {
    let mut map: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (key, value) = line.split_once('=').unwrap();
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().to_string();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        map.insert(key, value);
    }

    let raw = RawSettings {
        download_dir: map.get("download_dir").cloned(),
        yt_dlp: map.get("yt_dlp").cloned(),
        ffmpeg: map.get("ffmpeg").cloned(),
        gui_backend: map.get("gui_backend").cloned(),
        gui_download_mode: map.get("gui_download_mode").cloned(),
        download_logs: map.get("download_logs").map(|v| parse_bool(v, true)),
        gui_theme: map.get("gui_theme").cloned(),
        strip_video_ids: map.get("strip_video_ids").map(|v| parse_bool(v, true)),
        sanitize_filenames: map.get("sanitize_filenames").map(|v| parse_bool(v, true)),
        filename_spaces: map.get("filename_spaces").cloned(),
        playlist_folders: map.get("playlist_folders").map(|v| parse_bool(v, true)),
    };
    let (settings, _missing, warnings) = raw.into_settings();
    (settings, warnings)
}

/// Lenient boolean parse matching the legacy format.
pub fn parse_bool(value: &str, default: bool) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_aliases_and_defaults() {
        assert_eq!(
            GuiDownloadMode::parse_lenient("cli").0,
            GuiDownloadMode::ExternalCli
        );
        assert_eq!(
            GuiDownloadMode::parse_lenient("terminal").0,
            GuiDownloadMode::ExternalCli
        );
        assert_eq!(
            FilenameSpaces::parse_lenient("hyphen").0,
            FilenameSpaces::Dash
        );
        let (val, ok) = ToolSource::parse_lenient("nonsense");
        assert_eq!(val, ToolSource::Auto);
        assert!(!ok);
    }

    #[test]
    fn gui_backend_default_matches_platform() {
        #[cfg(windows)]
        assert_eq!(GuiBackend::default(), GuiBackend::Win32);
        #[cfg(target_os = "macos")]
        assert_eq!(GuiBackend::default(), GuiBackend::Cocoa);
        #[cfg(target_os = "linux")]
        assert_eq!(GuiBackend::default(), GuiBackend::Gtk);
    }

    #[test]
    fn legacy_migration_roundtrip() {
        let conf = "download_dir = ~/Movies\nyt_dlp = bundled\nfilename_spaces = underscore\ndownload_logs = no\ngui_download_mode = terminal\n";
        let (settings, _w) = parse_legacy(conf);
        assert_eq!(settings.download_dir, "~/Movies");
        assert_eq!(settings.yt_dlp, ToolSource::Bundled);
        assert_eq!(settings.filename_spaces, FilenameSpaces::Underscore);
        assert!(!settings.download_logs);
        assert_eq!(settings.gui_download_mode, GuiDownloadMode::ExternalCli);
        // Defaults fill the rest.
        assert_eq!(settings.gui_backend, GuiBackend::default());
        assert!(settings.playlist_folders);
    }

    #[test]
    fn toml_roundtrip_through_render() {
        let s = Settings {
            gui_backend: GuiBackend::Gtk,
            gui_theme: GuiTheme::Dark,
            filename_spaces: FilenameSpaces::Dash,
            ..Settings::default()
        };
        let rendered = s.render();
        let raw: RawSettings = basic_toml::from_str(&rendered).unwrap();
        let (parsed, missing, warnings) = raw.into_settings();
        assert_eq!(parsed, s);
        assert!(!missing);
        assert!(warnings.is_empty());
    }
}
