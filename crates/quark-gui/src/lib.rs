//! QuarkGUI — a small, reusable cross-platform UI toolkit.
//!
//! Describe a UI once with [`model`] types, then render it through a
//! [`backend::Renderer`]. Slint is the default backend on every platform;
//! native backends (Win32/Cocoa/GTK/Kirigami) plug in behind features
//! without changing your UI code. The crate is application-agnostic and can be
//! used by any program, not just Quark Downloader.
//!
//! ```no_run
//! use quark_gui::{App, Backend};
//! use quark_gui::model::{FormSpec, WindowSpec, Theme, Field};
//!
//! let app = App::new(Backend::Auto);
//! let mut form = FormSpec::new(WindowSpec::new("Demo", Theme::Light));
//! form.fields.push(Field::Text { id: "name".into(), label: "Name".into(), value: String::new() });
//! if let quark_gui::model::FormOutcome::Submit(values) = app.run_form(form) {
//!     println!("{}", values.text("name"));
//! }
//! ```
//!
//! # Backends
//!
//! | [`Backend`] | Cargo feature | Notes |
//! |-------------|---------------|-------|
//! | `Slint` | `slint` (default) | Pure Rust, no system dependencies |
//! | `Cocoa` | `native-cocoa` | Native AppKit (macOS) |
//! | `Win32` | `native-windows` | Native Win32 (Windows) |
//! | `Gtk` | `native-gtk` | GTK 4 via gtk4-rs (needs the GTK 4 libraries) |
//! | `Kirigami` | `native-kirigami` | Qt Widgets via a cxx bridge (needs Qt 6) |
//! | `Headless` | — | Accepts form defaults; for tests and non-interactive use |
//!
//! Requesting a backend that is not compiled in falls back to Slint (or
//! Headless when Slint is disabled too) — see [`App::new`].

#![warn(missing_docs)]

pub mod backend;
pub mod event;
pub mod model;

mod backends;

pub use backend::{Backend, Renderer};
pub use event::{CancelFlag, ProgressChannel, ProgressUpdate};
pub use model::{
    ExtraButton, Field, FieldValue, FormOutcome, FormSpec, FormValues, MessageKind, ProgressSpec,
    Theme, WindowSpec,
};

/// The entry point: holds a chosen renderer and forwards UI calls to it.
pub struct App {
    renderer: Box<dyn Renderer>,
    backend: Backend,
}

impl App {
    /// Create an app using `preferred`, falling back automatically when that
    /// backend is unavailable on this build/platform.
    pub fn new(preferred: Backend) -> Self {
        let (renderer, backend) = backend::renderer(preferred);
        App { renderer, backend }
    }

    /// The backend actually in use (may differ from the requested one).
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Show a form and block until the user submits, presses an extra button,
    /// or cancels.
    pub fn run_form(&self, spec: FormSpec) -> FormOutcome {
        self.renderer.run_form(spec)
    }

    /// Show a progress window, consuming `channel` until a
    /// [`ProgressUpdate::Done`] arrives or the user closes it. Returns the
    /// final exit code (or `1` when cancelled early).
    pub fn run_progress(&self, spec: ProgressSpec, channel: ProgressChannel) -> i32 {
        self.renderer.run_progress(spec, channel)
    }

    /// Show a modal message dialog.
    pub fn message(&self, kind: MessageKind, title: &str, body: &str) {
        self.renderer.message(kind, title, body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Field, FormOutcome};

    #[test]
    fn headless_form_accepts_defaults() {
        let app = App::new(Backend::Headless);
        let mut form = FormSpec::new(WindowSpec::new("t", Theme::Light));
        form.fields.push(Field::Text {
            id: "url".into(),
            label: "URL".into(),
            value: "abc".into(),
        });
        form.fields.push(Field::Check {
            id: "logs".into(),
            label: "Logs".into(),
            value: true,
        });
        match app.run_form(form) {
            FormOutcome::Submit(values) => {
                assert_eq!(values.text("url"), "abc");
                assert!(values.bool("logs"));
            }
            _ => panic!("expected submit"),
        }
    }
}
