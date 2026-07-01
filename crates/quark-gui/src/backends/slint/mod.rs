//! Slint backend: maps QuarkGUI's model onto a generic compiled Slint UI.
//! This is the default backend on every platform.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, Model, ModelRc, SharedString, TimerMode, VecModel};

use crate::backend::Renderer;
use crate::event::{ProgressChannel, ProgressUpdate};
use crate::model::{
    Field, FieldValue, FormOutcome, FormSpec, FormValues, MessageKind, ProgressSpec,
};

slint::include_modules!();

#[derive(Default)]
pub struct SlintRenderer;

impl SlintRenderer {
    pub fn new() -> Self {
        SlintRenderer
    }
}

#[derive(Clone, Copy)]
enum Decision {
    Submit,
    Extra(usize),
    Cancel,
}

fn to_field_data(field: &Field) -> FieldData {
    let mut data = FieldData::default();
    match field {
        Field::Text { id, label, value } => {
            data.id = id.into();
            data.kind = "text".into();
            data.label = label.into();
            data.text = value.into();
        }
        Field::Path {
            id, label, value, ..
        } => {
            data.id = id.into();
            data.kind = "path".into();
            data.label = label.into();
            data.text = value.into();
        }
        Field::List {
            id,
            label,
            items,
            placeholder,
        } => {
            data.id = id.into();
            data.kind = "list".into();
            data.label = label.into();
            // Items live in `options`; `text` carries the input placeholder.
            data.options = string_model(items);
            data.text = placeholder.into();
        }
        Field::Radio {
            id,
            label,
            options,
            selected,
        } => {
            data.id = id.into();
            data.kind = "radio".into();
            data.label = label.into();
            data.options = string_model(options);
            data.selected = *selected as i32;
        }
        Field::Combo {
            id,
            label,
            options,
            selected,
        } => {
            data.id = id.into();
            data.kind = "combo".into();
            data.label = label.into();
            data.options = string_model(options);
            data.selected = *selected as i32;
        }
        Field::Check { id, label, value } => {
            data.id = id.into();
            data.kind = "check".into();
            data.label = label.into();
            data.checked = *value;
        }
        Field::Section { label } => {
            data.kind = "section".into();
            data.label = label.into();
        }
    }
    data
}

fn string_model(items: &[String]) -> ModelRc<SharedString> {
    let rows: Vec<SharedString> = items.iter().map(SharedString::from).collect();
    ModelRc::from(Rc::new(VecModel::from(rows)))
}

fn read_values(fields: &[Field], model: &Rc<VecModel<FieldData>>) -> FormValues {
    let mut values = FormValues::default();
    for (idx, field) in fields.iter().enumerate() {
        let Some(id) = field.id() else { continue };
        let Some(row) = model.row_data(idx) else {
            continue;
        };
        let value = match field {
            Field::Text { .. } | Field::Path { .. } => FieldValue::Text(row.text.to_string()),
            Field::List { .. } => FieldValue::List(
                row.options
                    .iter()
                    .map(|s| s.to_string())
                    .filter(|s| !s.trim().is_empty())
                    .collect(),
            ),
            Field::Radio { .. } | Field::Combo { .. } => {
                FieldValue::Index(row.selected.max(0) as usize)
            }
            Field::Check { .. } => FieldValue::Bool(row.checked),
            Field::Section { .. } => continue,
        };
        values.0.insert(id.to_string(), value);
    }
    values
}

