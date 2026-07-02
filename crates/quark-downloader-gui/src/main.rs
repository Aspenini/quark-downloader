//! `quark-downloader-gui` — collects options via QuarkGUI and drives the
//! quark-core engine in-process. Slint is the default backend; the active
//! backend is chosen from the `gui_backend` setting.

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

const FORMAT_OPTIONS: &[&str] = &[
    "original", "mp4", "mkv", "webm", "mp3", "m4a", "flac", "wav", "opus", "vorbis",
];
const TOOL_OPTIONS: &[&str] = &["auto", "path", "bundled"];
const SPACES_OPTIONS: &[&str] = &["keep", "underscore", "dash", "remove"];
const BACKEND_OPTIONS: &[&str] = &["slint", "win32", "cocoa", "gtk", "kirigami"];
const MODE_OPTIONS: &[&str] = &["progress", "external_cli"];
const THEME_OPTIONS: &[&str] = &["light", "dark"];

fn main() {
    let mut settings = config::load(true).unwrap_or_default();
    let app = App::new(Backend::from_name(settings.gui_backend.as_str()));

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

    let mut form = FormSpec::new(WindowSpec::new(version::window_title(), theme));
    form.submit_label = "Download".into();
    form.cancel_label = "Quit".into();
    form.extra_buttons.push(ExtraButton {
        id: "settings".into(),
        label: "Settings".into(),
    });
    form.fields = vec![
        Field::List {
            id: "urls".into(),
            label: "URLs".into(),
            items: Vec::new(),
            placeholder: "https://...".into(),
        },
        Field::Radio {
            id: "media_type".into(),
            label: "Type".into(),
            options: vec!["audio".into(), "video".into()],
            selected: 1,
        },
        Field::Combo {
            id: "format".into(),
            label: "Format".into(),
            options: FORMAT_OPTIONS.iter().map(|s| s.to_string()).collect(),
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
        MediaType::Audio
    } else {
        MediaType::Video
    };
    let format = FORMAT_OPTIONS
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
    let mut form = FormSpec::new(WindowSpec::new(version::settings_window_title(), theme));
    form.submit_label = "Save".into();
    form.cancel_label = "Cancel".into();
    form.fields = vec![
        Field::Section {
            label: "Download".into(),
        },
        Field::Path {
            id: "download_dir".into(),
            label: "Default download folder".into(),
            value: settings.download_dir.clone(),
            directory: true,
        },
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
        Field::Section {
            label: "Interface".into(),
        },
        combo(
            "gui_backend",
            "GUI backend",
            BACKEND_OPTIONS,
            settings.gui_backend.as_str(),
        ),
        combo(
            "gui_download_mode",
            "Download mode",
            MODE_OPTIONS,
            settings.gui_download_mode.as_str(),
        ),
        combo(
            "gui_theme",
            "Theme",
            THEME_OPTIONS,
            settings.gui_theme.as_str(),
        ),
        Field::Check {
            id: "download_logs".into(),
            label: "Write download logs".into(),
            value: settings.download_logs,
        },
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
    Settings {
        download_dir: values.text("download_dir"),
        yt_dlp: ToolSource::parse_lenient(option_at(values, "yt_dlp", TOOL_OPTIONS)).0,
        ffmpeg: ToolSource::parse_lenient(option_at(values, "ffmpeg", TOOL_OPTIONS)).0,
        gui_backend: GuiBackend::parse_lenient(option_at(values, "gui_backend", BACKEND_OPTIONS)).0,
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

fn option_at<'a>(values: &FormValues, id: &str, options: &'a [&'a str]) -> &'a str {
    options.get(values.index(id)).copied().unwrap_or(options[0])
}
