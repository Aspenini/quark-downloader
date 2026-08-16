use quark_core::session::{DownloadParams, MainAction, MainSessionResult, SettingsForm};
use quark_core::{Format, MediaType};

use crate::copy;
use crate::event::{UiEffect, UiEvent, View};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiState {
    pub url_field: String,
    pub queue: Vec<String>,
    pub selected: Option<usize>,
    pub media: MediaType,
    pub format: String,
    pub output: String,
    pub settings: SettingsForm,
    pub draft: SettingsForm,
    pub settings_saved: bool,
    pub view: View,
}

impl UiState {
    pub fn new(default_dir: &str, settings: SettingsForm) -> Self {
        Self {
            url_field: String::new(),
            queue: Vec::new(),
            selected: None,
            media: MediaType::Video,
            format: Format::Original.as_str().into(),
            output: default_dir.to_string(),
            draft: settings.clone(),
            settings,
            settings_saved: false,
            view: View::Main,
        }
    }

    pub fn format_choices(&self) -> &'static [&'static str] {
        crate::catalog::formats_for(self.media)
    }

    fn settings_if_saved(&self) -> Option<&SettingsForm> {
        self.settings_saved.then_some(&self.settings)
    }
}

pub fn reduce(state: &mut UiState, event: UiEvent) -> Vec<UiEffect> {
    match event {
        UiEvent::SetUrlField(value) => {
            state.url_field = value;
            Vec::new()
        }
        UiEvent::AddUrl => add_url_field(state),
        UiEvent::PasteUrls(text) => paste_urls(state, &text),
        UiEvent::RemoveSelected => {
            if let Some(idx) = state.selected
                && idx < state.queue.len()
            {
                state.queue.remove(idx);
                state.selected = None;
            }
            Vec::new()
        }
        UiEvent::SelectIndex(idx) => {
            state.selected = idx.filter(|i| *i < state.queue.len());
            Vec::new()
        }
        UiEvent::SetMedia(media) => {
            state.media = media;
            state.format = Format::Original.as_str().into();
            Vec::new()
        }
        UiEvent::SetFormat(value) => match Format::parse_for(state.media, &value) {
            Ok(fmt) => {
                state.format = fmt.as_str().into();
                Vec::new()
            }
            Err(msg) => vec![UiEffect::Error(msg)],
        },
        UiEvent::SetOutput(value) => {
            state.output = value;
            Vec::new()
        }
        UiEvent::Download => download(state),
        UiEvent::Cancel | UiEvent::Close => vec![UiEffect::Emit(emit_cancel(state))],
        UiEvent::OpenSettings => {
            state.draft = state.settings.clone();
            state.view = View::Settings;
            vec![UiEffect::Show(View::Settings)]
        }
        UiEvent::CloseSettings => {
            state.draft = state.settings.clone();
            state.view = View::Main;
            vec![UiEffect::Show(View::Main)]
        }
        UiEvent::DraftSettings(form) => {
            state.draft = form;
            Vec::new()
        }
        UiEvent::SaveSettings => save_settings(state),
    }
}

fn add_url_field(state: &mut UiState) -> Vec<UiEffect> {
    let url = state.url_field.trim().to_string();
    if url.is_empty() {
        return Vec::new();
    }
    if !state.queue.iter().any(|u| u == &url) {
        state.queue.push(url);
    }
    state.url_field.clear();
    vec![UiEffect::ClearUrlField]
}

fn paste_urls(state: &mut UiState, text: &str) -> Vec<UiEffect> {
    for piece in text.split_whitespace() {
        let url = piece.trim();
        if url.is_empty() {
            continue;
        }
        if !state.queue.iter().any(|u| u == url) {
            state.queue.push(url.to_string());
        }
    }
    state.url_field.clear();
    vec![UiEffect::ClearUrlField]
}

fn download(state: &mut UiState) -> Vec<UiEffect> {
    let mut effects = add_url_field(state);
    if state.queue.is_empty() {
        effects.push(UiEffect::Error(copy::ERR_EMPTY_QUEUE.into()));
        return effects;
    }
    let output = state.output.trim();
    if output.is_empty() {
        effects.push(UiEffect::Error(copy::ERR_EMPTY_OUTPUT.into()));
        return effects;
    }
    effects.push(UiEffect::Emit(MainSessionResult {
        action: MainAction::Download(DownloadParams {
            urls: state.queue.clone(),
            media_type: state.media.as_str().into(),
            format: state.format.clone(),
            output_dir: output.to_string(),
        }),
        settings_form: state.settings_if_saved().cloned(),
    }));
    effects
}

fn emit_cancel(state: &UiState) -> MainSessionResult {
    MainSessionResult {
        action: MainAction::Cancel,
        settings_form: state.settings_if_saved().cloned(),
    }
}

fn save_settings(state: &mut UiState) -> Vec<UiEffect> {
    if state.draft.download_dir.trim().is_empty() {
        return vec![UiEffect::Error(copy::ERR_EMPTY_DOWNLOAD_DIR.into())];
    }
    state.settings = state.draft.clone();
    state.settings_saved = true;
    state.view = View::Main;
    vec![
        UiEffect::ApplyTheme(state.settings.gui_theme.clone()),
        UiEffect::Show(View::Main),
    ]
}
