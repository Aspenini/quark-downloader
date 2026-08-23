use quark_core::MediaType;
use quark_core::json::{self, Value};
use quark_core::session::{self, MainAction, MainSessionResult, SettingsForm};

use crate::event::{UiEffect, UiEvent};
use crate::reduce::UiState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptOutput {
    pub result: MainSessionResult,
    pub validation_error: Option<String>,
}

impl ScriptOutput {
    pub fn to_json(&self) -> String {
        if let Some(message) = &self.validation_error {
            return error_json(message);
        }
        match &self.result.action {
            MainAction::Download(p) => session::emit_json(
                "download",
                self.result.settings_form.as_ref(),
                &p.urls,
                &p.media_type,
                &p.format,
                &p.output_dir,
            ),
            MainAction::Cancel => session::emit_json(
                "cancel",
                self.result.settings_form.as_ref(),
                &[],
                "",
                "",
                "",
            ),
            MainAction::Error(message) => error_json(message),
        }
    }

    pub fn exit_code(&self) -> i32 {
        if self.validation_error.is_some() {
            return 2;
        }
        match self.result.action {
            MainAction::Error(_) => 2,
            _ => 0,
        }
    }
}

fn error_json(message: &str) -> String {
    format!(
        "{{\"v\":{},\"action\":\"error\",\"message\":{}}}",
        session::PROTOCOL_VERSION,
        json::stringify_str(message)
    )
}

pub fn run(input: &str) -> Result<ScriptOutput, String> {
    let data = json::parse(input).map_err(|e| e.to_string())?;
    let args = data.get("args");
    let default_dir = args
        .and_then(|a| a.get_str("default_dir"))
        .unwrap_or("/tmp/downloads");
    let settings = args
        .and_then(|a| a.get("settings"))
        .and_then(settings_from_value)
        .unwrap_or_else(default_script_settings);
    let mut state = UiState::new(default_dir, settings);
    let events = data
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| "script missing events array".to_string())?;

    let mut last_emit = None;
    let mut last_error = None;
    for event_value in events {
        for event in events_from_value(event_value)? {
            for effect in crate::reduce::reduce(&mut state, event) {
                match effect {
                    UiEffect::Error(msg) => last_error = Some(msg),
                    UiEffect::Emit(result) => {
                        last_error = None;
                        last_emit = Some(result);
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(message) = last_error {
        return Ok(ScriptOutput {
            result: MainSessionResult::error(message.clone()),
            validation_error: Some(message),
        });
    }
    Ok(ScriptOutput {
        result: last_emit
            .map(|result| *result)
            .unwrap_or_else(|| MainSessionResult {
                action: MainAction::Cancel,
                settings_form: state.settings_saved.then_some(state.settings),
            }),
        validation_error: None,
    })
}

fn default_script_settings() -> SettingsForm {
    SettingsForm::defaults()
}

pub fn settings_from_value(value: &Value) -> Option<SettingsForm> {
    Some(SettingsForm::from_strings(
        value.get_str("download_dir").unwrap_or("~/Downloads"),
        value.get_str("yt_dlp").unwrap_or("path"),
        value.get_str("ffmpeg").unwrap_or("path"),
        value.get_str("gui_download_mode").unwrap_or("progress"),
        &value
            .get("download_logs")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        value.get_str("gui_theme").unwrap_or("system"),
        &value
            .get("strip_video_ids")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        &value
            .get("sanitize_filenames")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        value.get_str("filename_spaces").unwrap_or("keep"),
        &value
            .get("playlist_folders")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        &value
            .get("open_output_dir")
            .map(Value::raw_display)
            .unwrap_or_else(|| "false".into()),
    ))
}

pub fn events_from_value(value: &Value) -> Result<Vec<UiEvent>, String> {
    if let Some(url) = value.get_str("add_url") {
        return Ok(vec![UiEvent::SetUrlField(url.to_string()), UiEvent::AddUrl]);
    }
    if let Some(text) = value.get_str("paste") {
        return Ok(vec![UiEvent::PasteUrls(text.to_string())]);
    }
    if value.get_bool("remove_selected") == Some(true) {
        return Ok(vec![UiEvent::RemoveSelected]);
    }
    if let Some(idx) = value.get_i32("select") {
        return Ok(vec![UiEvent::SelectIndex(Some(idx as usize))]);
    }
    if let Some(media) = value.get_str("set_media") {
        return Ok(vec![UiEvent::SetMedia(MediaType::parse(media)?)]);
    }
    if let Some(format) = value.get_str("set_format") {
        return Ok(vec![UiEvent::SetFormat(format.to_string())]);
    }
    if let Some(output) = value.get_str("set_output") {
        return Ok(vec![UiEvent::SetOutput(output.to_string())]);
    }
    if let Some(url) = value.get_str("set_url_field") {
        return Ok(vec![UiEvent::SetUrlField(url.to_string())]);
    }
    if value.get_bool("download") == Some(true) {
        return Ok(vec![UiEvent::Download]);
    }
    if value.get_bool("cancel") == Some(true) {
        return Ok(vec![UiEvent::Cancel]);
    }
    if value.get_bool("close") == Some(true) {
        return Ok(vec![UiEvent::Close]);
    }
    if value.get_bool("open_settings") == Some(true) {
        return Ok(vec![UiEvent::OpenSettings]);
    }
    if value.get_bool("close_settings") == Some(true) {
        return Ok(vec![UiEvent::CloseSettings]);
    }
    if value.get_bool("save_settings") == Some(true) {
        return Ok(vec![UiEvent::SaveSettings]);
    }
    if value.get_bool("reset_settings") == Some(true) {
        return Ok(vec![UiEvent::ResetSettings]);
    }
    if let Some(settings) = value.get("set_setting") {
        let form = settings_from_value(settings)
            .ok_or_else(|| "invalid set_setting object".to_string())?;
        return Ok(vec![UiEvent::DraftSettings(form)]);
    }
    Err(format!("unrecognized event: {value}"))
}
