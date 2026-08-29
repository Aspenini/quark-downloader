//! JNI surface for the Android app. Host tests cover the same helpers
//! without a JVM.

use std::path::{Path, PathBuf};

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;

use quark_core::config::Settings;
use quark_core::filename::{self, SpacesPolicy};
use quark_core::json;
use quark_core::media::{Format, MediaType};
use quark_core::playlist;
use quark_core::progress;
use quark_core::session::SettingsForm;
use quark_core::ytdlp;

mod session;

pub fn set_paths(config_dir: Option<&str>) {
    quark_platform::set_config_dir_override(config_dir.map(PathBuf::from));
}

pub fn set_js_runtime(spec: Option<&str>) {
    ytdlp::set_injected_js_runtime(spec.map(str::to_string));
}

pub fn gui_script(input: &str) -> Result<String, String> {
    quark_gui::run_script(input).map(|out| out.to_json())
}

pub fn build_ytdlp_args(
    url: &str,
    media: &str,
    format: &str,
    output_dir: &str,
    settings: &Settings,
    ffmpeg_location: Option<&Path>,
    js_runtime: Option<&str>,
) -> Result<Vec<String>, String> {
    let media_type = MediaType::parse(media)?;
    let format = Format::parse_for(media_type, format)?;
    let target = Path::new(output_dir);
    let plan = ytdlp::plan(&ytdlp::PlanRequest {
        ytdlp: Path::new("yt-dlp"),
        url,
        media_type,
        format,
        target_dir: target,
        settings,
        ffmpeg_location,
        js_runtime,
        is_playlist: playlist::playlist_url(url),
        no_color: true,
    })
    .map_err(|e| e.0)?;
    Ok(plan.args)
}

fn opts_json(args: &[String]) -> String {
    const TAKES_VALUE: &[&str] = &[
        "-o",
        "-f",
        "--socket-timeout",
        "--retries",
        "--fragment-retries",
        "--ffmpeg-location",
        "--audio-format",
        "--merge-output-format",
        "--recode-video",
        "--remux-video",
        "--remote-components",
        "--js-runtimes",
    ];
    let mut i = 0;
    let mut parts = Vec::new();
    while i < args.len() {
        let name = &args[i];
        if TAKES_VALUE.contains(&name.as_str()) && i + 1 < args.len() {
            parts.push(format!(
                "{{\"n\":{},\"v\":{}}}",
                json::stringify_str(name),
                json::stringify_str(&args[i + 1])
            ));
            i += 2;
        } else {
            parts.push(format!("{{\"n\":{}}}", json::stringify_str(name)));
            i += 1;
        }
    }
    format!("[{}]", parts.join(","))
}

pub fn settings_from_json(text: &str) -> Result<Settings, String> {
    if text.trim().is_empty() {
        return Ok(Settings::default());
    }
    let value = json::parse(text).map_err(|e| e.to_string())?;
    quark_gui::script::settings_from_value(&value)
        .map(|form: SettingsForm| form.to_settings())
        .ok_or_else(|| "invalid settings JSON".into())
}

pub fn parse_progress_json(line: &str) -> String {
    let percent = progress::parse_progress_percent(line);
    let eta = progress::parse_eta(line);
    let status = progress::parse_status_line(line);
    if percent.is_none() && eta.is_none() && status.is_none() {
        return "null".into();
    }
    let mut parts = Vec::new();
    if let Some(p) = percent {
        parts.push(format!("\"percent\":{p}"));
    }
    if let Some(e) = eta {
        parts.push(format!("\"eta\":{}", json::stringify_str(&e)));
    }
    if let Some(s) = status {
        parts.push(format!("\"status\":{}", json::stringify_str(&s)));
    }
    format!("{{{}}}", parts.join(","))
}

pub fn sanitize_file_name(name: &str, ascii_only: bool, spaces: &str) -> Result<String, String> {
    let policy = match spaces {
        "keep" => SpacesPolicy::Keep,
        "underscore" => SpacesPolicy::Underscore,
        "dash" => SpacesPolicy::Dash,
        "remove" => SpacesPolicy::Remove,
        other => return Err(format!("unknown spaces policy: {other}")),
    };
    Ok(filename::sanitize_filename(name, ascii_only, policy))
}

fn jni_string(env: &mut JNIEnv, value: JString) -> Result<String, String> {
    env.get_string(&value)
        .map(String::from)
        .map_err(|e| e.to_string())
}

fn jni_optional(env: &mut JNIEnv, value: JString) -> Result<Option<String>, String> {
    let s = jni_string(env, value)?;
    if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
}

