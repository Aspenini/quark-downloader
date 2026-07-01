//! Filesystem locations: home, config dir, app dir, tools dir, and the
//! platform default downloads directory.

use std::env;
use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "quark-downloader";

/// The user's home directory (USERPROFILE on Windows, HOME elsewhere).
pub fn user_home() -> PathBuf {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Expand a leading `~` / `~/` against the home directory, then normalize.
pub fn expand_path(input: &str) -> PathBuf {
    let expanded = if input == "~" {
        user_home()
    } else if let Some(rest) = input.strip_prefix("~/") {
        user_home().join(rest)
    } else {
        PathBuf::from(input)
    };
    // std has no expand of `.`/`..` without touching the FS; absolutize lightly.
    if expanded.is_absolute() {
        expanded
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(&expanded))
            .unwrap_or(expanded)
    }
}

/// Per-user config directory for this app's settings, logs, and bundled tools.
pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let base = env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(user_home);
        base.join(APP_DIR_NAME)
    }
    #[cfg(not(windows))]
    {
        let base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| user_home().join(".config"));
        base.join(APP_DIR_NAME)
    }
}

pub fn ensure_config_dir() -> std::io::Result<PathBuf> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory containing the running executable. Falls back to the current
/// directory when it can't be determined or points at a Cargo build cache.
pub fn app_dir() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let s = parent.to_string_lossy().replace('\\', "/");
            if s.contains("/target/debug") || s.contains("/target/release") {
                return env::current_dir().unwrap_or_else(|_| parent.to_path_buf());
            }
            return parent.to_path_buf();
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Where downloaded yt-dlp/ffmpeg live. Inside a macOS `.app` bundle the
/// install dir is read-only, so redirect to the config dir.
pub fn tools_dir() -> PathBuf {
    let app = app_dir();
    #[cfg(target_os = "macos")]
    {
        if app.to_string_lossy().contains(".app/Contents/MacOS") {
            return config_dir().join("tools");
        }
    }
    app.join("tools")
}

/// Default downloads directory: XDG user-dirs on Linux, else `~/Downloads`.
pub fn default_downloads_dir() -> PathBuf {
    #[cfg(not(windows))]
    {
        if let Some(dir) = xdg_download_dir() {
            return dir;
        }
    }
    user_home().join("Downloads")
}

/// Parse `XDG_DOWNLOAD_DIR` from `~/.config/user-dirs.dirs` if present.
pub fn xdg_download_dir() -> Option<PathBuf> {
    let home = user_home();
    let config = home.join(".config").join("user-dirs.dirs");
    let text = std::fs::read_to_string(&config).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("XDG_DOWNLOAD_DIR=") {
            let rest = rest.trim().trim_matches('"');
            let replaced = rest.replace("$HOME", &home.to_string_lossy());
            return Some(absolutize(Path::new(&replaced)));
        }
    }
    None
}

fn absolutize(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        env::current_dir()
            .map(|c| c.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}
