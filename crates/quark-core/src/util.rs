//! Small cross-platform helpers.

use std::path::{Path, PathBuf};

/// Locate an executable on `PATH` (mirrors `Process.find_executable`).
/// Accepts an absolute/relative path directly. On Windows, tries `PATHEXT`
/// extensions when none is given.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 || candidate.is_absolute() {
        return is_executable_file(candidate).then(|| candidate.to_path_buf());
    }

    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for variant in with_extensions(name) {
            let full = dir.join(&variant);
            if is_executable_file(&full) {
                return Some(full);
            }
        }
    }
    None
}

#[cfg(windows)]
fn with_extensions(name: &str) -> Vec<String> {
    if Path::new(name).extension().is_some() {
        return vec![name.to_string()];
    }
    let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.BAT;.CMD;.COM".to_string());
    let mut out = vec![name.to_string()];
    for ext in exts.split(';') {
        let ext = ext.trim();
        if !ext.is_empty() {
            out.push(format!("{name}{}", ext.to_ascii_lowercase()));
        }
    }
    out
}

#[cfg(not(windows))]
fn with_extensions(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}
