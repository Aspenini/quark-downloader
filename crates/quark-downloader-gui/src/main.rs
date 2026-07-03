//! `quark-downloader-gui` — collects options via QuarkGUI and drives the
//! quark-core engine in-process. The active native backend is chosen from the
//! `gui_backend` setting.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod download;
mod terminal;

use quark_core::config::{
    self, FilenameSpaces, GuiBackend, GuiDownloadMode, GuiTheme, Settings, ToolSource,
};
use quark_core::download::command::MediaType;
use quark_core::version;
use quark_gui::model::{
    ExtraButton, Field, FormOutcome, FormSpec, FormValues, MessageKind, Theme, WindowSpec,
};
use quark_gui::{App, Backend};

use download::DownloadParams;

const VIDEO_FORMAT_OPTIONS: &[&str] = &["original", "mp4", "mkv", "webm"];
const AUDIO_FORMAT_OPTIONS: &[&str] = &["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"];
const TOOL_OPTIONS: &[&str] = &["auto", "path", "bundled"];
const SPACES_OPTIONS: &[&str] = &["keep", "underscore", "dash", "remove"];
#[cfg(windows)]
const BACKEND_OPTIONS: &[&str] = &["win32", "kirigami"];
#[cfg(target_os = "macos")]
const BACKEND_OPTIONS: &[&str] = &["cocoa", "kirigami", "gtk"];
#[cfg(target_os = "linux")]
const BACKEND_OPTIONS: &[&str] = &["gtk", "kirigami"];
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
const BACKEND_OPTIONS: &[&str] = &["kirigami"];
const MODE_OPTIONS: &[&str] = &["progress", "external_cli"];
const THEME_OPTIONS: &[&str] = &["light", "dark"];

fn main() {
    let mut settings = config::load(true).unwrap_or_default();
    let backend = std::env::var("QUARK_GUI_BACKEND")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| settings.gui_backend.as_str().to_owned());
    let app = App::new(Backend::from_name(&backend));

    loop {
        let theme = ui_theme(&settings);
        match app.run_form(main_form(&settings, theme)) {
            FormOutcome::Submit(values) => {
                let params = parse_main(&values);
                if params.urls.is_empty() {
                    app.message(
                        MessageKind::Error,
                        "Quark Downloader",
                        "Add at least one URL.",
                    );
                    continue;
                }
                download::run(&app, &settings, params, theme);
                return;
            }
            FormOutcome::Button(id, _) if id == "settings" => {
                if let FormOutcome::Submit(values) = app.run_form(settings_form(&settings, theme)) {
                    settings = apply_settings(&values);
                    let _ = config::save(&settings);
                }
            }
            FormOutcome::Button(_, _) => {}
            FormOutcome::Cancel => return,
        }
    }
}

fn ui_theme(settings: &Settings) -> Theme {
    match settings.gui_theme {
        GuiTheme::Dark => Theme::Dark,
        GuiTheme::Light => Theme::Light,
    }
}

// ---- main form -----------------------------------------------------------

fn main_form(settings: &Settings, theme: Theme) -> FormSpec {
    let default_dir = settings
        .resolved_download_dir(&quark_core::paths::default_downloads_dir())
        .to_string_lossy()
        .to_string();

    let mut window = WindowSpec::new(version::window_title(), theme);
    window.width = 480.0;
    window.height = 430.0;
    let mut form = FormSpec::new(window);
    form.submit_label = "Download".into();
    form.cancel_label = "Close".into();
    form.extra_buttons.push(ExtraButton {
        id: "settings".into(),
        label: "⚙".into(),
    });
    form.fields = vec![
        Field::List {
            id: "urls".into(),
            label: "Video or playlist URL".into(),
            items: Vec::new(),
            placeholder: "https://...".into(),
        },
        Field::Radio {
            id: "media_type".into(),
            label: String::new(),
            options: vec!["video".into(), "audio".into()],
            selected: 0,
        },
        Field::DependentCombo {
            id: "format".into(),
            label: "Format".into(),
            controller: "media_type".into(),
            option_sets: vec![
                VIDEO_FORMAT_OPTIONS.iter().map(|s| s.to_string()).collect(),
                AUDIO_FORMAT_OPTIONS.iter().map(|s| s.to_string()).collect(),
            ],
            selected: 0,
        },
        Field::Path {
            id: "output_dir".into(),
            label: "Output folder".into(),
            value: default_dir,
            directory: true,
        },
    ];
    form
}

fn parse_main(values: &FormValues) -> DownloadParams {
    let media_type = if values.index("media_type") == 0 {
        MediaType::Video
    } else {
        MediaType::Audio
    };
    let format_options = if media_type == MediaType::Audio {
        AUDIO_FORMAT_OPTIONS
    } else {
        VIDEO_FORMAT_OPTIONS
    };
    let format = format_options
        .get(values.index("format"))
        .copied()
        .unwrap_or("original")
        .to_string();
    DownloadParams {
        urls: values.list("urls"),
        media_type,
        format,
        output_dir: values.text("output_dir"),
    }
}

// ---- settings form -------------------------------------------------------

