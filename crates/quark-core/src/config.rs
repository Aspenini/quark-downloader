use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::filename::SpacesPolicy;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolSource {
    #[default]
    Auto,
    Path,
    Bundled,
}

impl ToolSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Path => "path",
            Self::Bundled => "bundled",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GuiDownloadMode {
    #[default]
    Progress,
    ExternalCli,
}

impl GuiDownloadMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::ExternalCli => "external_cli",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GuiTheme {
    #[default]
    System,
    Light,
    Dark,
}

impl GuiTheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Force light/dark, or the desktop preference when `system`.
    pub fn resolve(self) -> Self {
        match self {
            Self::System => {
                if system_prefers_dark() {
                    Self::Dark
                } else {
                    Self::Light
                }
            }
            other => other,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilenameSpaces {
    #[default]
    Keep,
    Underscore,
    Dash,
    Remove,
}

impl FilenameSpaces {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Underscore => "underscore",
            Self::Dash => "dash",
            Self::Remove => "remove",
        }
    }

    pub fn to_policy(self) -> SpacesPolicy {
        match self {
            Self::Keep => SpacesPolicy::Keep,
            Self::Underscore => SpacesPolicy::Underscore,
            Self::Dash => SpacesPolicy::Dash,
            Self::Remove => SpacesPolicy::Remove,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GuiFrontend {
    #[default]
    Auto,
    Win32,
    Appkit,
    Qt,
}

impl GuiFrontend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Win32 => "win32",
            Self::Appkit => "appkit",
            Self::Qt => "qt",
        }
    }

    pub fn id(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Win32 => Some("win32"),
            Self::Appkit => Some("appkit"),
            Self::Qt => Some("qt"),
        }
    }

    /// Windows in-process dialogs when Auto or an explicit Win32 pick.
    pub fn uses_inprocess_win32(self) -> bool {
        if !quark_platform::uses_inprocess_gui() {
            return false;
        }
        matches!(self, Self::Auto | Self::Win32)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub download_dir: String,
    pub yt_dlp: ToolSource,
    pub ffmpeg: ToolSource,
    pub gui_download_mode: GuiDownloadMode,
    pub download_logs: bool,
    pub gui_theme: GuiTheme,
    pub strip_video_ids: bool,
    pub sanitize_filenames: bool,
    pub filename_spaces: FilenameSpaces,
    pub playlist_folders: bool,
    pub gui_frontend: GuiFrontend,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: "~/Downloads".into(),
            yt_dlp: ToolSource::Auto,
            ffmpeg: ToolSource::Auto,
            gui_download_mode: GuiDownloadMode::Progress,
            download_logs: true,
            gui_theme: GuiTheme::System,
            strip_video_ids: true,
            sanitize_filenames: true,
            filename_spaces: FilenameSpaces::Keep,
            playlist_folders: true,
            gui_frontend: GuiFrontend::Auto,
        }
    }
}

impl Settings {
    pub fn yt_dlp_source(&self) -> ToolSource {
        if quark_platform::allows_bundled_tools() {
            self.yt_dlp
        } else {
            ToolSource::Path
        }
    }

    pub fn ffmpeg_source(&self) -> ToolSource {
        if quark_platform::allows_bundled_tools() {
            self.ffmpeg
        } else {
            ToolSource::Path
        }
    }

    pub fn download_dir_expanded(&self, fallback: &Path) -> PathBuf {
        if self.download_dir.is_empty() {
            return fallback.to_path_buf();
        }
        expand_path(&self.download_dir)
    }
}

#[derive(Debug)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        Self(err.to_string())
    }
}

pub const CONFIG_NAME: &str = "quark-downloader.conf";
pub const APP_NAME: &str = "quark-downloader";

fn public_keys() -> &'static [&'static str] {
    if quark_platform::allows_bundled_tools() {
        &[
            "download_dir",
            "yt_dlp",
            "ffmpeg",
            "gui_download_mode",
            "download_logs",
            "gui_theme",
            "strip_video_ids",
            "sanitize_filenames",
            "filename_spaces",
            "playlist_folders",
            "gui_frontend",
        ]
    } else {
        &[
            "download_dir",
            "gui_download_mode",
            "download_logs",
            "gui_theme",
            "strip_video_ids",
            "sanitize_filenames",
            "filename_spaces",
            "playlist_folders",
            "gui_frontend",
        ]
    }
}

