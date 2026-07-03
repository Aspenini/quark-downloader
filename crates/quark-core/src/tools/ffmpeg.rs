//! Locate ffmpeg (and, on Windows, optionally download a bundled build).

use std::path::PathBuf;

use super::ToolError;
use crate::config::ToolSource;
use crate::events::{EventSink, ProgressEvent};
use crate::paths;

fn log(sink: &dyn EventSink, msg: impl Into<String>) {
    sink.emit(ProgressEvent::Log(msg.into()));
}

fn debug(sink: &dyn EventSink, msg: impl Into<String>) {
    sink.emit(ProgressEvent::Debug(msg.into()));
}

pub fn executable_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

#[cfg(windows)]
fn ffprobe_name() -> &'static str {
    "ffprobe.exe"
}

pub fn bundled_path() -> PathBuf {
    paths::tools_dir().join(executable_name())
}

fn path_executable() -> Option<PathBuf> {
    crate::util::find_executable("ffmpeg")
}

#[cfg(windows)]
fn bundled_exists() -> bool {
    bundled_path().exists()
}

/// Log where ffmpeg will come from, or warn if it is missing. Called once
/// before downloads start.
pub fn detect(source: ToolSource, sink: &dyn EventSink) {
    if let Some((from_path, path)) = locate(source) {
        if from_path {
            debug(sink, format!("Using ffmpeg from PATH: {}", path.display()));
        } else {
            debug(sink, format!("Using ffmpeg from: {}", path.display()));
        }
    } else {
        warn_not_found(sink);
    }
}

/// `(from_path, path)` of an ffmpeg binary to use, if one is available.
fn locate(source: ToolSource) -> Option<(bool, PathBuf)> {
    #[cfg(not(windows))]
    let source = {
        let _ = source;
        ToolSource::Auto
    };

    if matches!(source, ToolSource::Path | ToolSource::Auto) {
        if let Some(exe) = path_executable() {
            return Some((true, exe));
        }
    }
    #[cfg(windows)]
    {
        if matches!(source, ToolSource::Bundled | ToolSource::Auto) && bundled_exists() {
            return Some((false, bundled_path()));
        }
    }
    None
}

fn warn_not_found(sink: &dyn EventSink) {
    log(sink, "");
    log(sink, "Warning: ffmpeg not found on PATH.");
    #[cfg(target_os = "macos")]
    log(sink, "  Install with Homebrew: brew install ffmpeg");
    #[cfg(target_os = "linux")]
    log(
        sink,
        "  Install with your package manager, e.g. apt install ffmpeg",
    );
    #[cfg(windows)]
    log(
        sink,
        "  Install ffmpeg, add it to PATH, or allow a network download when converting formats.",
    );
    log(
        sink,
        "  Original-format downloads may still work; conversion requires ffmpeg.",
    );
}

