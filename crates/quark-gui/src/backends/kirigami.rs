//! Native Qt backend (the `kirigami` setting): Qt Widgets driven through a
//! small cxx bridge (`cpp/qt_backend.cpp`). Cross-platform wherever Qt 6 is
//! installed; on KDE systems the widgets render with the platform theme
//! (Breeze), matching Kirigami applications.

use crate::backend::Renderer;
use crate::event::{ProgressChannel, ProgressUpdate};
use crate::model::{
    Field, FieldValue, FormOutcome, FormSpec, FormValues, MessageKind, ProgressSpec,
};

#[cxx::bridge(namespace = "quark_gui_qt")]
mod ffi {
    /// One form field. `kind`: 0 text, 1 list, 2 radio, 3 combo, 4 check,
    /// 5 path (`flag` = directory), 6 section, 7 dependent combo.
    struct FieldFfi {
        kind: u8,
        id: String,
        label: String,
        value: String,
        options: Vec<String>,
        selected: usize,
        flag: bool,
        controller: String,
        option_set_sizes: Vec<usize>,
        dependent_options: Vec<String>,
    }

    struct FormFfi {
        title: String,
        dark: bool,
        width: i32,
        submit_label: String,
        cancel_label: String,
        extra_labels: Vec<String>,
        fields: Vec<FieldFfi>,
    }

    /// One submitted value. `kind`: 0 text, 1 list, 2 index, 3 bool.
    struct ValueFfi {
        id: String,
        kind: u8,
        text: String,
        list: Vec<String>,
        index: usize,
        flag: bool,
    }

    /// `outcome`: 0 cancel, 1 submit, 2 extra button (`extra_index`).
    struct FormResultFfi {
        outcome: u8,
        extra_index: usize,
        values: Vec<ValueFfi>,
    }

    struct ProgressFfi {
        title: String,
        dark: bool,
        initial_status: String,
    }

    /// One drained progress update. `kind`: 0 none pending, 1 percent
    /// (`number`), 2 status, 3 eta, 4 queue (`text`), 5 done (`code`).
    struct PollFfi {
        kind: u8,
        number: f64,
        text: String,
        code: i32,
    }

    extern "Rust" {
        type ProgressSource;
        fn poll(&self) -> PollFfi;
        fn request_cancel(&self);
    }

    unsafe extern "C++" {
        include!("quark-gui/cpp/qt_backend.h");

        fn qt_message(is_error: bool, title: &str, body: &str);
        fn qt_run_form(form: &FormFfi) -> FormResultFfi;
        fn qt_run_progress(spec: &ProgressFfi, source: Box<ProgressSource>) -> i32;
    }
}

/// Rust side of the progress channel, polled by a Qt timer.
pub struct ProgressSource {
    channel: ProgressChannel,
}

impl ProgressSource {
    fn poll(&self) -> ffi::PollFfi {
        // Skip log lines: the Qt progress view has no detail area.
        loop {
            let update = match self.channel.updates.try_recv() {
                Ok(update) => update,
                Err(_) => return poll_ffi(0, 0.0, String::new(), 0),
            };
            return match update {
                ProgressUpdate::Percent(p) => poll_ffi(1, p.clamp(0.0, 100.0), String::new(), 0),
                ProgressUpdate::Status(s) => poll_ffi(2, 0.0, s, 0),
                ProgressUpdate::Eta(e) => {
                    let text = e.map(|x| format!("Time left: {x}")).unwrap_or_default();
                    poll_ffi(3, 0.0, text, 0)
                }
                ProgressUpdate::Queue(q) => poll_ffi(4, 0.0, q, 0),
                ProgressUpdate::Log(_) => continue,
                ProgressUpdate::Done(c) => poll_ffi(5, 0.0, String::new(), c),
            };
        }
    }

    fn request_cancel(&self) {
        self.channel.request_cancel();
    }
}

fn poll_ffi(kind: u8, number: f64, text: String, code: i32) -> ffi::PollFfi {
    ffi::PollFfi {
        kind,
        number,
        text,
        code,
    }
}