impl Renderer for SlintRenderer {
    fn run_form(&self, spec: FormSpec) -> FormOutcome {
        let ui = match MainForm::new() {
            Ok(ui) => ui,
            Err(_) => return FormOutcome::Cancel,
        };

        let rows: Vec<FieldData> = spec.fields.iter().map(to_field_data).collect();
        let model = Rc::new(VecModel::from(rows));
        ui.set_fields(ModelRc::from(model.clone()));
        ui.set_window_title(spec.window.title.clone().into());
        ui.set_submit_label(spec.submit_label.clone().into());
        ui.set_cancel_label(spec.cancel_label.clone().into());
        let extras: Vec<SharedString> = spec
            .extra_buttons
            .iter()
            .map(|b| SharedString::from(&b.label))
            .collect();
        ui.set_extra_buttons(ModelRc::from(Rc::new(VecModel::from(extras))));

        let decision = Rc::new(RefCell::new(None::<Decision>));

        {
            let decision = decision.clone();
            ui.on_submit(move || {
                *decision.borrow_mut() = Some(Decision::Submit);
                let _ = slint::quit_event_loop();
            });
        }
        {
            let decision = decision.clone();
            ui.on_cancel(move || {
                *decision.borrow_mut() = Some(Decision::Cancel);
                let _ = slint::quit_event_loop();
            });
        }
        {
            let decision = decision.clone();
            ui.on_extra(move |idx| {
                *decision.borrow_mut() = Some(Decision::Extra(idx.max(0) as usize));
                let _ = slint::quit_event_loop();
            });
        }
        {
            // Which fields are directory pickers (index-aligned with the model).
            let directory: Vec<bool> = spec
                .fields
                .iter()
                .map(|f| {
                    matches!(
                        f,
                        Field::Path {
                            directory: true,
                            ..
                        }
                    )
                })
                .collect();
            let model = model.clone();
            ui.on_browse(move |idx| {
                let i = idx.max(0) as usize;
                let picked = if directory.get(i).copied().unwrap_or(false) {
                    rfd::FileDialog::new().pick_folder()
                } else {
                    rfd::FileDialog::new().pick_file()
                };
                if let Some(path) = picked {
                    if let Some(mut row) = model.row_data(i) {
                        row.text = path.to_string_lossy().to_string().into();
                        model.set_row_data(i, row);
                    }
                }
            });
        }
        {
            let model = model.clone();
            ui.on_list_add(move |idx, value| {
                let value = value.trim();
                if value.is_empty() {
                    return;
                }
                let i = idx.max(0) as usize;
                if let Some(mut row) = model.row_data(i) {
                    let mut items: Vec<SharedString> = row.options.iter().collect();
                    items.push(value.into());
                    row.options = ModelRc::from(Rc::new(VecModel::from(items)));
                    model.set_row_data(i, row);
                }
            });
        }
        {
            let model = model.clone();
            ui.on_list_remove(move |idx, item| {
                let i = idx.max(0) as usize;
                let j = item.max(0) as usize;
                if let Some(mut row) = model.row_data(i) {
                    let mut items: Vec<SharedString> = row.options.iter().collect();
                    if j < items.len() {
                        items.remove(j);
                        row.options = ModelRc::from(Rc::new(VecModel::from(items)));
                        model.set_row_data(i, row);
                    }
                }
            });
        }

        if ui.run().is_err() {
            return FormOutcome::Cancel;
        }

        let values = read_values(&spec.fields, &model);
        let decision = decision.borrow().unwrap_or(Decision::Cancel);
        match decision {
            Decision::Submit => FormOutcome::Submit(values),
            Decision::Extra(i) => match spec.extra_buttons.get(i) {
                Some(btn) => FormOutcome::Button(btn.id.clone(), values),
                None => FormOutcome::Cancel,
            },
            Decision::Cancel => FormOutcome::Cancel,
        }
    }

    fn run_progress(&self, spec: ProgressSpec, channel: ProgressChannel) -> i32 {
        let ui = match ProgressView::new() {
            Ok(ui) => ui,
            Err(_) => return drain_headless(&channel),
        };
        ui.set_window_title(spec.window.title.clone().into());
        ui.set_status(spec.initial_status.clone().into());

        {
            let channel = channel.clone();
            ui.on_cancel(move || {
                channel.request_cancel();
                let _ = slint::quit_event_loop();
            });
        }

        let code = Rc::new(RefCell::new(None::<i32>));
        let timer = slint::Timer::default();
        {
            let ui_weak = ui.as_weak();
            let channel = channel.clone();
            let code = code.clone();
            timer.start(TimerMode::Repeated, Duration::from_millis(80), move || {
                let Some(ui) = ui_weak.upgrade() else { return };
                let mut done = false;
                while let Ok(update) = channel.updates.try_recv() {
                    match update {
                        ProgressUpdate::Percent(p) => ui.set_percent(p as f32),
                        ProgressUpdate::Status(s) => ui.set_status(s.into()),
                        ProgressUpdate::Eta(e) => ui.set_eta(
                            e.map(|x| format!("Time left: {x}"))
                                .unwrap_or_default()
                                .into(),
                        ),
                        ProgressUpdate::Queue(q) => ui.set_queue(q.into()),
                        ProgressUpdate::Log(_) => {}
                        ProgressUpdate::Done(c) => {
                            *code.borrow_mut() = Some(c);
                            done = true;
                            break;
                        }
                    }
                }
                if done {
                    let _ = slint::quit_event_loop();
                }
            });
        }

        if ui.run().is_err() {
            return drain_headless(&channel);
        }

        let result = *code.borrow();
        match result {
            Some(c) => c,
            None => {
                // Window closed before completion: treat as cancel.
                channel.request_cancel();
                1
            }
        }
    }

    fn message(&self, kind: MessageKind, title: &str, body: &str) {
        let Ok(ui) = MessageDialog::new() else {
            eprintln!("{title}: {body}");
            return;
        };
        ui.set_window_title(title.into());
        ui.set_body(body.into());
        ui.set_is_error(kind == MessageKind::Error);
        ui.on_dismiss(|| {
            let _ = slint::quit_event_loop();
        });
        let _ = ui.run();
    }

    fn name(&self) -> &'static str {
        "slint"
    }
}

/// Fallback when the GUI cannot be created: drain updates and return the code.
fn drain_headless(channel: &ProgressChannel) -> i32 {
    let mut code = 0;
    while let Ok(update) = channel.updates.recv() {
        if let ProgressUpdate::Done(c) = update {
            code = c;
            break;
        }
    }
    code
}
