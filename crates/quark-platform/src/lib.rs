//! OS-specific HTTP, process, and path helpers.
//!
//! `quark-core` stays free of `windows-sys` and raw FFI by calling this crate.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{HiddenProcess, read_handle_lines, spawn_cmd_start_wait};

pub fn fetch_body(url: &str, user_agent: &str) -> io::Result<String> {
    #[cfg(windows)]
    {
        windows::fetch_body(url, user_agent)
    }
    #[cfg(not(windows))]
    {
        unix::fetch_body(url, user_agent)
    }
}

pub fn download_file(url: &str, dest: &Path, user_agent: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        windows::download_file(url, dest, user_agent)
    }
    #[cfg(not(windows))]
    {
        unix::download_file(url, dest, user_agent)
    }
}

pub fn which(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        windows::which(name).or_else(|| which_path(name))
    }
    #[cfg(not(windows))]
    {
        which_path(name)
    }
}

fn which_path(name: &str) -> Option<PathBuf> {
    let name_path = Path::new(name);
    if name_path.components().count() > 1 {
        return existing_exe(name_path);
    }
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        if let Some(found) = existing_exe(&dir.join(name)) {
            return Some(found);
        }
        #[cfg(windows)]
        {
            if !name.ends_with(".exe")
                && let Some(found) = existing_exe(&dir.join(format!("{name}.exe")))
            {
                return Some(found);
            }
        }
    }
    None
}

fn existing_exe(path: &Path) -> Option<PathBuf> {
    path.is_file().then(|| path.to_path_buf())
}

/// Windows can ship bundled yt-dlp/ffmpeg; other hosts always use PATH.
pub fn allows_bundled_tools() -> bool {
    cfg!(windows)
}

pub fn uses_inprocess_gui() -> bool {
    cfg!(windows)
}

pub fn prefers_appkit() -> bool {
    cfg!(target_os = "macos")
}

pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

pub fn pause_before_exit() -> bool {
    cfg!(windows)
}

pub fn hide_console() -> bool {
    cfg!(windows) && std::env::var_os("QUARK_GUI").as_deref() == Some(std::ffi::OsStr::new("1"))
}

pub fn cli_name() -> &'static str {
    if cfg!(windows) {
        "quark-downloader.exe"
    } else {
        "quark-downloader"
    }
}

pub fn exe(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Drop the Windows `\\?\` extended-length prefix so paths look like `C:\Users\...`.
///
/// Rust's `fs::canonicalize` uses `GetFinalPathNameByHandleW`, which returns
/// verbatim paths (`\\?\C:\...`, `\\?\UNC\server\share`). Those are valid for
/// Win32 I/O but look wrong in prompts, dialogs, and logs. Volume GUID and
/// other device-namespace prefixes are left alone.
pub fn simplify_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(strip_extended_prefix(&path.as_ref().to_string_lossy()))
}

fn strip_extended_prefix(s: &str) -> String {
    let rest = s.strip_prefix(r"\\?\").or_else(|| s.strip_prefix("//?/"));
    let Some(rest) = rest else {
        return s.to_string();
    };

    if let Some(unc) = rest
        .strip_prefix(r"UNC\")
        .or_else(|| rest.strip_prefix("UNC/"))
    {
        return format!(r"\\{unc}");
    }

    let bytes = rest.as_bytes();
    if bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes.len() == 2 || bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return rest.to_string();
    }

    s.to_string()
}

pub fn config_dir(app: &str) -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join(app);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        });
    base.join(app)
}

pub fn enable_virtual_terminal() {
    #[cfg(windows)]
    {
        use std::sync::atomic::AtomicU8;
        static TRIED: AtomicU8 = AtomicU8::new(0);
        windows::enable_virtual_terminal(&TRIED);
    }
}

pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        unix::is_root()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn local_offset_secs() -> i64 {
    #[cfg(windows)]
    {
        windows::local_offset_secs()
    }
    #[cfg(unix)]
    {
        unix::local_offset_secs()
    }
    #[cfg(not(any(windows, unix)))]
    {
        0
    }
}

pub fn sha256_hex(path: &Path) -> io::Result<String> {
    #[cfg(windows)]
    {
        windows::sha256_hex(path)
    }
    #[cfg(not(windows))]
    {
        sha256_hex_command(path)
    }
}

#[cfg(not(windows))]
fn sha256_hex_command(path: &Path) -> io::Result<String> {
    use std::process::Command;
    let output = Command::new("sha256sum").arg(path).output().or_else(|_| {
        Command::new("shasum")
            .args(["-a", "256", &path.to_string_lossy()])
            .output()
    })?;
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| io::Error::other("could not hash file"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_name_matches_host() {
        if allows_bundled_tools() {
            assert!(cli_name().ends_with(".exe"));
            assert_eq!(exe("ffmpeg"), "ffmpeg.exe");
        } else {
            assert_eq!(cli_name(), "quark-downloader");
            assert_eq!(exe("ffmpeg"), "ffmpeg");
        }
    }

    #[test]
    fn config_dir_is_under_app_name() {
        let dir = config_dir("quark-downloader");
        assert!(dir.ends_with("quark-downloader"));
    }

    #[test]
    fn simplify_path_strips_verbatim_disk() {
        assert_eq!(
            strip_extended_prefix(r"\\?\C:\Users\bob\Downloads"),
            r"C:\Users\bob\Downloads"
        );
        assert_eq!(strip_extended_prefix(r"\\?\c:\Users\bob"), r"c:\Users\bob");
        assert_eq!(strip_extended_prefix(r"\\?\D:"), "D:");
        assert_eq!(strip_extended_prefix("//?/C:/Users/bob"), "C:/Users/bob");
    }

    #[test]
    fn simplify_path_strips_verbatim_unc() {
        assert_eq!(
            strip_extended_prefix(r"\\?\UNC\server\share\dir"),
            r"\\server\share\dir"
        );
        assert_eq!(
            strip_extended_prefix("//?/UNC/server/share"),
            r"\\server/share"
        );
    }

    #[test]
    fn simplify_path_leaves_normal_and_device_paths() {
        assert_eq!(
            strip_extended_prefix(r"C:\Users\bob\Downloads"),
            r"C:\Users\bob\Downloads"
        );
        assert_eq!(
            strip_extended_prefix("/home/bob/Downloads"),
            "/home/bob/Downloads"
        );
        assert_eq!(
            strip_extended_prefix(r"\\server\share\dir"),
            r"\\server\share\dir"
        );
        assert_eq!(
            strip_extended_prefix(r"\\?\Volume{12345678-1234-1234-1234-123456789abc}\foo"),
            r"\\?\Volume{12345678-1234-1234-1234-123456789abc}\foo"
        );
        assert_eq!(
            strip_extended_prefix(r"\\?\GLOBALROOT\Device\HarddiskVolume1"),
            r"\\?\GLOBALROOT\Device\HarddiskVolume1"
        );
    }
}
