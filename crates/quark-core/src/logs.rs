use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::config;

pub const MAX_LOGS: usize = 10;

struct ActiveLog {
    file: File,
    path: PathBuf,
}

static ACTIVE: Mutex<Option<ActiveLog>> = Mutex::new(None);

pub fn logs_dir() -> Result<PathBuf, config::ConfigError> {
    config::ensure_config_dir()?;
    Ok(config::config_dir().join("logs"))
}

pub fn active_path() -> Option<PathBuf> {
    ACTIVE.lock().ok()?.as_ref().map(|l| l.path.clone())
}

pub fn open_download_log(enabled: bool) -> Option<PathBuf> {
    if !enabled {
        return None;
    }
    let mut guard = ACTIVE.lock().ok()?;
    if let Some(log) = guard.as_ref() {
        return Some(log.path.clone());
    }
    let dir = logs_dir().ok()?;
    let (file, path) = open_log(&dir).ok()?;
    *guard = Some(ActiveLog {
        file,
        path: path.clone(),
    });
    Some(path)
}

pub fn open_log(dir: &Path) -> io::Result<(File, PathBuf)> {
    fs::create_dir_all(dir)?;
    let stamp = timestamp();
    let path = dir.join(format!("{stamp}.log"));
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    let _ = prune_old_logs(dir);
    Ok((file, path))
}

pub fn prune_old_logs(dir: &Path) -> io::Result<()> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            files.push((path, modified));
        }
    }
    files.sort_by_key(|(_, t)| *t);
    let excess = files.len().saturating_sub(MAX_LOGS);
    for (path, _) in files.into_iter().take(excess) {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

pub fn log_line(message: &str) {
    print_to(message, &mut io::stdout());
}

pub fn log_line_err(message: &str) {
    print_to(message, &mut io::stderr());
}

pub fn print_to(message: &str, io: &mut dyn Write) {
    let _ = writeln!(io, "{message}");
    if let Ok(mut guard) = ACTIVE.lock()
        && let Some(log) = guard.as_mut()
    {
        let _ = writeln!(log.file, "{message}");
        let _ = log.file.flush();
    }
}

pub fn close() {
    if let Ok(mut guard) = ACTIVE.lock() {
        *guard = None;
    }
}

fn timestamp() -> String {
    // Local-ish YYYY-MM-DD_HH-MM-SS without extra crates.
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = civil_from_unix(now as i64);
    format!("{y:04}-{mo:02}-{d:02}_{h:02}-{mi:02}-{s:02}")
}

fn civil_from_unix(mut unix: i64) -> (i32, u32, u32, u32, u32, u32) {
    unix += quark_platform::local_offset_secs();
    let secs = unix.rem_euclid(86_400);
    let days = unix.div_euclid(86_400);
    let h = (secs / 3600) as u32;
    let mi = ((secs % 3600) / 60) as u32;
    let s = (secs % 60) as u32;
    let (y, mo, d) = civil_from_days(days);
    (y, mo, d, h, mi, s)
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    // Howard Hinnant civil_from_days
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn keeps_only_newest_rotated_logs() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("quark-logs-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        for i in 0..12 {
            let path = dir.join(format!("download-{i}.log"));
            fs::write(&path, i.to_string()).unwrap();
            let time = SystemTime::now() - Duration::from_secs(12 - i);
            let _ = filetime_set(&path, time);
        }
        prune_old_logs(&dir).unwrap();
        let mut files: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".log"))
            .collect();
        files.sort();
        assert_eq!(files.len(), MAX_LOGS);
        assert!(!files.iter().any(|f| f == "download-0.log"));
        assert!(!files.iter().any(|f| f == "download-1.log"));
        let _ = fs::remove_dir_all(&dir);
    }

    fn filetime_set(path: &Path, time: SystemTime) -> io::Result<()> {
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_modified(time)
    }
}
