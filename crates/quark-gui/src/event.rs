//! Live updates for a progress view. The host app pushes [`ProgressUpdate`]s
//! from a worker thread; the backend renders them and reports cancellation
//! back through the shared flag. Independent of any download engine.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Receiver;

/// One update to a progress view.
#[derive(Debug, Clone)]
pub enum ProgressUpdate {
    /// Overall completion 0.0..=100.0.
    Percent(f64),
    /// Short status text.
    Status(String),
    /// Estimated time remaining (or `None` to clear).
    Eta(Option<String>),
    /// Queue/playlist position label, e.g. "URL 2 of 5 - item 3 of 12".
    Queue(String),
    /// A human log line (shown in the detail area if the backend has one).
    Log(String),
    /// Terminal update with an exit code; the view may close afterward.
    Done(i32),
}

/// Wires a progress window to its producer: a stream of updates plus a flag the
/// backend sets when the user closes/cancels the window.
#[derive(Clone)]
pub struct ProgressChannel {
    /// The stream of updates the backend renders.
    pub updates: Receiver<ProgressUpdate>,
    cancel: Arc<AtomicBool>,
}

impl ProgressChannel {
    /// Wrap a receiver, with a fresh (uncancelled) cancellation flag.
    pub fn new(updates: Receiver<ProgressUpdate>) -> Self {
        ProgressChannel {
            updates,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// A handle the host can poll to learn whether the user cancelled.
    pub fn cancel_flag(&self) -> CancelFlag {
        CancelFlag(self.cancel.clone())
    }

    /// Called by the backend when the user closes the window.
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Whether the user has cancelled the progress view.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

/// Read-only view of a [`ProgressChannel`]'s cancellation state.
#[derive(Clone)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    /// Whether the user has cancelled the progress view.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
