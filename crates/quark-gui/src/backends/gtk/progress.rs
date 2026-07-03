//! Native GTK 4 progress window. A 50 ms glib timeout drains the update
//! channel and drives the labels and progress bar.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk4 as gtk;
use gtk4::prelude::*;

use super::{apply_theme, run_until};
use crate::event::{ProgressChannel, ProgressUpdate};
use crate::model::ProgressSpec;

pub fn run_progress(spec: ProgressSpec, channel: ProgressChannel) -> i32 {
    apply_theme(spec.window.theme);

    let window = gtk::Window::new();
    window.set_title(Some(&spec.window.title));
    window.set_default_width(460);

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 10);
    outer.set_margin_top(16);
    outer.set_margin_bottom(16);
    outer.set_margin_start(16);
    outer.set_margin_end(16);

    let queue = start_label("");
    let bar = gtk::ProgressBar::new();
    let status = start_label(&spec.initial_status);
    let eta = start_label("");
    let cancel = gtk::Button::with_label("Cancel");
    cancel.set_halign(gtk::Align::End);

    outer.append(&queue);
    outer.append(&bar);
    outer.append(&status);
    outer.append(&eta);
    outer.append(&cancel);
    window.set_child(Some(&outer));

    let done = Rc::new(Cell::new(false));
    let exit_code = Rc::new(Cell::new(None::<i32>));

    {
        let done = done.clone();
        cancel.connect_clicked(move |_| done.set(true));
    }
    {
        let done = done.clone();
        window.connect_close_request(move |_| {
            done.set(true);
            gtk::glib::Propagation::Proceed
        });
    }

    let poll = {
        let channel = channel.clone();
        let done = done.clone();
        let exit_code = exit_code.clone();
        let bar = bar.clone();
        let status = status.clone();
        let eta = eta.clone();
        let queue = queue.clone();
        move || {
            while let Ok(update) = channel.updates.try_recv() {
                match update {
                    ProgressUpdate::Percent(p) => bar.set_fraction(p.clamp(0.0, 100.0) / 100.0),
                    ProgressUpdate::Status(s) => status.set_text(&s),
                    ProgressUpdate::Eta(e) => {
                        let text = e.map(|x| format!("Time left: {x}")).unwrap_or_default();
                        eta.set_text(&text);
                    }
                    ProgressUpdate::Queue(q) => queue.set_text(&q),
                    ProgressUpdate::Log(_) => {}
                    ProgressUpdate::Done(c) => {
                        exit_code.set(Some(c));
                        done.set(true);
                    }
                }
            }
            if done.get() {
                gtk::glib::ControlFlow::Break
            } else {
                gtk::glib::ControlFlow::Continue
            }
        }
    };
    gtk::glib::timeout_add_local(Duration::from_millis(50), poll);

    window.present();
    run_until(&done);
    window.close();
    let ctx = gtk::glib::MainContext::default();
    while ctx.iteration(false) {}

    let code = exit_code.get();
    if code.is_none() {
        // Cancelled before completion.
        channel.request_cancel();
    }
    code.unwrap_or(1)
}

fn start_label(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_halign(gtk::Align::Start);
    l
}