pub fn user_home() -> PathBuf {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        PathBuf::from(home)
    } else {
        PathBuf::from(".")
    }
}

pub fn expand_path(path: &str) -> PathBuf {
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        user_home().join(rest)
    } else if path == "~" {
        user_home()
    } else {
        PathBuf::from(path)
    };
    let resolved = fs::canonicalize(&expanded).unwrap_or_else(|_| {
        if expanded.is_absolute() {
            expanded
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(expanded)
        }
    });
    quark_platform::simplify_path(resolved)
}

pub fn app_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let s = parent.to_string_lossy().replace('\\', "/");
        if s.contains("/target/debug") || s.contains("/target/release") {
            return std::env::current_dir().unwrap_or_else(|_| parent.to_path_buf());
        }
        return parent.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn config_dir() -> PathBuf {
    quark_platform::config_dir(APP_NAME)
}

pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_NAME)
}

pub fn ensure_config_dir() -> Result<PathBuf, ConfigError> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|ex| {
        ConfigError(format!(
            "Cannot create config directory:\n  {}\n{}\nCheck that HOME is set and you own that path. Do not use sudo.",
            dir.display(),
            ex
        ))
    })?;
    Ok(dir)
}

pub fn load(quiet: bool) -> Result<Settings, ConfigError> {
    ensure_config_dir()?;
    let path = config_path();
    if !path.exists() {
        create_default(&path, !quiet)?;
    }
    let (settings, keys) = parse_file_with_keys(&path, quiet)?;
    append_missing_defaults(&path, &settings, &keys)?;
    Ok(settings)
}

pub fn save(settings: &Settings) -> Result<(), ConfigError> {
    ensure_config_dir()?;
    fs::write(config_path(), render(settings))?;
    Ok(())
}

pub fn create_default(path: &Path, announce: bool) -> Result<(), ConfigError> {
    fs::write(path, render(&Settings::default()))?;
    if announce {
        println!("Created config: {}", path.display());
    }
    Ok(())
}

pub fn parse_file(path: &Path, quiet: bool) -> Result<Settings, ConfigError> {
    Ok(parse_file_with_keys(path, quiet)?.0)
}

pub fn parse_file_with_keys(
    path: &Path,
    quiet: bool,
) -> Result<(Settings, Vec<String>), ConfigError> {
    let file = File::open(path)?;
    let mut settings = Settings::default();
    let mut keys = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        let normalized = key.to_ascii_lowercase();
        keys.push(normalized.clone());
        match normalized.as_str() {
            "download_dir" => settings.download_dir = value.to_string(),
            "yt_dlp" => {
                if quark_platform::allows_bundled_tools() {
                    settings.yt_dlp = parse_tool_source(value, "yt_dlp", quiet);
                }
            }
            "ffmpeg" => {
                if quark_platform::allows_bundled_tools() {
                    settings.ffmpeg = parse_tool_source(value, "ffmpeg", quiet);
                }
            }
            "gui_download_mode" => {
                settings.gui_download_mode = parse_gui_download_mode(value, quiet);
            }
            "download_logs" => {
                settings.download_logs = parse_bool(value, "download_logs", true, quiet);
            }
            "gui_theme" => settings.gui_theme = parse_gui_theme(value, quiet),
            "strip_video_ids" => {
                settings.strip_video_ids = parse_bool(value, "strip_video_ids", true, quiet);
            }
            "sanitize_filenames" => {
                settings.sanitize_filenames = parse_bool(value, "sanitize_filenames", true, quiet);
            }
            "filename_spaces" => {
                settings.filename_spaces = parse_filename_spaces(value, quiet);
            }
            "playlist_folders" => {
                settings.playlist_folders = parse_bool(value, "playlist_folders", true, quiet);
            }
            "gui_frontend" => {
                if quark_platform::persist_gui_frontend() {
                    settings.gui_frontend = parse_gui_frontend(value, quiet);
                }
            }
            _ => {
                if !quiet {
                    println!("Warning: unknown config key {key:?} in {}", path.display());
                }
            }
        }
    }
    Ok((settings, keys))
}