/// Directory to pass to yt-dlp's `--ffmpeg-location`. PATH first; on Windows,
/// tools/ then download. `quiet` suppresses the "Using ffmpeg" log line when
/// `detect` already reported it.
pub fn ensure(source: ToolSource, sink: &dyn EventSink, quiet: bool) -> Result<PathBuf, ToolError> {
    #[cfg(windows)]
    {
        match source {
            ToolSource::Path => ensure_path_only(sink, quiet),
            ToolSource::Bundled => ensure_bundled(sink, quiet),
            ToolSource::Auto => {
                if let Some(exe) = path_executable() {
                    if !quiet {
                        debug(sink, format!("Using ffmpeg from PATH: {}", exe.display()));
                    }
                    return Ok(exe.parent().map(|p| p.to_path_buf()).unwrap_or_default());
                }
                ensure_bundled(sink, quiet)
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = source;
        if let Some(exe) = path_executable() {
            if !quiet {
                debug(sink, format!("Using ffmpeg from PATH: {}", exe.display()));
            }
            return Ok(exe.parent().map(|p| p.to_path_buf()).unwrap_or_default());
        }
        Err(raise_not_found())
    }
}

#[cfg(not(windows))]
fn raise_not_found() -> ToolError {
    #[cfg(target_os = "macos")]
    let msg = "ffmpeg not found on PATH.\nInstall with Homebrew: brew install ffmpeg";
    #[cfg(target_os = "linux")]
    let msg =
        "ffmpeg not found on PATH.\nInstall with your package manager, e.g. apt install ffmpeg";
    #[cfg(all(not(target_os = "macos"), not(target_os = "linux")))]
    let msg = "ffmpeg not found on PATH.";
    ToolError(msg.to_string())
}

// ---- Windows bundled download -------------------------------------------

#[cfg(windows)]
mod win {
    use super::*;
    use crate::net::http;

    const GITHUB_LATEST_URL: &str =
        "https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest";
    const VERSION_FILE: &str = ".ffmpeg-version";

    pub fn ensure_path_only(sink: &dyn EventSink, quiet: bool) -> Result<PathBuf, ToolError> {
        if let Some(exe) = path_executable() {
            if !quiet {
                debug(sink, format!("Using ffmpeg from PATH: {}", exe.display()));
            }
            return Ok(exe.parent().map(|p| p.to_path_buf()).unwrap_or_default());
        }
        Err(ToolError(
            "ffmpeg not found on PATH (quark-downloader.toml: ffmpeg = path).\n\
             Install ffmpeg and add it to PATH, or set ffmpeg = auto or bundled."
                .to_string(),
        ))
    }

    pub fn ensure_bundled(sink: &dyn EventSink, quiet: bool) -> Result<PathBuf, ToolError> {
        let tools = paths::tools_dir();
        std::fs::create_dir_all(&tools).ok();
        if bundled_exists() {
            if !quiet {
                log(
                    sink,
                    format!("Using ffmpeg from: {}", bundled_path().display()),
                );
            }
            return Ok(tools);
        }
        if std::env::var("QUARK_SKIP_FFMPEG_DOWNLOAD").as_deref() == Ok("1") {
            return Err(ToolError(
                "ffmpeg not found in tools/ (quark-downloader.toml: ffmpeg = bundled).\n\
                 Place ffmpeg.exe in tools/ or allow a network download when converting formats."
                    .to_string(),
            ));
        }
        log(sink, "Downloading ffmpeg...");
        download_latest(sink)?;
        Ok(tools)
    }

    fn asset_target() -> &'static str {
        if cfg!(target_arch = "aarch64") {
            "ffmpeg-master-latest-winarm64-gpl.zip"
        } else {
            "ffmpeg-master-latest-win64-gpl.zip"
        }
    }

    fn download_latest(sink: &dyn EventSink) -> Result<(), ToolError> {
        let body = http::fetch_body(GITHUB_LATEST_URL)?;
        let release: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ToolError(format!("parsing release JSON: {e}")))?;
        let target = asset_target();
        let asset = release
            .get("assets")
            .and_then(|v| v.as_array())
            .and_then(|a| {
                a.iter()
                    .find(|x| x.get("name").and_then(|n| n.as_str()) == Some(target))
            })
            .ok_or_else(|| ToolError(format!("FFmpeg release has no asset named {target}")))?;
        let url = asset
            .get("browser_download_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("asset missing URL".into()))?;
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("latest")
            .to_string();

        let tools = paths::tools_dir();
        let archive = tools.join(target);
        log(sink, format!("Fetching {target}..."));
        http::download_file(url, &archive)?;
        extract_and_install(&archive, &tag, sink)?;
        let _ = std::fs::remove_file(&archive);
        Ok(())
    }

    fn extract_and_install(
        archive: &std::path::Path,
        tag: &str,
        sink: &dyn EventSink,
    ) -> Result<(), ToolError> {
        let file =
            std::fs::File::open(archive).map_err(|e| ToolError(format!("opening archive: {e}")))?;
        let mut zip =
            zip::ZipArchive::new(file).map_err(|e| ToolError(format!("reading archive: {e}")))?;
        let tools = paths::tools_dir();

        let mut found_ffmpeg = false;
        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|e| ToolError(format!("archive entry: {e}")))?;
            let Some(name) = entry
                .enclosed_name()
                .and_then(|p| p.file_name().map(|f| f.to_owned()))
            else {
                continue;
            };
            let name = name.to_string_lossy().to_string();
            let dest = if name == executable_name() {
                found_ffmpeg = true;
                Some(bundled_path())
            } else if name == ffprobe_name() {
                Some(tools.join(ffprobe_name()))
            } else {
                None
            };
            if let Some(dest) = dest {
                // Extract to a temp name first so an interrupted extraction
                // never leaves a truncated binary at the final path.
                let tmp = dest.with_extension("download");
                let mut out = std::fs::File::create(&tmp)
                    .map_err(|e| ToolError(format!("writing {}: {e}", tmp.display())))?;
                std::io::copy(&mut entry, &mut out)
                    .map_err(|e| ToolError(format!("extracting {name}: {e}")))?;
                drop(out);
                if dest.exists() {
                    let _ = std::fs::remove_file(&dest);
                }
                std::fs::rename(&tmp, &dest)
                    .map_err(|e| ToolError(format!("installing {}: {e}", dest.display())))?;
            }
        }
        if !found_ffmpeg {
            return Err(ToolError(format!(
                "Extracted archive did not contain {}",
                executable_name()
            )));
        }
        let _ = std::fs::write(tools.join(VERSION_FILE), tag);
        log(sink, format!("ffmpeg ready ({tag})."));
        Ok(())
    }
}

#[cfg(windows)]
use win::{ensure_bundled, ensure_path_only};
