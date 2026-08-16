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