pub fn append_missing_defaults(
    path: &Path,
    settings: &Settings,
    existing_keys: &[String],
) -> Result<(), ConfigError> {
    let missing: Vec<&str> = public_keys()
        .iter()
        .copied()
        .filter(|key| !existing_keys.iter().any(|k| k == key))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new().append(true).open(path)?;
    writeln!(file)?;
    writeln!(file, "# Added by quark-downloader")?;
    for key in missing {
        writeln!(file, "{key} = {}", config_value(settings, key))?;
    }
    Ok(())
}

pub fn config_value(settings: &Settings, key: &str) -> String {
    match key {
        "download_dir" => settings.download_dir.clone(),
        "yt_dlp" => {
            if quark_platform::allows_bundled_tools() {
                settings.yt_dlp.as_str().into()
            } else {
                "path".into()
            }
        }
        "ffmpeg" => {
            if quark_platform::allows_bundled_tools() {
                settings.ffmpeg.as_str().into()
            } else {
                "path".into()
            }
        }
        "gui_download_mode" => settings.gui_download_mode.as_str().into(),
        "download_logs" => settings.download_logs.to_string(),
        "gui_theme" => settings.gui_theme.as_str().into(),
        "strip_video_ids" => settings.strip_video_ids.to_string(),
        "sanitize_filenames" => settings.sanitize_filenames.to_string(),
        "filename_spaces" => settings.filename_spaces.as_str().into(),
        "playlist_folders" => settings.playlist_folders.to_string(),
        "gui_frontend" => settings.gui_frontend.as_str().into(),
        _ => String::new(),
    }
}

pub fn parse_tool_source(value: &str, key: &str, quiet: bool) -> ToolSource {
    match value.to_ascii_lowercase().as_str() {
        "path" => ToolSource::Path,
        "bundled" => ToolSource::Bundled,
        "auto" => ToolSource::Auto,
        _ => {
            if !quiet {
                println!("Warning: invalid {key} value {value:?}, using auto");
            }
            ToolSource::Auto
        }
    }
}

pub fn parse_gui_download_mode(value: &str, quiet: bool) -> GuiDownloadMode {
    match value.to_ascii_lowercase().as_str() {
        "external_cli" | "cli" | "terminal" => GuiDownloadMode::ExternalCli,
        "progress" | "gui" => GuiDownloadMode::Progress,
        _ => {
            if !quiet {
                println!("Warning: invalid gui_download_mode value {value:?}, using progress");
            }
            GuiDownloadMode::Progress
        }
    }
}

pub fn parse_filename_spaces(value: &str, quiet: bool) -> FilenameSpaces {
    match value.to_ascii_lowercase().as_str() {
        "keep" | "space" | "spaces" => FilenameSpaces::Keep,
        "underscore" | "underscores" => FilenameSpaces::Underscore,
        "dash" | "dashes" | "hyphen" => FilenameSpaces::Dash,
        "remove" | "none" => FilenameSpaces::Remove,
        _ => {
            if !quiet {
                println!("Warning: invalid filename_spaces value {value:?}, using keep");
            }
            FilenameSpaces::Keep
        }
    }
}

pub fn parse_gui_theme(value: &str, quiet: bool) -> GuiTheme {
    match value.to_ascii_lowercase().as_str() {
        "system" | "auto" => GuiTheme::System,
        "dark" => GuiTheme::Dark,
        "light" => GuiTheme::Light,
        _ => {
            if !quiet {
                println!("Warning: invalid gui_theme value {value:?}, using system");
            }
            GuiTheme::System
        }
    }
}

/// Desktop color-scheme preference (COSMIC, then Plasma, then GTK).
pub fn system_prefers_dark() -> bool {
    cosmic_is_dark()
        .or_else(kde_prefers_dark)
        .or_else(gtk_prefers_dark)
        .or_else(macos_prefers_dark)
        .unwrap_or(false)
}

fn cosmic_is_dark() -> Option<bool> {
    let path = xdg_config_home().join("cosmic/com.system76.CosmicTheme.Mode/v1/is_dark");
    parse_boolish(&fs::read_to_string(path).ok()?)
}

fn kde_prefers_dark() -> Option<bool> {
    parse_kdeglobals(&fs::read_to_string(xdg_config_home().join("kdeglobals")).ok()?)
}