pub struct QtRenderer;

impl QtRenderer {
    pub fn new() -> Self {
        QtRenderer
    }
}

impl Default for QtRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for QtRenderer {
    fn run_form(&self, spec: FormSpec) -> FormOutcome {
        let form = to_form_ffi(&spec);
        let result = ffi::qt_run_form(&form);
        let values = from_values_ffi(result.values);
        match result.outcome {
            1 => FormOutcome::Submit(values),
            2 => match spec.extra_buttons.get(result.extra_index) {
                Some(b) => FormOutcome::Button(b.id.clone(), values),
                None => FormOutcome::Cancel,
            },
            _ => FormOutcome::Cancel,
        }
    }

    fn run_progress(&self, spec: ProgressSpec, channel: ProgressChannel) -> i32 {
        let ffi_spec = ffi::ProgressFfi {
            title: spec.window.title.clone(),
            dark: spec.window.theme.is_dark(),
            initial_status: spec.initial_status.clone(),
        };
        ffi::qt_run_progress(&ffi_spec, Box::new(ProgressSource { channel }))
    }

    fn message(&self, kind: MessageKind, title: &str, body: &str) {
        ffi::qt_message(matches!(kind, MessageKind::Error), title, body);
    }

    fn name(&self) -> &'static str {
        "kirigami"
    }
}

fn to_form_ffi(spec: &FormSpec) -> ffi::FormFfi {
    let fields = spec
        .fields
        .iter()
        .map(|field| match field {
            Field::Text { id, label, value } => field_ffi(0, id, label, value, &[], 0, false),
            Field::List {
                id, label, items, ..
            } => field_ffi(1, id, label, "", items, 0, false),
            Field::Radio {
                id,
                label,
                options,
                selected,
            } => field_ffi(2, id, label, "", options, *selected, false),
            Field::Combo {
                id,
                label,
                options,
                selected,
            } => field_ffi(3, id, label, "", options, *selected, false),
            Field::DependentCombo {
                id,
                label,
                controller,
                option_sets,
                selected,
            } => {
                let options = option_sets
                    .get(spec.selected_index(controller))
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let mut field = field_ffi(7, id, label, "", options, *selected, false);
                field.controller = controller.clone();
                field.option_set_sizes = option_sets.iter().map(Vec::len).collect();
                field.dependent_options = option_sets.iter().flatten().cloned().collect();
                field
            }
            Field::Check { id, label, value } => field_ffi(4, id, label, "", &[], 0, *value),
            Field::Path {
                id,
                label,
                value,
                directory,
            } => field_ffi(5, id, label, value, &[], 0, *directory),
            Field::Section { label } => field_ffi(6, "", label, "", &[], 0, false),
        })
        .collect();

    ffi::FormFfi {
        title: spec.window.title.clone(),
        dark: spec.window.theme.is_dark(),
        width: spec.window.width as i32,
        submit_label: spec.submit_label.clone(),
        cancel_label: spec.cancel_label.clone(),
        extra_labels: spec.extra_buttons.iter().map(|b| b.label.clone()).collect(),
        fields,
    }
}

fn field_ffi(
    kind: u8,
    id: &str,
    label: &str,
    value: &str,
    options: &[String],
    selected: usize,
    flag: bool,
) -> ffi::FieldFfi {
    ffi::FieldFfi {
        kind,
        id: id.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        options: options.to_vec(),
        selected,
        flag,
        controller: String::new(),
        option_set_sizes: Vec::new(),
        dependent_options: Vec::new(),
    }
}

fn from_values_ffi(values: Vec<ffi::ValueFfi>) -> FormValues {
    let mut out = FormValues::default();
    for value in values {
        let field_value = match value.kind {
            0 => FieldValue::Text(value.text),
            1 => FieldValue::List(value.list),
            2 => FieldValue::Index(value.index),
            _ => FieldValue::Bool(value.flag),
        };
        out.0.insert(value.id, field_value);
    }
    out
}
