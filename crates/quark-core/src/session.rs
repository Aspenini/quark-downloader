use crate::config::{self, FilenameSpaces, GuiDownloadMode, GuiTheme, Settings, ToolSource};
use crate::json::{self, Value};
use crate::result::DownloadResult;

pub const PROTOCOL_VERSION: u32 = 1;
pub fn cli_name() -> &'static str {
    quark_platform::cli_name()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadParams {
    pub urls: Vec<String>,
    pub media_type: String,
    pub format: String,
    pub output_dir: String,
}

impl DownloadParams {
    pub fn url(&self) -> &str {
        self.urls.first().map(String::as_str).unwrap_or("")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsForm {
    pub download_dir: String,
    pub yt_dlp: String,
    pub ffmpeg: String,
    pub gui_download_mode: String,
    pub download_logs: bool,
    pub gui_theme: String,
    pub strip_video_ids: bool,
    pub sanitize_filenames: bool,
    pub filename_spaces: String,
    pub playlist_folders: bool,
    pub gui_frontend: String,
}

impl SettingsForm {
    #[allow(clippy::too_many_arguments)]
    pub fn from_strings(
        download_dir: &str,
        yt_dlp: &str,
        ffmpeg: &str,
        gui_download_mode: &str,
        download_logs: &str,
        gui_theme: &str,
        strip_video_ids: &str,
        sanitize_filenames: &str,
        filename_spaces: &str,
        playlist_folders: &str,
        gui_frontend: &str,
    ) -> Self {
        Self {
            download_dir: download_dir.to_string(),
            yt_dlp: yt_dlp.to_string(),
            ffmpeg: ffmpeg.to_string(),
            gui_download_mode: gui_download_mode.to_string(),
            download_logs: config::parse_bool(download_logs, "download_logs", true, true),
            gui_theme: config::parse_gui_theme(gui_theme, true).as_str().into(),
            strip_video_ids: config::parse_bool(strip_video_ids, "strip_video_ids", true, true),
            sanitize_filenames: config::parse_bool(
                sanitize_filenames,
                "sanitize_filenames",
                true,
                true,
            ),
            filename_spaces: config::parse_filename_spaces(filename_spaces, true)
                .as_str()
                .into(),
            playlist_folders: config::parse_bool(playlist_folders, "playlist_folders", true, true),
            gui_frontend: config::parse_gui_frontend(gui_frontend, true)
                .as_str()
                .into(),
        }
    }

    pub fn to_settings(&self) -> Settings {
        Settings {
            download_dir: self.download_dir.clone(),
            yt_dlp: if quark_platform::allows_bundled_tools() {
                config::parse_tool_source(&self.yt_dlp, "yt_dlp", true)
            } else {
                ToolSource::Path
            },
            ffmpeg: if quark_platform::allows_bundled_tools() {
                config::parse_tool_source(&self.ffmpeg, "ffmpeg", true)
            } else {
                ToolSource::Path
            },
            gui_download_mode: config::parse_gui_download_mode(&self.gui_download_mode, true),
            download_logs: self.download_logs,
            gui_theme: config::parse_gui_theme(&self.gui_theme, true),
            strip_video_ids: self.strip_video_ids,
            sanitize_filenames: self.sanitize_filenames,
            filename_spaces: config::parse_filename_spaces(&self.filename_spaces, true),
            playlist_folders: self.playlist_folders,
            gui_frontend: config::parse_gui_frontend(&self.gui_frontend, true),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MainAction {
    Download(DownloadParams),
    Cancel,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainSessionResult {
    pub action: MainAction,
    pub settings_form: Option<SettingsForm>,
}

impl MainSessionResult {
    pub fn cancel() -> Self {
        Self {
            action: MainAction::Cancel,
            settings_form: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            action: MainAction::Error(message.into()),
            settings_form: None,
        }
    }
}

pub fn build_session_args(default_dir: &str, settings: &Settings) -> Vec<String> {
    let ytdlp = if quark_platform::allows_bundled_tools() {
        settings.yt_dlp.as_str()
    } else {
        "path"
    };
    let ffmpeg = if quark_platform::allows_bundled_tools() {
        settings.ffmpeg.as_str()
    } else {
        "path"
    };
    vec![
        "--session".into(),
        default_dir.into(),
        settings.download_dir.clone(),
        ytdlp.into(),
        ffmpeg.into(),
        settings.gui_download_mode.as_str().into(),
        settings.download_logs.to_string(),
        settings.gui_theme.as_str().into(),
        settings.strip_video_ids.to_string(),
        settings.sanitize_filenames.to_string(),
        settings.filename_spaces.as_str().into(),
        settings.playlist_folders.to_string(),
        settings.gui_frontend.as_str().into(),
    ]
}

pub fn parse(text: &str) -> MainSessionResult {
    let stripped = text.trim();
    if stripped.is_empty() {
        return MainSessionResult::cancel();
    }
    if stripped.starts_with('{') {
        return parse_json(stripped);
    }
    parse_legacy(text)
}

fn parse_json(text: &str) -> MainSessionResult {
    let Ok(data) = json::parse(text) else {
        return MainSessionResult::error("invalid session JSON");
    };
    match data.get_i32("v") {
        Some(v) if v == PROTOCOL_VERSION as i32 => {}
        Some(v) => {
            return MainSessionResult::error(format!(
                "unsupported session protocol version {v} (expected {PROTOCOL_VERSION})"
            ));
        }
        None => return MainSessionResult::error("session JSON missing protocol version"),
    }
    let settings_form = parse_settings_json(data.get("settings"));
    let action = match data.get_str("action") {
        Some("download") => {
            let urls = data
                .get("urls")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            let media = data.get_str("media_type").unwrap_or("video").to_string();
            let format = data.get_str("format").unwrap_or("original").to_string();
            let output = data.get_str("output_dir").unwrap_or("").to_string();
            if urls.is_empty() || output.is_empty() {
                MainAction::Error("download action missing urls or output_dir".into())
            } else {
                MainAction::Download(DownloadParams {
                    urls,
                    media_type: media,
                    format,
                    output_dir: output,
                })
            }
        }
        Some("error") => MainAction::Error(
            data.get_str("message")
                .unwrap_or("frontend error")
                .to_string(),
        ),
        _ => MainAction::Cancel,
    };
    MainSessionResult {
        action,
        settings_form,
    }
}

fn parse_settings_json(node: Option<&Value>) -> Option<SettingsForm> {
    let obj = node?;
    Some(SettingsForm::from_strings(
        obj.get_str("download_dir").unwrap_or("~/Downloads"),
        obj.get_str("yt_dlp").unwrap_or("path"),
        obj.get_str("ffmpeg").unwrap_or("path"),
        obj.get_str("gui_download_mode").unwrap_or("progress"),
        &obj.get("download_logs")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        obj.get_str("gui_theme")
            .unwrap_or(GuiTheme::System.as_str()),
        &obj.get("strip_video_ids")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        &obj.get("sanitize_filenames")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        obj.get_str("filename_spaces")
            .unwrap_or(FilenameSpaces::Keep.as_str()),
        &obj.get("playlist_folders")
            .map(Value::raw_display)
            .unwrap_or_else(|| "true".into()),
        obj.get_str("gui_frontend").unwrap_or("auto"),
    ))
}

pub fn emit_json(
    action: &str,
    settings: Option<&SettingsForm>,
    urls: &[String],
    media_type: &str,
    format: &str,
    output_dir: &str,
) -> String {
    let mut out = format!(
        "{{\"v\":{},\"action\":{}",
        PROTOCOL_VERSION,
        json::stringify_str(action)
    );
    if let Some(settings) = settings {
        out.push_str(&format!(
            ",\"settings\":{{\"download_dir\":{},\"yt_dlp\":{},\"ffmpeg\":{},\"gui_download_mode\":{},\"download_logs\":{},\"gui_theme\":{},\"strip_video_ids\":{},\"sanitize_filenames\":{},\"filename_spaces\":{},\"playlist_folders\":{},\"gui_frontend\":{}}}",
            json::stringify_str(&settings.download_dir),
            json::stringify_str(&settings.yt_dlp),
            json::stringify_str(&settings.ffmpeg),
            json::stringify_str(&settings.gui_download_mode),
            settings.download_logs,
            json::stringify_str(&settings.gui_theme),
            settings.strip_video_ids,
            settings.sanitize_filenames,
            json::stringify_str(&settings.filename_spaces),
            settings.playlist_folders,
            json::stringify_str(&settings.gui_frontend),
        ));
    }
    if action == "download" {
        let urls_json = urls
            .iter()
            .map(|u| json::stringify_str(u))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&format!(
            ",\"urls\":[{urls_json}],\"media_type\":{},\"format\":{},\"output_dir\":{}",
            json::stringify_str(media_type),
            json::stringify_str(format),
            json::stringify_str(output_dir)
        ));
    }
    out.push('}');
    out
}

fn parse_legacy(text: &str) -> MainSessionResult {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    if lines.is_empty() || lines[0] != "__SESSION__" {
        return MainSessionResult::cancel();
    }
    let mut action = MainAction::Cancel;
    let mut settings_form = None;
    let mut i = 1;
    while i < lines.len() {
        match lines[i] {
            "__SETTINGS__" => {
                let (block, next) = read_block(&lines, i + 1);
                if let Some(form) = parse_settings(&block) {
                    settings_form = Some(form);
                }
                i = next;
            }
            "__DOWNLOAD__" => {
                let (block, next) = read_block(&lines, i + 1);
                if block.len() >= 4 && !block[0].is_empty() && !block[3].is_empty() {
                    action = MainAction::Download(DownloadParams {
                        urls: vec![block[0].to_string()],
                        media_type: block[1].to_string(),
                        format: block[2].to_string(),
                        output_dir: block[3].to_string(),
                    });
                }
                i = next;
            }
            "__DOWNLOAD_MULTI__" => {
                let (block, next) = read_block(&lines, i + 1);
                if let Some(download) = parse_download_multi(&block) {
                    action = download;
                }
                i = next;
            }
            "__CANCEL__" => {
                action = MainAction::Cancel;
                i += 1;
            }
            _ => i += 1,
        }
    }
    MainSessionResult {
        action,
        settings_form,
    }
}

fn read_block<'a>(lines: &'a [&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut stop = start;
    while stop < lines.len() && !lines[stop].starts_with("__") {
        stop += 1;
    }
    (lines[start..stop].to_vec(), stop)
}

fn parse_settings(block: &[&str]) -> Option<SettingsForm> {
    if block.len() < 5 {
        return None;
    }
    Some(SettingsForm::from_strings(
        block[0],
        block[1],
        block[2],
        block[3],
        block[4],
        block.get(5).copied().unwrap_or(GuiTheme::System.as_str()),
        block.get(6).copied().unwrap_or("true"),
        block.get(7).copied().unwrap_or("true"),
        block
            .get(8)
            .copied()
            .unwrap_or(FilenameSpaces::Keep.as_str()),
        block.get(9).copied().unwrap_or("true"),
        block.get(10).copied().unwrap_or("auto"),
    ))
}

fn parse_download_multi(block: &[&str]) -> Option<MainAction> {
    let count: usize = block.first()?.parse().ok()?;
    if count == 0 || block.len() != count + 4 {
        return None;
    }
    let urls: Vec<String> = block[1..=count]
        .iter()
        .filter(|u| !u.is_empty())
        .map(|s| (*s).to_string())
        .collect();
    if urls.is_empty() {
        return None;
    }
    let media_type = block[count + 1].to_string();
    let format = block[count + 2].to_string();
    let output_dir = block[count + 3].to_string();
    if output_dir.is_empty() {
        return None;
    }
    Some(MainAction::Download(DownloadParams {
        urls,
        media_type,
        format,
        output_dir,
    }))
}

pub fn build_cli_args(cli: &str, params: &DownloadParams) -> Vec<String> {
    let mut args = vec![cli.to_string()];
    for url in &params.urls {
        args.push("--url".into());
        args.push(url.clone());
    }
    args.extend([
        "--type".into(),
        params.media_type.clone(),
        "--format".into(),
        params.format.clone(),
        "--output-dir".into(),
        params.output_dir.clone(),
        "--no-pause".into(),
        "--emit-result-json".into(),
    ]);
    args
}

pub fn resolve_cli() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(override_path) = std::env::var_os("QUARK_DOWNLOADER_CLI") {
        let p = PathBuf::from(override_path);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(gui_exe) = std::env::current_exe()
        && let Some(parent) = gui_exe.parent()
    {
        let sibling = parent.join(cli_name());
        if sibling.exists() {
            return Some(sibling);
        }
    }
    let dev = PathBuf::from("build").join(cli_name());
    if dev.exists() {
        return Some(dev);
    }
    let target_rel = PathBuf::from("target").join("release").join(cli_name());
    if target_rel.exists() {
        return Some(target_rel);
    }
    crate::process::which(cli_name())
}

pub fn default_output_dir() -> std::path::PathBuf {
    crate::download::default_output_dir()
}

pub fn parse_emit_line(line: &str) -> Option<DownloadResult> {
    DownloadResult::parse_emit_line(line)
}

#[allow(dead_code)]
fn _keep_mode(mode: GuiDownloadMode) -> &'static str {
    mode.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_session_args_in_protocol_order() {
        let settings = Settings {
            download_dir: "~/Videos".into(),
            yt_dlp: ToolSource::Bundled,
            ffmpeg: ToolSource::Path,
            gui_download_mode: GuiDownloadMode::ExternalCli,
            download_logs: false,
            gui_theme: GuiTheme::Dark,
            strip_video_ids: false,
            sanitize_filenames: true,
            filename_spaces: FilenameSpaces::Underscore,
            playlist_folders: false,
            gui_frontend: crate::config::GuiFrontend::Qt,
        };
        let expected_ytdlp = if quark_platform::allows_bundled_tools() {
            "bundled"
        } else {
            "path"
        };
        assert_eq!(
            build_session_args("/tmp/dl", &settings),
            [
                "--session",
                "/tmp/dl",
                "~/Videos",
                expected_ytdlp,
                "path",
                "external_cli",
                "false",
                "dark",
                "false",
                "true",
                "underscore",
                "false",
                "qt",
            ]
        );
    }

    #[test]
    fn parses_multi_url_download_with_settings() {
        let result = parse(
            "__SESSION__\n__SETTINGS__\n~/Videos\nbundled\npath\nexternal_cli\nfalse\ndark\nfalse\nfalse\ndash\nfalse\n__DOWNLOAD_MULTI__\n3\nhttps://example.com/a\nhttps://example.com/b\nhttps://example.com/c\nvideo\nmp4\n/tmp/downloads\n",
        );
        let form = result.settings_form.unwrap();
        assert_eq!(form.gui_theme, "dark");
        assert!(!form.strip_video_ids);
        assert!(!form.sanitize_filenames);
        assert_eq!(form.filename_spaces, "dash");
        assert!(!form.playlist_folders);
        match result.action {
            MainAction::Download(p) => {
                assert_eq!(
                    p.urls,
                    [
                        "https://example.com/a",
                        "https://example.com/b",
                        "https://example.com/c"
                    ]
                );
                assert_eq!(p.media_type, "video");
                assert_eq!(p.format, "mp4");
                assert_eq!(p.output_dir, "/tmp/downloads");
            }
            MainAction::Cancel | MainAction::Error(_) => panic!("expected download"),
        }
    }

    #[test]
    fn parses_legacy_single_url() {
        let result = parse(
            "__SESSION__\n__DOWNLOAD__\nhttps://example.com/video\naudio\nmp3\n/tmp/downloads\n",
        );
        match result.action {
            MainAction::Download(p) => {
                assert_eq!(p.urls, ["https://example.com/video"]);
                assert_eq!(p.url(), "https://example.com/video");
            }
            MainAction::Cancel | MainAction::Error(_) => panic!("expected download"),
        }
    }

    #[test]
    fn defaults_missing_settings_lines() {
        let result =
            parse("__SESSION__\n__SETTINGS__\n~/Legacy\nauto\nauto\nprogress\ntrue\n__CANCEL__\n");
        assert!(matches!(result.action, MainAction::Cancel));
        let form = result.settings_form.unwrap();
        assert_eq!(form.gui_theme, "system");
        assert!(form.strip_video_ids);
        assert!(form.sanitize_filenames);
        assert_eq!(form.filename_spaces, "keep");
        assert!(form.playlist_folders);
    }

    #[test]
    fn rejects_malformed_multi_blocks() {
        let result = parse(
            "__SESSION__\n__DOWNLOAD_MULTI__\n2\nhttps://example.com/a\nvideo\nmp4\n/tmp/downloads\n",
        );
        assert!(matches!(result.action, MainAction::Cancel));
    }

    #[test]
    fn cancels_on_empty_or_unknown() {
        assert!(matches!(parse("").action, MainAction::Cancel));
        assert!(matches!(parse("garbage").action, MainAction::Cancel));
    }

    #[test]
    fn parses_json_v1_download() {
        let result = parse(
            r#"{
        "v": 1,
        "action": "download",
        "settings": {
          "download_dir": "~/Videos",
          "yt_dlp": "path",
          "ffmpeg": "path",
          "gui_download_mode": "progress",
          "download_logs": false,
          "gui_theme": "dark",
          "strip_video_ids": true,
          "sanitize_filenames": true,
          "filename_spaces": "keep",
          "playlist_folders": true
        },
        "urls": ["https://example.com/a", "https://example.com/b"],
        "media_type": "video",
        "format": "mp4",
        "output_dir": "/tmp/downloads"
      }"#,
        );
        let form = result.settings_form.unwrap();
        assert_eq!(form.gui_theme, "dark");
        assert!(!form.download_logs);
        match result.action {
            MainAction::Download(p) => {
                assert_eq!(p.urls, ["https://example.com/a", "https://example.com/b"]);
                assert_eq!(p.format, "mp4");
                assert_eq!(p.output_dir, "/tmp/downloads");
            }
            MainAction::Cancel | MainAction::Error(_) => panic!("expected download"),
        }
    }

    #[test]
    fn parses_json_v1_cancel() {
        let result = parse(r#"{"v":1,"action":"cancel"}"#);
        assert!(matches!(result.action, MainAction::Cancel));
    }

    #[test]
    fn rejects_unknown_protocol_version() {
        let result = parse(r#"{"v":2,"action":"cancel"}"#);
        assert!(matches!(result.action, MainAction::Error(_)));
    }

    #[test]
    fn settings_form_from_strings() {
        let form = SettingsForm::from_strings(
            "~/Videos",
            "bundled",
            "path",
            "external_cli",
            "off",
            "dark",
            "true",
            "true",
            "keep",
            "true",
            "auto",
        );
        let settings = form.to_settings();
        assert_eq!(settings.download_dir, "~/Videos");
        if quark_platform::allows_bundled_tools() {
            assert_eq!(settings.yt_dlp, ToolSource::Bundled);
            assert_eq!(settings.ffmpeg, ToolSource::Path);
        } else {
            assert_eq!(settings.yt_dlp, ToolSource::Path);
            assert_eq!(settings.ffmpeg, ToolSource::Path);
        }
        assert_eq!(settings.gui_download_mode, GuiDownloadMode::ExternalCli);
        assert!(!settings.download_logs);
        assert_eq!(settings.gui_theme, GuiTheme::Dark);
    }

    #[test]
    fn parses_named_frontends() {
        use crate::config::{GuiFrontend, parse_gui_frontend};
        assert_eq!(parse_gui_frontend("qt", true), GuiFrontend::Qt);
        assert_eq!(parse_gui_frontend("win32", true), GuiFrontend::Win32);
        assert_eq!(parse_gui_frontend("appkit", true), GuiFrontend::Appkit);
        assert_eq!(parse_gui_frontend("gtk", true), GuiFrontend::Auto);
    }
}
