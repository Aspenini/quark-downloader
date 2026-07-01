//! A non-graphical renderer that accepts form defaults and drains progress
//! updates. Useful for tests, CI, and proving the abstraction without a
//! display. Also the ultimate fallback when no GUI toolkit is available.

use crate::backend::Renderer;
use crate::event::ProgressChannel;
use crate::model::{
    Field, FieldValue, FormOutcome, FormSpec, FormValues, MessageKind, ProgressSpec,
};

#[derive(Default)]
pub struct HeadlessRenderer;

impl Renderer for HeadlessRenderer {
    fn run_form(&self, spec: FormSpec) -> FormOutcome {
        let mut values = FormValues::default();
        for field in &spec.fields {
            let Some(id) = field.id() else { continue };
            let value = match field {
                Field::Text { value, .. } | Field::Path { value, .. } => {
                    FieldValue::Text(value.clone())
                }
                Field::List { items, .. } => FieldValue::List(items.clone()),
                Field::Radio { selected, .. } | Field::Combo { selected, .. } => {
                    FieldValue::Index(*selected)
                }
                Field::Check { value, .. } => FieldValue::Bool(*value),
                Field::Section { .. } => continue,
            };
            values.0.insert(id.to_string(), value);
        }
        FormOutcome::Submit(values)
    }

    fn run_progress(&self, _spec: ProgressSpec, channel: ProgressChannel) -> i32 {
        let mut code = 0;
        while let Ok(update) = channel.updates.recv() {
            if let crate::event::ProgressUpdate::Done(c) = update {
                code = c;
                break;
            }
        }
        code
    }

    fn message(&self, kind: MessageKind, title: &str, body: &str) {
        let tag = match kind {
            MessageKind::Info => "info",
            MessageKind::Error => "error",
        };
        eprintln!("[{tag}] {title}: {body}");
    }

    fn name(&self) -> &'static str {
        "headless"
    }
}