fn gtk_prefers_dark() -> Option<bool> {
    for name in ["gtk-4.0/settings.ini", "gtk-3.0/settings.ini"] {
        if let Ok(text) = fs::read_to_string(xdg_config_home().join(name))
            && let Some(v) = parse_gtk_settings(&text)
        {
            return Some(v);
        }
    }
    None
}

fn macos_prefers_dark() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
            .ok()?;
        if !out.status.success() {
            return Some(false);
        }
        return Some(
            String::from_utf8_lossy(&out.stdout)
                .to_ascii_lowercase()
                .contains("dark"),
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn xdg_config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| user_home().join(".config"))
}

fn parse_boolish(text: &str) -> Option<bool> {
    match text
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
        .as_str()
    {
        "true" | "1" | "yes" | "on" | "dark" => Some(true),
        "false" | "0" | "no" | "off" | "light" => Some(false),
        _ => None,
    }
}

fn parse_kdeglobals(text: &str) -> Option<bool> {
    let mut color_scheme = String::new();
    let mut look_and_feel = String::new();
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("ColorScheme=") {
            color_scheme = v.trim().to_ascii_lowercase();
        }
        if let Some(v) = line.strip_prefix("LookAndFeelPackage=") {
            look_and_feel = v.trim().to_ascii_lowercase();
        }
    }
    let blob = format!("{color_scheme} {look_and_feel}");
    if blob.contains("dark") {
        Some(true)
    } else if !color_scheme.is_empty() || !look_and_feel.is_empty() {
        Some(false)
    } else {
        None
    }
}

fn parse_gtk_settings(text: &str) -> Option<bool> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("gtk-application-prefer-dark-theme") {
            return parse_boolish(rest.trim().trim_start_matches('=').trim());
        }
        if let Some(rest) = line.strip_prefix("gtk-interface-color-scheme") {
            let v = rest
                .trim()
                .trim_start_matches('=')
                .trim()
                .to_ascii_lowercase();
            if v.contains("dark") {
                return Some(true);
            }
            if v.contains("light") {
                return Some(false);
            }
        }
    }
    None
}

pub fn parse_gui_frontend(value: &str, quiet: bool) -> GuiFrontend {
    match value.to_ascii_lowercase().as_str() {
        "win32" => GuiFrontend::Win32,
        "appkit" => GuiFrontend::Appkit,
        "qt" => GuiFrontend::Qt,
        "auto" => GuiFrontend::Auto,
        _ => {
            if !quiet {
                println!("Warning: invalid gui_frontend value {value:?}, using auto");
            }
            GuiFrontend::Auto
        }
    }
}

pub fn parse_bool(value: &str, key: &str, default: bool, quiet: bool) -> bool {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => {
            if !quiet {
                println!("Warning: invalid {key} value {value:?}, using {default}");
            }
            default
        }
    }
}

