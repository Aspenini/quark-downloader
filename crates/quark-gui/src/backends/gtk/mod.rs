//! Native GTK 4 backend built with gtk4-rs. Cross-platform wherever the GTK 4
//! libraries are present (Linux first-class; also builds on macOS/Windows).

mod form;
mod progress;

use std::cell::Cell;
use std::rc::Rc;

use gtk4 as gtk;

use crate::backend::Renderer;
use crate::event::ProgressChannel;
use crate::model::{FormOutcome, FormSpec, MessageKind, ProgressSpec, Theme};

pub struct GtkRenderer {
    available: bool,
}

impl GtkRenderer {
    pub fn new() -> Self {
        GtkRenderer {
            available: gtk::init().is_ok(),
        }
    }
}

impl Default for GtkRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for GtkRenderer {
    fn run_form(&self, spec: FormSpec) -> FormOutcome {
        if !self.available {
            return FormOutcome::Cancel;
        }
        form::run_form(spec)
    }

    fn run_progress(&self, spec: ProgressSpec, channel: ProgressChannel) -> i32 {
        if !self.available {
            return drain(&channel);
        }
        progress::run_progress(spec, channel)
    }

    fn message(&self, kind: MessageKind, title: &str, body: &str) {
        if !self.available {
            eprintln!("{title}: {body}");
            return;
        }
        let prefix = match kind {
            MessageKind::Error => "Error",
            MessageKind::Info => "Info",
        };
        let dialog = gtk::AlertDialog::builder()
            .message(title)
            .detail(format!("{prefix}: {body}"))
            .modal(true)
            .build();
        let done = Rc::new(Cell::new(false));
        let done_cb = done.clone();
        dialog.choose(
            None::<&gtk::Window>,
            None::<&gtk4::gio::Cancellable>,
            move |_| done_cb.set(true),
        );
        run_until(&done);
    }

    fn name(&self) -> &'static str {
        "gtk"
    }
}

/// Apply the requested theme via the GTK settings.
pub(crate) fn apply_theme(theme: Theme) {
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(theme.is_dark());
    }
}

/// Iterate the default main context until `done` is set.
pub(crate) fn run_until(done: &Rc<Cell<bool>>) {
    let ctx = gtk::glib::MainContext::default();
    while !done.get() {
        ctx.iteration(true);
    }
}

/// Fallback used when GTK could not initialise (e.g. no display): drain the
/// channel for the exit code so the producer thread is not blocked.
fn drain(channel: &ProgressChannel) -> i32 {
    let mut code = 0;
    while let Ok(update) = channel.updates.recv() {
        if let crate::event::ProgressUpdate::Done(c) = update {
            code = c;
            break;
        }
    }
    code
}
