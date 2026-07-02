//! The backend-agnostic UI description. You build these once; a
//! [`Renderer`](crate::backend::Renderer) turns them into real widgets. Kept
//! deliberately small and form-oriented so it maps cleanly onto Slint and
//! every native toolkit.

use std::collections::HashMap;

/// Light or dark rendering for a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Light backgrounds, dark text.
    Light,
    /// Dark backgrounds, light text.
    Dark,
}

impl Theme {
    /// Whether this is [`Theme::Dark`].
    pub fn is_dark(self) -> bool {
        matches!(self, Theme::Dark)
    }
}

/// Top-level window attributes shared by every view.
#[derive(Debug, Clone)]
pub struct WindowSpec {
    /// Titlebar text.
    pub title: String,
    /// Light or dark rendering.
    pub theme: Theme,
    /// Preferred content width in logical pixels.
    pub width: f32,
    /// Preferred content height in logical pixels.
    pub height: f32,
}

impl WindowSpec {
    /// A window spec with the default 560×520 size.
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
        /// Key for reading the value back.
        id: String,
        /// Label shown beside the input.
        label: String,
        /// Initial text.
        value: String,
    },
    /// An editable list of strings with add/remove (e.g. a URL queue).
    List {
        /// Key for reading the value back.
        id: String,
        /// Label shown above the list.
        label: String,
        /// Initial items.
        items: Vec<String>,
        /// Hint text for the empty entry row (backends may ignore it).
        placeholder: String,
    },
    /// Mutually exclusive options.
    Radio {
        /// Key for reading the value back.
        id: String,
        /// Label shown above the group.
        label: String,
        /// One entry per radio button.
        options: Vec<String>,
        /// Index of the initially selected option.
        selected: usize,
    },
    /// A dropdown.
    Combo {
        /// Key for reading the value back.
        id: String,
        /// Label shown beside the dropdown.
        label: String,
        /// One entry per dropdown item.
        options: Vec<String>,
        /// Index of the initially selected item.
        selected: usize,
    },
    /// A dropdown whose options follow the selected index of another field.
    DependentCombo {
        /// Key for reading the value back.
        id: String,
        /// Label shown beside the dropdown.
        label: String,
        /// ID of the controlling radio field.
        controller: String,
        /// One option list per controller selection.
        option_sets: Vec<Vec<String>>,
        /// Index of the initially selected item.
        selected: usize,
    },
    /// A boolean toggle.
    Check {
        /// Key for reading the value back.
        id: String,
        /// The checkbox's own label.
        label: String,
        /// Initial checked state.
        value: bool,
    },
    /// A path with a native browse button.
    Path {
        /// Key for reading the value back.
        id: String,
        /// Label shown beside the input.
        label: String,
        /// Initial path text.
        value: String,
        /// `true` to pick directories, `false` to pick files.
        directory: bool,
    },
    /// A non-interactive section heading.
    Section {
        /// Heading text.
        label: String,
    },
}

impl Field {
    /// The field's readback key, or `None` for non-input fields.
    pub fn id(&self) -> Option<&str> {
        match self {
            Field::Text { id, .. }
            | Field::List { id, .. }
            | Field::Radio { id, .. }
            | Field::Combo { id, .. }
            | Field::DependentCombo { id, .. }
            | Field::Check { id, .. }
            | Field::Path { id, .. } => Some(id),
            Field::Section { .. } => None,
        }
    }

    /// Resolve this field's initial selected index, when it has one.
    pub fn selected_index(&self) -> Option<usize> {
        match self {
            Field::Radio { selected, .. }
            | Field::Combo { selected, .. }
            | Field::DependentCombo { selected, .. } => Some(*selected),
            _ => None,
        }
    }
}

/// An extra button beyond submit/cancel (e.g. a settings gear).
#[derive(Debug, Clone)]
pub struct ExtraButton {
    /// Reported through [`FormOutcome::Button`] when pressed.
    pub id: String,
    /// Button text.
    pub label: String,
}

/// A complete form to present.
#[derive(Debug, Clone)]
pub struct FormSpec {
    /// Window attributes.
    pub window: WindowSpec,
    /// Inputs, in display order.
    pub fields: Vec<Field>,
    /// Text of the confirming button (default "OK").
    pub submit_label: String,
    /// Text of the dismissing button (default "Cancel").
    pub cancel_label: String,
    /// Additional buttons beside submit/cancel.
    pub extra_buttons: Vec<ExtraButton>,
}

impl FormSpec {
    /// An empty form with "OK"/"Cancel" buttons.
    pub fn new(window: WindowSpec) -> Self {
        FormSpec {
            window,
            fields: Vec::new(),
            submit_label: "OK".into(),
            cancel_label: "Cancel".into(),
            extra_buttons: Vec::new(),
        }
    }

    /// Initial selected index for an input field, defaulting to zero.
    pub fn selected_index(&self, id: &str) -> usize {
        self.fields
            .iter()
            .find(|field| field.id() == Some(id))
            .and_then(Field::selected_index)
            .unwrap_or(0)
    }
}

/// The value of one field after the user submits.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// From [`Field::Text`] and [`Field::Path`].
    Text(String),
    /// From [`Field::List`].
    List(Vec<String>),
    /// From radio and combo fields: the selected index.
    Index(usize),
    /// From [`Field::Check`].
    Bool(bool),
}

impl FieldValue {
    /// The text, if this is a [`FieldValue::Text`].
    pub fn as_text(&self) -> Option<&str> {
        match self {
            FieldValue::Text(s) => Some(s),
            _ => None,
        }
    }
    /// The items, if this is a [`FieldValue::List`].
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            FieldValue::List(v) => Some(v),
            _ => None,
        }
    }
    /// The selected index, if this is a [`FieldValue::Index`].
    pub fn as_index(&self) -> Option<usize> {
        match self {
            FieldValue::Index(i) => Some(*i),
            _ => None,
        }
    }
    /// The flag, if this is a [`FieldValue::Bool`].
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
    /// The raw value for `id`, if present.
    pub fn get(&self, id: &str) -> Option<&FieldValue> {
        self.0.get(id)
    }
    /// The text for `id`, or `""` when absent or not text.
    pub fn text(&self, id: &str) -> String {
        self.get(id)
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string()
    }
    /// The items for `id`, or empty when absent or not a list.
    pub fn list(&self, id: &str) -> Vec<String> {
        self.get(id)
            .and_then(|v| v.as_list())
            .map(|s| s.to_vec())
            .unwrap_or_default()
    }
    /// The selected index for `id`, or `0` when absent or not an index.
    pub fn index(&self, id: &str) -> usize {
        self.get(id).and_then(|v| v.as_index()).unwrap_or(0)
    }
    /// The flag for `id`, or `false` when absent or not a bool.
    pub fn bool(&self, id: &str) -> bool {
        self.get(id).and_then(|v| v.as_bool()).unwrap_or(false)
    }
}

/// What the user did with a form.
#[derive(Debug, Clone)]
pub enum FormOutcome {
    /// The submit button was pressed; all values captured.
    Submit(FormValues),
    /// An extra button was pressed; values captured at that moment.
    Button(String, FormValues),
    /// The form was dismissed without submitting.
    Cancel,
}

/// A progress window's static labels.
#[derive(Debug, Clone)]
pub struct ProgressSpec {
    /// Window attributes.
    pub window: WindowSpec,
    /// Status line shown before the first update arrives.
    pub initial_status: String,
}

/// Severity of a message dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// Informational.
    Info,
    /// An error.
    Error,
}