fn settings_form(settings: &Settings, theme: Theme) -> FormSpec {
    let backend_options = available_backend_options();
    let mut window = WindowSpec::new(version::settings_window_title(), theme);
    window.width = 480.0;
    window.height = 600.0;
    let mut form = FormSpec::new(window);
    form.submit_label = "Save".into();
    form.cancel_label = "Cancel".into();
    form.fields = vec![
        Field::Section {
            label: "General".into(),
        },
        Field::Path {
            id: "download_dir".into(),
            label: "Default download folder".into(),
            value: settings.download_dir.clone(),
            directory: true,
        },
        combo(
            "gui_theme",
            "Theme",
            THEME_OPTIONS,
            settings.gui_theme.as_str(),
        ),
        combo(
            "gui_backend",
            "GUI backend (applies next launch)",
            &backend_options,
            settings.gui_backend.as_str(),
        ),
        Field::Section {
            label: "Download Naming".into(),
        },
        Field::Check {
            id: "strip_video_ids".into(),
            label: "Strip [VIDEOID] from names".into(),
            value: settings.strip_video_ids,
        },
        Field::Check {
            id: "sanitize_filenames".into(),
            label: "ASCII-safe filenames".into(),
            value: settings.sanitize_filenames,
        },
        combo(
            "filename_spaces",
            "Spaces in names",
            SPACES_OPTIONS,
            settings.filename_spaces.as_str(),
        ),
        Field::Check {
            id: "playlist_folders".into(),
            label: "Playlist folders".into(),
            value: settings.playlist_folders,
        },
        Field::Section {
            label: "Downloads".into(),
        },
        combo(
            "gui_download_mode",
            "Download window",
            MODE_OPTIONS,
            settings.gui_download_mode.as_str(),
        ),
        Field::Check {
            id: "download_logs".into(),
            label: "Create download logs".into(),
            value: settings.download_logs,
        },
        Field::Section {
            label: "Tools".into(),
        },
        combo(
            "yt_dlp",
            "yt-dlp source",
            TOOL_OPTIONS,
            settings.yt_dlp.as_str(),
        ),
        combo(
            "ffmpeg",
            "ffmpeg source",
            TOOL_OPTIONS,
            settings.ffmpeg.as_str(),
        ),
    ];
    form
}

fn combo(id: &str, label: &str, options: &[&str], current: &str) -> Field {
    let selected = options.iter().position(|o| *o == current).unwrap_or(0);
    Field::Combo {
        id: id.into(),
        label: label.into(),
        options: options.iter().map(|s| s.to_string()).collect(),
        selected,
    }
}

fn apply_settings(values: &FormValues) -> Settings {
    let backend_options = available_backend_options();
    Settings {
        download_dir: values.text("download_dir"),
        yt_dlp: ToolSource::parse_lenient(option_at(values, "yt_dlp", TOOL_OPTIONS)).0,
        ffmpeg: ToolSource::parse_lenient(option_at(values, "ffmpeg", TOOL_OPTIONS)).0,
        gui_backend: GuiBackend::parse_lenient(option_at(values, "gui_backend", &backend_options))
            .0,
        gui_download_mode: GuiDownloadMode::parse_lenient(option_at(
            values,
            "gui_download_mode",
            MODE_OPTIONS,
        ))
        .0,
        download_logs: values.bool("download_logs"),
        gui_theme: GuiTheme::parse_lenient(option_at(values, "gui_theme", THEME_OPTIONS)).0,
        strip_video_ids: values.bool("strip_video_ids"),
        sanitize_filenames: values.bool("sanitize_filenames"),
        filename_spaces: FilenameSpaces::parse_lenient(option_at(
            values,
            "filename_spaces",
            SPACES_OPTIONS,
        ))
        .0,
        playlist_folders: values.bool("playlist_folders"),
    }
}

fn available_backend_options() -> Vec<&'static str> {
    BACKEND_OPTIONS
        .iter()
        .copied()
        .filter(|name| Backend::from_name(name).available())
        .collect()
}

fn option_at<'a>(values: &FormValues, id: &str, options: &'a [&'a str]) -> &'a str {
    options.get(values.index(id)).copied().unwrap_or(options[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use quark_gui::model::FieldValue;

    #[test]
    fn format_options_follow_media_type() {
        let form = main_form(&Settings::default(), Theme::Light);
        let format = form
            .fields
            .iter()
            .find(|field| field.id() == Some("format"))
            .expect("format field");
        let Field::DependentCombo {
            controller,
            option_sets,
            ..
        } = format
        else {
            panic!("format must be a dependent combo");
        };
        assert_eq!(controller, "media_type");
        assert_eq!(
            option_sets[0],
            VIDEO_FORMAT_OPTIONS
                .iter()
                .map(|format| format.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            option_sets[1],
            AUDIO_FORMAT_OPTIONS
                .iter()
                .map(|format| format.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn selected_format_is_read_from_the_active_media_type() {
        let mut values = FormValues::default();
        values.0.insert("media_type".into(), FieldValue::Index(1));
        values.0.insert("format".into(), FieldValue::Index(1));
        assert_eq!(parse_main(&values).format, "mp3");

        values.0.insert("media_type".into(), FieldValue::Index(0));
        assert_eq!(parse_main(&values).format, "mp4");
    }
}