fn jni_return(env: &mut JNIEnv, s: &str) -> jstring {
    env.new_string(s)
        .map(|v| v.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

fn jni_error(env: &mut JNIEnv, msg: &str) -> jstring {
    let _ = env.throw_new("java/lang/IllegalArgumentException", msg);
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_catalog(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    jni_return(&mut env, &session::catalog_json())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_sessionStart(
    mut env: JNIEnv,
    _class: JClass,
    default_dir: JString,
    settings_json: JString,
) -> jstring {
    match (|| {
        let dir = jni_string(&mut env, default_dir)?;
        let settings = jni_string(&mut env, settings_json)?;
        session::start(&dir, &settings)
    })() {
        Ok(json) => jni_return(&mut env, &json),
        Err(e) => jni_error(&mut env, &e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_sessionDispatch(
    mut env: JNIEnv,
    _class: JClass,
    event_json: JString,
) -> jstring {
    match jni_string(&mut env, event_json).and_then(|s| session::dispatch(&s)) {
        Ok(json) => jni_return(&mut env, &json),
        Err(e) => jni_error(&mut env, &e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_isPlaylistUrl(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jni::sys::jboolean {
    match jni_string(&mut env, url) {
        Ok(url) => playlist::playlist_url(&url) as u8,
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_sanitizeComponent(
    mut env: JNIEnv,
    _class: JClass,
    name: JString,
    ascii_only: jni::sys::jboolean,
    spaces: JString,
) -> jstring {
    let result = (|| {
        let name = jni_string(&mut env, name)?;
        let spaces = jni_string(&mut env, spaces)?;
        let policy = match spaces.as_str() {
            "keep" => SpacesPolicy::Keep,
            "underscore" => SpacesPolicy::Underscore,
            "dash" => SpacesPolicy::Dash,
            "remove" => SpacesPolicy::Remove,
            other => return Err(format!("unknown spaces policy: {other}")),
        };
        Ok(filename::sanitize_component(&name, ascii_only != 0, policy))
    })();
    match result {
        Ok(s) => jni_return(&mut env, &s),
        Err(e) => jni_error(&mut env, &e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_setPaths(
    mut env: JNIEnv,
    _class: JClass,
    config_dir: JString,
) {
    match jni_optional(&mut env, config_dir) {
        Ok(dir) => set_paths(dir.as_deref()),
        Err(e) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException", &e);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_setJsRuntime(
    mut env: JNIEnv,
    _class: JClass,
    spec: JString,
) {
    match jni_optional(&mut env, spec) {
        Ok(s) => set_js_runtime(s.as_deref()),
        Err(e) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException", &e);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_guiScript(
    mut env: JNIEnv,
    _class: JClass,
    input: JString,
) -> jstring {
    match jni_string(&mut env, input).and_then(|s| gui_script(&s)) {
        Ok(json) => jni_return(&mut env, &json),
        Err(e) => jni_error(&mut env, &e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_buildYtDlpArgs(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    media: JString,
    format: JString,
    output_dir: JString,
    settings_json: JString,
    ffmpeg_location: JString,
    js_runtime: JString,
) -> jstring {
    let result = (|| {
        let url = jni_string(&mut env, url)?;
        let media = jni_string(&mut env, media)?;
        let format = jni_string(&mut env, format)?;
        let output_dir = jni_string(&mut env, output_dir)?;
        let settings = settings_from_json(&jni_string(&mut env, settings_json)?)?;
        let mut ffmpeg = jni_optional(&mut env, ffmpeg_location)?;
        let runtime = jni_optional(&mut env, js_runtime)?;
        let parsed = Format::parse_for(MediaType::parse(&media)?, &format)?;
        if parsed.needs_ffmpeg() && ffmpeg.is_none() {
            ffmpeg = Some("ffmpeg".into());
        }
        let args = build_ytdlp_args(
            &url,
            &media,
            &format,
            &output_dir,
            &settings,
            ffmpeg.as_deref().map(Path::new),
            runtime.as_deref(),
        )?;
        Ok::<String, String>(opts_json(&args))
    })();
    match result {
        Ok(json) => jni_return(&mut env, &json),
        Err(e) => jni_error(&mut env, &e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_parseProgress(
    mut env: JNIEnv,
    _class: JClass,
    line: JString,
) -> jstring {
    match jni_string(&mut env, line) {
        Ok(line) => jni_return(&mut env, &parse_progress_json(&line)),
        Err(e) => jni_error(&mut env, &e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_aspenini_quark_QuarkNative_sanitizeFilename(
    mut env: JNIEnv,
    _class: JClass,
    name: JString,
    ascii_only: jni::sys::jboolean,
    spaces: JString,
) -> jstring {
    let result = (|| {
        let name = jni_string(&mut env, name)?;
        let spaces = jni_string(&mut env, spaces)?;
        sanitize_file_name(&name, ascii_only != 0, &spaces)
    })();
    match result {
        Ok(s) => jni_return(&mut env, &s),
        Err(e) => jni_error(&mut env, &e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_script_downloads_a_url() {
        let json = gui_script(
            r#"{"args":{"default_dir":"/tmp/dl"},"events":[{"add_url":"https://example.com/a"},{"download":true}]}"#,
        )
        .unwrap();
        assert!(json.contains("\"action\":\"download\""), "{json}");
        assert!(json.contains("https://example.com/a"), "{json}");
    }

    #[test]
    fn build_args_audio_mp3() {
        let settings = Settings::default();
        let args = build_ytdlp_args(
            "https://example.com/a",
            "audio",
            "mp3",
            "/out",
            &settings,
            Some(Path::new("/opt/ffmpeg")),
            None,
        )
        .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("-x"));
        assert!(joined.contains("--audio-format"));
        assert!(joined.contains("mp3"));
        assert!(joined.contains("--no-color"));
    }

    #[test]
    fn parse_progress_extracts_percent() {
        let json = parse_progress_json("[download]  12.5% of 1.00MiB at 1.00MiB/s ETA 00:03");
        assert!(json.contains("\"percent\":12.5"), "{json}");
        assert!(json.contains("\"eta\""), "{json}");
    }

    #[test]
    fn sanitize_spaces_underscore() {
        assert_eq!(
            sanitize_file_name("A B.mp4", true, "underscore").unwrap(),
            "A_B.mp4"
        );
    }
}
