//! Rotated download logs, exposed as a composable [`EventSink`] that records
//! `Log` lines to a timestamped file (keeping the newest [`MAX_LOGS`]).

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::events::{EventSink, ProgressEvent};
use crate::paths;

pub const MAX_LOGS: usize = 10;

pub fn logs_dir() -> PathBuf {
    paths::config_dir().join("logs")
}

/// Create a new timestamped log file and prune old ones. Returns the file and
/// its path, or `None` if the directory/file could not be created.
pub fn open_log() -> Option<(File, PathBuf)> {
    let dir = logs_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let stamp = timestamp();
    let path = dir.join(format!("{stamp}.log"));
    let file = File::create(&path).ok()?;
    prune_old_logs(&dir);
    Some((file, path))
}

fn timestamp() -> String {
    // Local time without a date crate: derive from SystemTime via chrono-free
    // formatting is unavailable, so fall back to a monotonic-unique name based
    // on the Unix seconds. Human ordering is preserved.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn prune_old_logs(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut logs: Vec<(PathBuf, std::time::SystemTime)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("log") {
                return None;
            }
            let mtime = e.metadata().and_then(|m| m.modified()).ok()?;
            Some((path, mtime))
        })
        .collect();
    logs.sort_by_key(|(_, t)| *t);
    let excess = logs.len().saturating_sub(MAX_LOGS);
    for (path, _) in logs.into_iter().take(excess) {
        let _ = std::fs::remove_file(path);
    }
}

/// An [`EventSink`] that appends `Log` lines to an open file.
pub struct FileSink {
    file: Mutex<File>,
    path: PathBuf,
}

impl FileSink {
    /// Open a new rotated log file. Returns `None` when logging is disabled or
    /// the file cannot be created.
    pub fn open() -> Option<Self> {
        let (file, path) = open_log()?;
        Some(FileSink {
            file: Mutex::new(file),
            path,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl EventSink for FileSink {
    fn emit(&self, event: ProgressEvent) {
        if let ProgressEvent::Log(line) | ProgressEvent::Debug(line) = event {
            if let Ok(mut file) = self.file.lock() {
                let _ = writeln!(file, "{line}");
                let _ = file.flush();
            }
        }
    }
}
