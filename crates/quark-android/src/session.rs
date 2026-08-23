use std::sync::Mutex;

use quark_core::json;
use quark_core::session::{MainAction, MainSessionResult, SettingsForm};
use quark_gui::copy;
use quark_gui::event::{UiEffect, View};
use quark_gui::reduce::{self, UiState};

struct Session {
    state: UiState,
    default_dir: String,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

pub fn catalog_json() -> String {
    format!(
        "{{\"audio\":{},\"video\":{},\"spaces\":{},\"themes\":{},\"err_empty_queue\":{},\"err_empty_output\":{},\"err_empty_download_dir\":{}}}",
        str_array(quark_gui::AUDIO_FORMATS),
        str_array(quark_gui::VIDEO_FORMATS),
        str_array(quark_gui::SPACES),
        str_array(quark_gui::THEMES),
        json::stringify_str(copy::ERR_EMPTY_QUEUE),
        json::stringify_str(copy::ERR_EMPTY_OUTPUT),
        json::stringify_str(copy::ERR_EMPTY_DOWNLOAD_DIR),
    )
}

pub fn start(default_dir: &str, settings_json: &str) -> Result<String, String> {
    let settings = super::settings_from_json(settings_json)?;
    let form = SettingsForm::from_settings(&settings);
    let mut form = form;
    if form.download_dir.trim().is_empty() || form.download_dir == "~/Downloads" {
        form.download_dir = default_dir.to_string();
    }
    let state = UiState::new(default_dir, form);
    let snap = snapshot(&state);
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(Session {
        state,
        default_dir: default_dir.to_string(),
    });
    Ok(format!("{{\"state\":{snap},\"effects\":[]}}"))
}

pub fn dispatch(event_json: &str) -> Result<String, String> {
    let value = json::parse(event_json).map_err(|e| e.to_string())?;
    let events = quark_gui::script::events_from_value(&value)?;
    let mut guard = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    let session = guard
        .as_mut()
        .ok_or_else(|| "session not started".to_string())?;
    let mut effects = Vec::new();
    for event in events {
        if matches!(event, quark_gui::event::UiEvent::ResetSettings) {
            let mut draft = SettingsForm::defaults();
            draft.download_dir = session.default_dir.clone();
            session.state.draft = draft;
            continue;
        }
        effects.extend(reduce::reduce(&mut session.state, event));
    }
    let snap = snapshot(&session.state);
    let fx = effects
        .iter()
        .map(effect_json)
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("{{\"state\":{snap},\"effects\":[{fx}]}}"))
}

fn snapshot(state: &UiState) -> String {
    let queue = state
        .queue
        .iter()
        .map(|u| json::stringify_str(u))
        .collect::<Vec<_>>()
        .join(",");
    let formats = state
        .format_choices()
        .iter()
        .map(|f| json::stringify_str(f))
        .collect::<Vec<_>>()
        .join(",");
    let selected = match state.selected {
        Some(i) => i.to_string(),
        None => "null".into(),
    };
    let view = match state.view {
        View::Main => "main",
        View::Settings => "settings",
    };
    format!(
        "{{\"url_field\":{},\"queue\":[{queue}],\"selected\":{selected},\"media\":{},\"format\":{},\"output\":{},\"view\":{},\"settings_saved\":{},\"settings\":{},\"draft\":{},\"formats\":[{formats}]}}",
        json::stringify_str(&state.url_field),
        json::stringify_str(state.media.as_str()),
        json::stringify_str(&state.format),
        json::stringify_str(&state.output),
        json::stringify_str(view),
        if state.settings_saved {
            "true"
        } else {
            "false"
        },
        form_json(&state.settings),
        form_json(&state.draft),
    )
}

fn form_json(form: &SettingsForm) -> String {
    format!(
        "{{\"download_dir\":{},\"yt_dlp\":{},\"ffmpeg\":{},\"gui_download_mode\":{},\"download_logs\":{},\"open_output_dir\":{},\"gui_theme\":{},\"strip_video_ids\":{},\"sanitize_filenames\":{},\"filename_spaces\":{},\"playlist_folders\":{}}}",
        json::stringify_str(&form.download_dir),
        json::stringify_str(&form.yt_dlp),
        json::stringify_str(&form.ffmpeg),
        json::stringify_str(&form.gui_download_mode),
        form.download_logs,
        form.open_output_dir,
        json::stringify_str(&form.gui_theme),
        form.strip_video_ids,
        form.sanitize_filenames,
        json::stringify_str(&form.filename_spaces),
        form.playlist_folders,
    )
}

fn effect_json(effect: &UiEffect) -> String {
    match effect {
        UiEffect::Error(msg) => format!("{{\"error\":{}}}", json::stringify_str(msg)),
        UiEffect::ClearUrlField => "{\"clear_url\":true}".into(),
        UiEffect::ApplyTheme(theme) => {
            format!("{{\"apply_theme\":{}}}", json::stringify_str(theme))
        }
        UiEffect::Show(View::Main) => "{\"show\":\"main\"}".into(),
        UiEffect::Show(View::Settings) => "{\"show\":\"settings\"}".into(),
        UiEffect::Emit(result) => format!("{{\"emit\":{}}}", emit_json(result)),
    }
}

fn emit_json(result: &MainSessionResult) -> String {
    match &result.action {
        MainAction::Download(p) => quark_core::session::emit_json(
            "download",
            result.settings_form.as_ref(),
            &p.urls,
            &p.media_type,
            &p.format,
            &p.output_dir,
        ),
        MainAction::Cancel => {
            quark_core::session::emit_json("cancel", result.settings_form.as_ref(), &[], "", "", "")
        }
        MainAction::Error(message) => format!(
            "{{\"v\":{},\"action\":\"error\",\"message\":{}}}",
            quark_core::session::PROTOCOL_VERSION,
            json::stringify_str(message)
        ),
    }
}

fn str_array(items: &[&str]) -> String {
    let inner = items
        .iter()
        .map(|s| json::stringify_str(s))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{inner}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_add_and_download() {
        start("/sdcard/Download", "").unwrap();
        let out = dispatch(r#"{"add_url":"https://example.com/a"}"#).unwrap();
        assert!(out.contains("https://example.com/a"), "{out}");
        let out = dispatch(r#"{"download":true}"#).unwrap();
        assert!(out.contains("\"action\":\"download\""), "{out}");
    }

    #[test]
    fn catalog_lists_match_gui() {
        let json = catalog_json();
        assert!(json.contains("\"mp3\""));
        assert!(json.contains("\"mp4\""));
        assert!(json.contains(copy::ERR_EMPTY_QUEUE));
    }
}
