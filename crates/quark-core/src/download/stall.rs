//! Per-run stall detection. Tracks the last time yt-dlp emitted output, the
//! current playlist item, and whether the run is in silent post-processing.
//!
//! When a playlist is restarted past a stuck item, yt-dlp renumbers from 1
//! after `--playlist-items N:`; the `offset` rewrites those back to absolute.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::progress;

/// How long yt-dlp may produce no output before the current item is treated as
/// stuck. yt-dlp prints progress every second or two while working.
pub const STALL_TIMEOUT: Duration = Duration::from_secs(30);

struct Inner {
    last: Instant,
    suspended: bool,
    killed: bool,
    finished: bool,
    current_item: Option<u32>,
    total_items: Option<u32>,
}

pub struct StallMonitor {
    inner: Mutex<Inner>,
    offset: u32,
}

impl StallMonitor {
    pub fn new(offset: u32, total_items: Option<u32>) -> Self {
        StallMonitor {
            inner: Mutex::new(Inner {
                last: Instant::now(),
                suspended: false,
                killed: false,
                finished: false,
                current_item: None,
                total_items,
            }),
            offset,
        }
    }

    /// Record activity from a line and, for playlist item lines, rewrite the
    /// item numbers back to absolute. Returns the (possibly rewritten) line
    /// and the absolute `(item, total)` when the line announced a new item.
    pub fn observe(&self, line: &str) -> (String, Option<(u32, u32)>) {
        let mut inner = self.inner.lock().unwrap();
        inner.last = Instant::now();

        if let Some((rel, rel_total)) = progress::parse_playlist_item(line) {
            if inner.total_items.is_none() {
                inner.total_items = Some(rel_total + self.offset);
            }
            let abs = rel + self.offset;
            inner.current_item = Some(abs);
            inner.suspended = false;
            let abs_total = inner.total_items.unwrap_or(rel_total);
            let prefix = format!("[download] Downloading item {rel} of {rel_total}");
            let replacement = format!("[download] Downloading item {abs} of {abs_total}");
            return (
                line.replacen(&prefix, &replacement, 1),
                Some((abs, abs_total)),
            );
        }

        if progress::is_postprocessing(line) {
            inner.suspended = true;
        } else if progress::is_resume(line) {
            inner.suspended = false;
        }
        (line.to_string(), None)
    }

    pub fn stalled(&self, timeout: Duration) -> bool {
        let inner = self.inner.lock().unwrap();
        if inner.suspended || inner.finished {
            return false;
        }
        inner.last.elapsed() >= timeout
    }

    pub fn mark_killed(&self) {
        self.inner.lock().unwrap().killed = true;
    }

    pub fn finish(&self) {
        self.inner.lock().unwrap().finished = true;
    }

    pub fn killed(&self) -> bool {
        self.inner.lock().unwrap().killed
    }

    pub fn finished(&self) -> bool {
        self.inner.lock().unwrap().finished
    }

    pub fn current_item(&self) -> Option<u32> {
        self.inner.lock().unwrap().current_item
    }

    pub fn total_items(&self) -> Option<u32> {
        self.inner.lock().unwrap().total_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_playlist_item_with_offset() {
        let m = StallMonitor::new(5, None);
        let (out, item) = m.observe("[download] Downloading item 1 of 7");
        assert_eq!(out, "[download] Downloading item 6 of 12");
        assert_eq!(item, Some((6, 12)));
        assert_eq!(m.current_item(), Some(6));
        assert_eq!(m.total_items(), Some(12));
    }

    #[test]
    fn suspends_during_postprocessing() {
        let m = StallMonitor::new(0, None);
        m.observe("[Merger] Merging formats");
        assert!(!m.stalled(Duration::from_secs(0)));
        m.observe("[download] Destination: x");
        assert!(m.stalled(Duration::from_secs(0)));
    }
}
