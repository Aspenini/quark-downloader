//! Native GTK 4 form window.

use std::cell::Cell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::prelude::*;

use super::{apply_theme, run_until};
use crate::model::{Field, FieldValue, FormOutcome, FormSpec, FormValues};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Decision {
    Submit,
    Extra(usize),
    Cancel,
}

/// A widget to read back after the window closes.
enum Control {
    Text(gtk::Entry),
    List(gtk::TextView),
    Combo(gtk::DropDown),
    Check(gtk::CheckButton),
    Radio(Vec<gtk::CheckButton>),
}

struct Entry {
    id: String,
    control: Control,
}

pub fn run_form(spec: FormSpec) -> FormOutcome {
    apply_theme(spec.window.theme);

    let window = gtk::Window::new();
    window.set_title(Some(&spec.window.title));
    window.set_default_width(spec.window.width as i32);
    window.set_modal(true);

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 10);
    outer.set_margin_top(16);
    outer.set_margin_bottom(16);
    outer.set_margin_start(16);
    outer.set_margin_end(16);

    let mut entries: Vec<Entry> = Vec::new();
    for field in &spec.fields {
        outer.append(&build_field(&window, field, &mut entries));
    }

    // Button row, right-aligned: [extras...] [cancel] [submit].
    let decision = Rc::new(Cell::new(None::<Decision>));
    let done = Rc::new(Cell::new(false));
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);

    let decide = |label: &str, choice: Decision| {
        let button = gtk::Button::with_label(label);
        let decision = decision.clone();
        let done = done.clone();
        button.connect_clicked(move |_| {
            decision.set(Some(choice));
            done.set(true);
        });
        button
    };
    for (i, btn) in spec.extra_buttons.iter().enumerate() {
        buttons.append(&decide(&btn.label, Decision::Extra(i)));
    }
    buttons.append(&decide(&spec.cancel_label, Decision::Cancel));
    let submit = decide(&spec.submit_label, Decision::Submit);
    submit.add_css_class("suggested-action");
    buttons.append(&submit);
    outer.append(&buttons);

    {
        let done = done.clone();
        window.connect_close_request(move |_| {
            done.set(true);
            gtk::glib::Propagation::Proceed
        });
    }

    window.set_child(Some(&outer));
    window.present();
    run_until(&done);

    let values = read_values(&entries);
    window.close();
    // Let the close settle so the window disappears promptly.
    let ctx = gtk::glib::MainContext::default();
    while ctx.iteration(false) {}

    match decision.get() {
        Some(Decision::Submit) => FormOutcome::Submit(values),
        Some(Decision::Extra(i)) => match spec.extra_buttons.get(i) {
            Some(b) => FormOutcome::Button(b.id.clone(), values),
            None => FormOutcome::Cancel,
        },
        _ => FormOutcome::Cancel,
    }
}

fn build_field(window: &gtk::Window, field: &Field, entries: &mut Vec<Entry>) -> gtk::Widget {
    match field {
        Field::Section { label } => {
            let l = gtk::Label::new(None);
            l.set_markup(&format!("<b>{}</b>", gtk::glib::markup_escape_text(label)));
            l.set_halign(gtk::Align::Start);
            l.upcast()
        }
        Field::Text { id, label, value } => {
            let row = labeled_row(label);
            let entry = gtk::Entry::new();
            entry.set_text(value);
            entry.set_hexpand(true);
            row.append(&entry);
            entries.push(Entry {
                id: id.clone(),
                control: Control::Text(entry),
            });
            row.upcast()
        }
        Field::Path {
            id,
            label,
            value,
            directory,
        } => {
            let row = labeled_row(label);
            let entry = gtk::Entry::new();
            entry.set_text(value);
            entry.set_hexpand(true);
            row.append(&entry);
            let browse = gtk::Button::with_label("Browse...");
            {
                let entry = entry.clone();
                let window = window.clone();
                let directory = *directory;
                browse.connect_clicked(move |_| {
                    browse_path(&window, &entry, directory);
                });
            }
            row.append(&browse);
            entries.push(Entry {
                id: id.clone(),
                control: Control::Text(entry),
            });
            row.upcast()
        }
        Field::List {
            id, label, items, ..
        } => {
            let col = gtk::Box::new(gtk::Orientation::Vertical, 4);
            col.append(&start_label(label));
            let view = gtk::TextView::new();
            view.buffer().set_text(&items.join("\n"));
            let scroll = gtk::ScrolledWindow::new();
            scroll.set_min_content_height(110);
            scroll.set_child(Some(&view));
            col.append(&scroll);
            entries.push(Entry {
                id: id.clone(),
                control: Control::List(view),
            });
            col.upcast()
        }
        Field::Combo {
            id,
            label,
            options,
            selected,
        } => {
            let row = labeled_row(label);
            let strs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
            let drop = gtk::DropDown::from_strings(&strs);
            drop.set_selected(*selected as u32);
            row.append(&drop);
            entries.push(Entry {
                id: id.clone(),
                control: Control::Combo(drop),
            });
            row.upcast()
        }
        Field::Check { id, label, value } => {
            let check = gtk::CheckButton::with_label(label);
            check.set_active(*value);
            entries.push(Entry {
                id: id.clone(),
                control: Control::Check(check.clone()),
            });
            check.upcast()
        }
        Field::Radio {
            id,
            label,
            options,
            selected,
        } => {
            let col = gtk::Box::new(gtk::Orientation::Vertical, 2);
            col.append(&start_label(label));
            let mut buttons: Vec<gtk::CheckButton> = Vec::with_capacity(options.len());
            for (i, opt) in options.iter().enumerate() {
                let b = gtk::CheckButton::with_label(opt);
                if let Some(first) = buttons.first() {
                    b.set_group(Some(first));
                }
                b.set_active(i == *selected);
                col.append(&b);
                buttons.push(b);
            }
            entries.push(Entry {
                id: id.clone(),
                control: Control::Radio(buttons),
            });
            col.upcast()
        }
    }
}

fn labeled_row(label: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let l = start_label(label);
    l.set_width_chars(18);
    l.set_xalign(0.0);
    row.append(&l);
    row
}

fn start_label(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_halign(gtk::Align::Start);
    l
}

fn browse_path(window: &gtk::Window, entry: &gtk::Entry, directory: bool) {
    let dialog = gtk::FileDialog::new();
    let entry = entry.clone();
    let apply = move |file: Result<gtk4::gio::File, gtk::glib::Error>| {
        if let Ok(file) = file {
            if let Some(path) = file.path() {
                entry.set_text(&path.to_string_lossy());
            }
        }
    };
    if directory {
        dialog.select_folder(Some(window), None::<&gtk4::gio::Cancellable>, apply);
    } else {
        dialog.open(Some(window), None::<&gtk4::gio::Cancellable>, apply);
    }
}

fn read_values(entries: &[Entry]) -> FormValues {
    let mut values = FormValues::default();
    for entry in entries {
        let value = match &entry.control {
            Control::Text(e) => FieldValue::Text(e.text().to_string()),
            Control::List(view) => {
                let buffer = view.buffer();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                FieldValue::List(
                    text.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect(),
                )
            }
            Control::Combo(drop) => {
                let sel = drop.selected();
                FieldValue::Index(if sel == gtk::INVALID_LIST_POSITION {
                    0
                } else {
                    sel as usize
                })
            }
            Control::Check(check) => FieldValue::Bool(check.is_active()),
            Control::Radio(buttons) => {
                FieldValue::Index(buttons.iter().position(|b| b.is_active()).unwrap_or(0))
            }
        };
        values.0.insert(entry.id.clone(), value);
    }
    values
}
