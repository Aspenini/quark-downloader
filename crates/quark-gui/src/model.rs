//! The backend-agnostic UI description. You build these once; a [`Renderer`]
//! turns them into real widgets. Kept deliberately small and form-oriented so
//! it maps cleanly onto Slint and every native toolkit.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn is_dark(self) -> bool {
        matches!(self, Theme::Dark)
    }
}

/// Top-level window attributes shared by every view.
#[derive(Debug, Clone)]
pub struct WindowSpec {
    pub title: String,
    pub theme: Theme,
    pub width: f32,
    pub height: f32,
}

impl WindowSpec {
    pub fn new(title: impl Into<String>, theme: Theme) -> Self {
        WindowSpec {
            title: title.into(),
            theme,
            width: 560.0,
            height: 520.0,
        }
    }
}

/// A single input in a form. Each carries a stable `id` used to read its value
/// back from [`FormValues`].
#[derive(Debug, Clone)]
pub enum Field {
    /// Single-line text.
    Text {
        id: String,
        label: String,
        value: String,
    },
    /// An editable list of strings with add/remove (e.g. a URL queue).
    List {
        id: String,
        label: String,
        items: Vec<String>,
        placeholder: String,
    },
    /// Mutually exclusive options.
    Radio {
        id: String,
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    /// A dropdown.
    Combo {
        id: String,
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    /// A boolean toggle.
    Check {
        id: String,
        label: String,
        value: bool,
    },
    /// A path with a native browse button.
    Path {
        id: String,
        label: String,
        value: String,
        directory: bool,
    },
    /// A non-interactive section heading.
    Section { label: String },
}

impl Field {
    pub fn id(&self) -> Option<&str> {
        match self {
            Field::Text { id, .. }
            | Field::List { id, .. }
            | Field::Radio { id, .. }
            | Field::Combo { id, .. }
            | Field::Check { id, .. }
            | Field::Path { id, .. } => Some(id),
            Field::Section { .. } => None,
        }
    }
}

/// An extra button beyond submit/cancel (e.g. a settings gear).
#[derive(Debug, Clone)]
pub struct ExtraButton {
    pub id: String,
    pub label: String,
}

/// A complete form to present.
#[derive(Debug, Clone)]
pub struct FormSpec {
    pub window: WindowSpec,
    pub fields: Vec<Field>,
    pub submit_label: String,
    pub cancel_label: String,
    pub extra_buttons: Vec<ExtraButton>,
}

impl FormSpec {
    pub fn new(window: WindowSpec) -> Self {
        FormSpec {
            window,
            fields: Vec::new(),
            submit_label: "OK".into(),
            cancel_label: "Cancel".into(),
            extra_buttons: Vec::new(),
        }
    }
}

/// The value of one field after the user submits.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(String),
    List(Vec<String>),
    Index(usize),
    Bool(bool),
}

impl FieldValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            FieldValue::Text(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            FieldValue::List(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_index(&self) -> Option<usize> {
        match self {
            FieldValue::Index(i) => Some(*i),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FieldValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// All field values keyed by id.
#[derive(Debug, Clone, Default)]
pub struct FormValues(pub HashMap<String, FieldValue>);

impl FormValues {
    pub fn get(&self, id: &str) -> Option<&FieldValue> {
        self.0.get(id)
    }
    pub fn text(&self, id: &str) -> String {
        self.get(id)
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string()
    }
    pub fn list(&self, id: &str) -> Vec<String> {
        self.get(id)
            .and_then(|v| v.as_list())
            .map(|s| s.to_vec())
            .unwrap_or_default()
    }
    pub fn index(&self, id: &str) -> usize {
        self.get(id).and_then(|v| v.as_index()).unwrap_or(0)
    }
    pub fn bool(&self, id: &str) -> bool {
        self.get(id).and_then(|v| v.as_bool()).unwrap_or(false)
    }
}

/// What the user did with a form.
#[derive(Debug, Clone)]
pub enum FormOutcome {
    Submit(FormValues),
    /// An extra button was pressed; values captured at that moment.
    Button(String, FormValues),
    Cancel,
}

/// A progress window's static labels.
#[derive(Debug, Clone)]
pub struct ProgressSpec {
    pub window: WindowSpec,
    pub initial_status: String,
}

/// Severity of a message dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Info,
    Error,
}
