//! Running a download from the GUI: in-process progress, or an external CLI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{unbounded, Sender};
use quark_core::config::{GuiDownloadMode, Settings};
use quark_core::download::command::MediaType;
use quark_core::events::{EventSink, ProgressEvent};
use quark_core::logs::FileSink;
use quark_core::{CancelToken, DownloadRequest};
use quark_gui::event::{ProgressChannel, ProgressUpdate};
use quark_gui::model::{MessageKind, ProgressSpec, Theme, WindowSpec};
use quark_gui::App;

use crate::terminal;

pub struct DownloadParams {
    pub urls: Vec<String>,
    pub media_type: MediaType,
    pub format: String,
    pub output_dir: String,
}

/// Run the download per the configured GUI mode. Blocks until done.
pub fn run(app: &App, settings: &Settings, params: DownloadParams, theme: Theme) {
    match settings.gui_download_mode {
        GuiDownloadMode::ExternalCli => {
            if let Err(e) = terminal::launch_cli(&params) {
                app.message(
                    MessageKind::Error,
                    "Quark Downloader",
                    &format!("Could not open a terminal:\n{e}"),
                );
            }
        }
        GuiDownloadMode::Progress => run_with_progress(app, settings, params, theme),
    }
}

fn run_with_progress(app: &App, settings: &Settings, params: DownloadParams, theme: Theme) {
    let (tx, rx) = unbounded::<ProgressUpdate>();
    let channel = ProgressChannel::new(rx);

    let cancel = CancelToken::new();
    let request = DownloadRequest {
        urls: params.urls,
        media_type: params.media_type,
        format: params.format,
        output_dir: Some(params.output_dir.into()),
        hidden_console: cfg!(windows),
    };

    // Engine runs on a worker thread, feeding the progress channel.
    let settings_clone = settings.clone();
    let cancel_engine = cancel.clone();
    let gui_sink = Arc::new(GuiSink::new(tx));
    let engine = thread::spawn(move || {
        let sink: Box<dyn EventSink> = match FileSink::open() {
            Some(file) if settings_clone.download_logs => {
                Box::new(quark_core::events::MultiSink::new(vec![
                    Box::new(SinkRef(gui_sink.clone())),
                    Box::new(file),
                ]))
            }
            _ => Box::new(SinkRef(gui_sink.clone())),
        };
        quark_core::run(&request, &settings_clone, sink.as_ref(), &cancel_engine)
    });

    // Bridge: propagate a UI close/cancel into the engine's cancel token.
    let finished = Arc::new(AtomicBool::new(false));
    let bridge = {
        let flag = channel.cancel_flag();
        let cancel = cancel.clone();
        let finished = finished.clone();
        thread::spawn(move || loop {
            if flag.is_cancelled() {
                cancel.cancel();
                break;
            }
            if finished.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        })
    };

    let spec = ProgressSpec {
        window: WindowSpec::new("Downloading", theme),
        initial_status: "Preparing...".into(),
    };
    let exit_code = app.run_progress(spec, channel.clone());

    let engine_code = engine.join().unwrap_or(1);
    finished.store(true, Ordering::SeqCst);
    let _ = bridge.join();

    if channel.is_cancelled() {
        return; // user cancelled; no completion popup
    }

    let code = if exit_code != 0 {
        exit_code
    } else {
        engine_code
    };
    if code == 0 {
        app.message(MessageKind::Info, "Quark Downloader", "Download Complete!");
    } else {
        let mut body = String::from("Download failed.");
        if settings.download_logs {
            body.push_str(&format!(
                "\n\nLogs: {}",
                quark_core::logs::logs_dir().display()
            ));
        }
        app.message(MessageKind::Error, "Quark Downloader", &body);
    }
}

/// Translates engine [`ProgressEvent`]s into UI [`ProgressUpdate`]s.
struct GuiSink {
    tx: Sender<ProgressUpdate>,
    queue: Mutex<(String, String)>, // (url label, item label)
}

impl GuiSink {
    fn new(tx: Sender<ProgressUpdate>) -> Self {
        GuiSink {
            tx,
            queue: Mutex::new((String::new(), String::new())),
        }
    }

    fn combined_queue(&self) -> String {
        let q = self.queue.lock().unwrap();
        [q.0.as_str(), q.1.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" - ")
    }
}

impl EventSink for GuiSink {
    fn emit(&self, event: ProgressEvent) {
        let update = match event {
            ProgressEvent::QueuePosition { index, total } => {
                {
                    let mut q = self.queue.lock().unwrap();
                    q.0 = format!("URL {index} of {total}");
                    q.1.clear();
                }
                let _ = self.tx.send(ProgressUpdate::Percent(0.0));
                Some(ProgressUpdate::Queue(self.combined_queue()))
            }
            ProgressEvent::PlaylistItem { item, total } => {
                {
                    let mut q = self.queue.lock().unwrap();
                    q.1 = match total {
                        Some(t) => format!("item {item} of {t}"),
                        None => format!("item {item}"),
                    };
                }
                Some(ProgressUpdate::Queue(self.combined_queue()))
            }
            ProgressEvent::Progress { percent } => Some(ProgressUpdate::Percent(percent)),
            ProgressEvent::Eta(eta) => Some(ProgressUpdate::Eta(eta)),
            ProgressEvent::Status(status) => Some(ProgressUpdate::Status(status)),
            ProgressEvent::Log(line) => Some(ProgressUpdate::Log(line)),
            ProgressEvent::ItemFinished { .. } => None,
            ProgressEvent::Done { exit_code } => Some(ProgressUpdate::Done(exit_code)),
        };
        if let Some(update) = update {
            let _ = self.tx.send(update);
        }
    }
}

/// Lets the shared `GuiSink` be used through the `EventSink` trait object.
struct SinkRef(Arc<GuiSink>);
impl EventSink for SinkRef {
    fn emit(&self, event: ProgressEvent) {
        self.0.emit(event);
    }
}
