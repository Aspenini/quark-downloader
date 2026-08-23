use quark_core::MediaType;
use quark_core::session::SettingsForm;

/// Every frontend must bind each of these. Matching on `UiEvent` exhaustively
/// is the compile-time capability check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEvent {
    SetUrlField(String),
    AddUrl,
    PasteUrls(String),
    RemoveSelected,
    SelectIndex(Option<usize>),
    SetMedia(MediaType),
    SetFormat(String),
    SetOutput(String),
    Download,
    Cancel,
    Close,
    OpenSettings,
    CloseSettings,
    DraftSettings(SettingsForm),
    ResetSettings,
    SaveSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Main,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEffect {
    Error(String),
    ClearUrlField,
    ApplyTheme(String),
    Show(View),
    Emit(Box<quark_core::session::MainSessionResult>),
}

pub const REQUIRED_ACTIONS: &[fn() -> UiEvent] = &[
    || UiEvent::AddUrl,
    || UiEvent::PasteUrls(String::new()),
    || UiEvent::RemoveSelected,
    || UiEvent::Download,
    || UiEvent::Cancel,
    || UiEvent::Close,
    || UiEvent::OpenSettings,
    || UiEvent::CloseSettings,
    || UiEvent::ResetSettings,
    || UiEvent::SaveSettings,
];