pub fn render(settings: &Settings) -> String {
    let mut lines = vec![
        "# Quark Downloader configuration".into(),
        "# Save and restart to apply changes.".into(),
        String::new(),
        "# Default folder offered at the output prompt (~ = your home directory)".into(),
        format!("download_dir = {}", settings.download_dir),
        String::new(),
        "# Download naming".into(),
        "#   strip_video_ids    - drop the trailing \" [VIDEOID]\" from filenames".into(),
        "#   sanitize_filenames - make filenames mostly ASCII-safe on all platforms".into(),
        "#   filename_spaces    - keep | underscore | dash | remove".into(),
        "#   playlist_folders   - put playlist downloads in a folder named after the playlist"
            .into(),
        format!("strip_video_ids = {}", settings.strip_video_ids),
        format!("sanitize_filenames = {}", settings.sanitize_filenames),
        format!("filename_spaces = {}", settings.filename_spaces.as_str()),
        format!("playlist_folders = {}", settings.playlist_folders),
        String::new(),
    ];

    if quark_platform::allows_bundled_tools() {
        lines.extend([
            "# How to locate yt-dlp and ffmpeg".into(),
            "#   auto    - PATH first, then bundled tools beside the app".into(),
            "#   path    - PATH only".into(),
            "#   bundled - bundled tools beside the app only (may download if missing)".into(),
            format!("yt_dlp = {}", settings.yt_dlp.as_str()),
            format!("ffmpeg = {}", settings.ffmpeg.as_str()),
            String::new(),
        ]);
    } else {
        lines.extend([
            "# yt-dlp and ffmpeg are always resolved from PATH.".into(),
            "#   macOS: brew install yt-dlp ffmpeg".into(),
            "#   Linux: package manager or pipx install yt-dlp; install ffmpeg via apt/dnf/etc."
                .into(),
            String::new(),
        ]);
    }

    lines.extend([
        "# GUI download behavior".into(),
        "#   progress     - show the GUI progress dialog and completion popup".into(),
        "#   external_cli - open the CLI window after Download and close the GUI".into(),
        format!(
            "gui_download_mode = {}",
            settings.gui_download_mode.as_str()
        ),
        String::new(),
        "# Create rotated logs for CLI and GUI downloads".into(),
        format!("download_logs = {}", settings.download_logs),
        String::new(),
        "# GUI appearance".into(),
        "#   system - follow the desktop (Plasma, COSMIC, or macOS appearance)".into(),
        "#   light  - force light controls".into(),
        "#   dark   - force dark controls".into(),
        format!("gui_theme = {}", settings.gui_theme.as_str()),
        String::new(),
    ]);

    if quark_platform::persist_gui_frontend() {
        lines.extend([
            "# Which GUI frontend to use".into(),
            "#   auto     - Qt on Linux, the native frontend elsewhere".into(),
            "#   qt       - Qt 6 (uses CuteCosmic when installed on COSMIC)".into(),
            "#   win32    - in-process Win32 (Windows)".into(),
            "#   appkit   - AppKit helper (macOS)".into(),
            format!("gui_frontend = {}", settings.gui_frontend.as_str()),
            String::new(),
        ]);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_conf(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("quark-config-{name}-{nanos}.conf"))
    }

    #[test]
    fn expand_path_drops_windows_extended_prefix() {
        let path = expand_path(".");
        let s = path.to_string_lossy();
        assert!(
            !s.starts_with(r"\\?\"),
            "canonical path should be display-friendly, got {s}"
        );
        assert!(path.is_absolute(), "{s}");
    }

    #[test]
    fn parses_all_config_fields() {
        let path = temp_conf("all");
        fs::write(
            &path,
            "download_dir = ~/Media\nyt_dlp = path\nffmpeg = bundled\ngui_download_mode = external_cli\ndownload_logs = off\ngui_theme = dark\nstrip_video_ids = false\nsanitize_filenames = false\nfilename_spaces = underscore\nplaylist_folders = false\n",
        )
        .unwrap();
        let settings = parse_file(&path, true).unwrap();
        assert_eq!(settings.download_dir, "~/Media");
        if quark_platform::allows_bundled_tools() {
            assert_eq!(settings.yt_dlp, ToolSource::Path);
            assert_eq!(settings.ffmpeg, ToolSource::Bundled);
        } else {
            assert_eq!(settings.yt_dlp, ToolSource::Auto);
            assert_eq!(settings.ffmpeg, ToolSource::Auto);
            assert_eq!(settings.yt_dlp_source(), ToolSource::Path);
            assert_eq!(settings.ffmpeg_source(), ToolSource::Path);
        }
        assert_eq!(settings.gui_download_mode, GuiDownloadMode::ExternalCli);
        assert!(!settings.download_logs);
        assert_eq!(settings.gui_theme, GuiTheme::Dark);
        assert!(!settings.strip_video_ids);
        assert!(!settings.sanitize_filenames);
        assert_eq!(settings.filename_spaces, FilenameSpaces::Underscore);
        assert!(!settings.playlist_folders);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn falls_back_for_invalid_values() {
        let path = temp_conf("invalid");
        fs::write(
            &path,
            "yt_dlp = nope\nffmpeg = wrong\ngui_download_mode = mystery\ndownload_logs = maybe\ngui_theme = neon\nstrip_video_ids = maybe\nsanitize_filenames = perhaps\nfilename_spaces = tabs\nplaylist_folders = sometimes\ngui_frontend = neon\n",
        )
        .unwrap();
        let settings = parse_file(&path, true).unwrap();
        assert_eq!(settings.yt_dlp, ToolSource::Auto);
        assert_eq!(settings.ffmpeg, ToolSource::Auto);
        assert_eq!(settings.gui_download_mode, GuiDownloadMode::Progress);
        assert!(settings.download_logs);
        assert_eq!(settings.gui_theme, GuiTheme::System);
        assert!(settings.strip_video_ids);
        assert!(settings.sanitize_filenames);
        assert_eq!(settings.filename_spaces, FilenameSpaces::Keep);
        assert!(settings.playlist_folders);
        assert_eq!(settings.gui_frontend, GuiFrontend::Auto);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn renders_public_settings() {
        let settings = Settings {
            download_dir: "D:/Downloads".into(),
            yt_dlp: ToolSource::Bundled,
            ffmpeg: ToolSource::Path,
            gui_download_mode: GuiDownloadMode::ExternalCli,
            download_logs: false,
            gui_theme: GuiTheme::Dark,
            strip_video_ids: false,
            sanitize_filenames: false,
            filename_spaces: FilenameSpaces::Dash,
            playlist_folders: false,
            gui_frontend: GuiFrontend::Qt,
        };
        let rendered = render(&settings);
        assert!(rendered.contains("download_dir = D:/Downloads"));
        if quark_platform::allows_bundled_tools() {
            assert!(rendered.contains("yt_dlp = bundled"));
            assert!(rendered.contains("ffmpeg = path"));
        } else {
            assert!(!rendered.contains("yt_dlp ="));
            assert!(!rendered.contains("ffmpeg ="));
            assert!(rendered.contains("always resolved from PATH"));
        }
        assert!(rendered.contains("gui_frontend = qt"));
        assert!(rendered.contains("gui_download_mode = external_cli"));
        assert!(rendered.contains("download_logs = false"));
        assert!(rendered.contains("gui_theme = dark"));
        assert!(rendered.contains("strip_video_ids = false"));
        assert!(rendered.contains("sanitize_filenames = false"));
        assert!(rendered.contains("filename_spaces = dash"));
        assert!(rendered.contains("playlist_folders = false"));
    }

    #[test]
    fn appends_missing_public_settings() {
        let path = temp_conf("migrate");
        fs::write(
            &path,
            "download_dir = ~/Downloads\nyt_dlp = auto\nffmpeg = auto\n",
        )
        .unwrap();
        let (settings, keys) = parse_file_with_keys(&path, true).unwrap();
        append_missing_defaults(&path, &settings, &keys).unwrap();
        let migrated = fs::read_to_string(&path).unwrap();
        assert!(migrated.contains("download_dir = ~/Downloads"));
        assert!(migrated.contains("gui_download_mode = progress"));
        assert!(migrated.contains("download_logs = true"));
        assert!(migrated.contains("gui_theme = system"));
        assert!(migrated.contains("strip_video_ids = true"));
        assert!(migrated.contains("sanitize_filenames = true"));
        assert!(migrated.contains("filename_spaces = keep"));
        assert!(migrated.contains("playlist_folders = true"));
        if quark_platform::persist_gui_frontend() {
            assert!(migrated.contains("gui_frontend = auto"));
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn parses_system_theme() {
        assert_eq!(parse_gui_theme("system", true), GuiTheme::System);
        assert_eq!(parse_gui_theme("auto", true), GuiTheme::System);
        assert_eq!(parse_gui_theme("dark", true), GuiTheme::Dark);
        assert_eq!(GuiTheme::Light.resolve(), GuiTheme::Light);
        assert_eq!(GuiTheme::Dark.resolve(), GuiTheme::Dark);
    }

    #[test]
    fn detects_desktop_dark_from_config_text() {
        assert_eq!(parse_boolish("true"), Some(true));
        assert_eq!(parse_boolish("false\n"), Some(false));
        assert_eq!(
            parse_kdeglobals("[General]\nColorScheme=BreezeDark\n"),
            Some(true)
        );
        assert_eq!(
            parse_kdeglobals("[KDE]\nLookAndFeelPackage=org.kde.breeze.desktop\n"),
            Some(false)
        );
        assert_eq!(
            parse_gtk_settings("[Settings]\ngtk-application-prefer-dark-theme=1\n"),
            Some(true)
        );
        assert_eq!(
            parse_gtk_settings("[Settings]\ngtk-interface-color-scheme=prefer-light\n"),
            Some(false)
        );
    }
}
