//! A terminal [`EventSink`] that renders a live progress bar (improvement #2)
//! and prints log lines above it. Falls back to plain output when not a TTY.

use std::sync::Mutex;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use quark_core::events::{EventSink, ProgressEvent};

pub struct CliSink {
    bar: ProgressBar,
    prefix: Mutex<String>,
    eta: Mutex<String>,
    tty: bool,
}

impl CliSink {
    pub fn new(is_tty: bool) -> Self {
        let bar = ProgressBar::new(100);
        if is_tty {
            bar.set_style(
                ProgressStyle::with_template(
                    "{prefix:.cyan} {bar:40.green/dim} {percent:>3}% {msg}",
                )
                .unwrap()
                .progress_chars("=> "),
            );
        } else {
            // Without a terminal, indicatif suppresses the bar (and `println`),
            // so log lines are written directly instead.
            bar.set_draw_target(ProgressDrawTarget::hidden());
        }
        CliSink {
            bar,
            prefix: Mutex::new(String::new()),
            eta: Mutex::new(String::new()),
            tty: is_tty,
        }
    }

    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }

    fn refresh_message(&self) {
        let eta = self.eta.lock().unwrap().clone();
        self.bar.set_message(eta);
    }
}

impl EventSink for CliSink {
    fn emit(&self, event: ProgressEvent) {
        match event {
            ProgressEvent::Log(line) => {
                if self.tty {
                    self.bar.println(line);
                } else {
                    println!("{line}");
                }
            }
            ProgressEvent::Progress { percent } => {
                if self.tty {
                    self.bar
                        .set_position(percent.round().clamp(0.0, 100.0) as u64);
                }
            }
            ProgressEvent::Eta(eta) => {
                *self.eta.lock().unwrap() = eta.map(|e| format!("ETA {e}")).unwrap_or_default();
                self.refresh_message();
            }
            ProgressEvent::Status(status) => {
                // Keep the bar's message short; show status when there's no ETA.
                if self.eta.lock().unwrap().is_empty() {
                    self.bar.set_message(status);
                }
            }
            ProgressEvent::QueuePosition { index, total } => {
                let mut prefix = self.prefix.lock().unwrap();
                *prefix = format!("[{index}/{total}]");
                self.bar.set_prefix(prefix.clone());
                self.bar.set_position(0);
            }
            ProgressEvent::PlaylistItem { item, total } => {
                let label = match total {
                    Some(t) => format!("item {item}/{t}"),
                    None => format!("item {item}"),
                };
                let base = self.prefix.lock().unwrap().clone();
                self.bar.set_prefix(if base.is_empty() {
                    label
                } else {
                    format!("{base} {label}")
                });
                self.bar.set_position(0);
            }
            ProgressEvent::ItemFinished { .. } => {
                *self.eta.lock().unwrap() = String::new();
            }
            ProgressEvent::Done { .. } => self.finish(),
        }
    }
}
