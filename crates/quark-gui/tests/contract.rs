use quark_core::session::{MainAction, SettingsForm};
use quark_gui::script::run;

fn default_settings() -> SettingsForm {
    SettingsForm::from_strings(
        "~/Videos", "path", "path", "progress", "false", "dark", "true", "true", "keep", "true",
        "false",
    )
}

fn script(body: &str) -> String {
    format!(
        r#"{{"args":{{"default_dir":"/tmp/dl","settings":{{"download_dir":"~/Videos","yt_dlp":"path","ffmpeg":"path","gui_download_mode":"progress","download_logs":false,"open_output_dir":false,"gui_theme":"dark","strip_video_ids":true,"sanitize_filenames":true,"filename_spaces":"keep","playlist_folders":true}}}},"events":{body}}}"#
    )
}

fn download_urls(input: &str) -> Vec<String> {
    match run(input).unwrap().result.action {
        MainAction::Download(p) => p.urls,
        other => panic!("expected download, got {other:?}"),
    }
}

#[test]
fn adds_two_urls_and_downloads() {
    let out = run(&script(
        r#"[{"add_url":"https://example.com/a"},{"add_url":"https://example.com/b"},{"download":true}]"#,
    ))
    .unwrap();
    match &out.result.action {
        MainAction::Download(p) => {
            assert_eq!(p.urls, ["https://example.com/a", "https://example.com/b"]);
            assert_eq!(p.media_type, "video");
            assert_eq!(p.format, "original");
            assert_eq!(p.output_dir, "/tmp/dl");
        }
        other => panic!("{other:?}"),
    }
    assert!(out.result.settings_form.is_none());
    assert_eq!(out.exit_code(), 0);
}

#[test]
fn ignores_duplicate_urls() {
    let urls = download_urls(&script(
        r#"[{"add_url":"https://example.com/a"},{"add_url":"https://example.com/a"},{"download":true}]"#,
    ));
    assert_eq!(urls, ["https://example.com/a"]);
}

#[test]
fn empty_download_is_error() {
    let out = run(&script(r#"[{"download":true}]"#)).unwrap();
    assert_eq!(
        out.validation_error.as_deref(),
        Some(quark_gui::ERR_EMPTY_QUEUE)
    );
    assert_eq!(out.exit_code(), 2);
}

#[test]
fn empty_output_is_error() {
    let out = run(&script(
        r#"[{"add_url":"https://example.com/a"},{"set_output":"  "},{"download":true}]"#,
    ))
    .unwrap();
    assert_eq!(
        out.validation_error.as_deref(),
        Some(quark_gui::ERR_EMPTY_OUTPUT)
    );
}

#[test]
fn flushes_url_field_on_download() {
    let urls = download_urls(&script(
        r#"[{"set_url_field":"https://example.com/z"},{"download":true}]"#,
    ));
    assert_eq!(urls, ["https://example.com/z"]);
}

#[test]
fn audio_switch_resets_format_then_accepts_mp3() {
    let out = run(&script(
        r#"[{"add_url":"https://example.com/a"},{"set_format":"mp4"},{"set_media":"audio"},{"set_format":"mp3"},{"download":true}]"#,
    ))
    .unwrap();
    match out.result.action {
        MainAction::Download(p) => {
            assert_eq!(p.media_type, "audio");
            assert_eq!(p.format, "mp3");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn save_settings_then_download_includes_settings() {
    let out = run(&script(
        r#"[{"open_settings":true},{"set_setting":{"download_dir":"~/Media","yt_dlp":"path","ffmpeg":"path","gui_download_mode":"progress","download_logs":false,"open_output_dir":true,"gui_theme":"dark","strip_video_ids":false,"sanitize_filenames":true,"filename_spaces":"dash","playlist_folders":true}},{"save_settings":true},{"add_url":"https://example.com/a"},{"download":true}]"#,
    ))
    .unwrap();
    let form = out
        .result
        .settings_form
        .expect("settings should be present");
    assert_eq!(form.download_dir, "~/Media");
    assert_eq!(form.filename_spaces, "dash");
    assert!(!form.strip_video_ids);
    assert!(form.open_output_dir);
}

#[test]
fn discarded_settings_are_not_emitted() {
    let out = run(&script(
        r#"[{"open_settings":true},{"set_setting":{"download_dir":"~/Nope","yt_dlp":"path","ffmpeg":"path","gui_download_mode":"progress","download_logs":true,"open_output_dir":true,"gui_theme":"light","strip_video_ids":true,"sanitize_filenames":true,"filename_spaces":"keep","playlist_folders":true}},{"close_settings":true},{"add_url":"https://example.com/a"},{"download":true}]"#,
    ))
    .unwrap();
    assert!(out.result.settings_form.is_none());
}

#[test]
fn cancel_without_save_has_no_settings() {
    let out = run(&script(r#"[{"cancel":true}]"#)).unwrap();
    assert!(matches!(out.result.action, MainAction::Cancel));
    assert!(out.result.settings_form.is_none());
}

#[test]
fn paste_adds_two_urls() {
    let urls = download_urls(&script(
        r#"[{"paste":"https://example.com/a\nhttps://example.com/b"},{"download":true}]"#,
    ));
    assert_eq!(urls, ["https://example.com/a", "https://example.com/b"]);
}

#[test]
fn unknown_session_version_is_error() {
    let parsed = quark_core::session::parse(
        r#"{"v":99,"action":"download","urls":["https://a"],"media_type":"video","format":"original","output_dir":"/tmp"}"#,
    );
    assert!(matches!(parsed.action, MainAction::Error(_)));
}

#[test]
fn required_actions_are_exhaustive_to_bind() {
    quark_gui::assert_frontend_binds(|event| {
        let _ = event;
    });
    let _ = default_settings();
}
