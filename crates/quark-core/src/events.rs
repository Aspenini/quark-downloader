//! The single seam between the download engine and any front-end.
//!
//! The engine never prints directly; it emits [`ProgressEvent`]s to an
//! [`EventSink`]. The CLI sink drives a progress bar; a GUI sink forwards
//! events into its UI thread.

use std::sync::Arc;

/// A discrete update from a running download.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// A human-readable line (the old `QuarkLogs.puts` passthrough).
    Log(String),
    /// Diagnostic detail (tool paths, the exact yt-dlp command). Log files
    /// record it; terminals show it only in verbose mode.
    Debug(String),
    /// Position in a multi-URL queue, e.g. "URL 2 of 5".
    QueuePosition { index: usize, total: usize },
    /// Current playlist item, e.g. "item 3 of 12".
    PlaylistItem { item: u32, total: Option<u32> },
    /// Download completion of the current item, 0.0..=100.0.
    Progress { percent: f64 },
    /// Estimated time remaining for the current item.
    Eta(Option<String>),
    /// Current download rate (e.g. `2.53MiB/s`), or `None` while unknown.
    Speed(Option<String>),
    /// A short status string (extracting, merging, ...).
    Status(String),
    /// One queue item finished with the given exit code.
    ItemFinished { url: String, exit_code: i32 },
    /// The whole run finished with the given exit code.
    Done { exit_code: i32 },
}

/// Receives events from the engine. Implementations must be thread-safe.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}

/// A sink that discards everything. Useful for tests.
pub struct NullSink;
impl EventSink for NullSink {
    fn emit(&self, _event: ProgressEvent) {}
}

/// A sink that prints `Log`/`Debug` lines to stdout and ignores the rest.
/// Used as a simple default and for `external_cli` style output.
pub struct PlainSink;
impl EventSink for PlainSink {
    fn emit(&self, event: ProgressEvent) {
        if let ProgressEvent::Log(line) | ProgressEvent::Debug(line) = event {
            println!("{line}");
        }
    }
}

/// Convenience alias for a shared sink.
pub type SharedSink = Arc<dyn EventSink>;

/// Fans one event out to several sinks (e.g. a UI plus a log file).
pub struct MultiSink {
    sinks: Vec<Box<dyn EventSink>>,
}

impl MultiSink {
    pub fn new(sinks: Vec<Box<dyn EventSink>>) -> Self {
        MultiSink { sinks }
    }
}

impl EventSink for MultiSink {
    fn emit(&self, event: ProgressEvent) {
        let Some((last, rest)) = self.sinks.split_last() else {
            return;
        };
        for sink in rest {
            sink.emit(event.clone());
        }
        last.emit(event);
    }
}
