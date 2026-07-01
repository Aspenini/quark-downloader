//! Locate, download, and keep yt-dlp up to date.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use super::{js_runtime, ToolError};
use crate::config::ToolSource;
use crate::events::{EventSink, ProgressEvent};
use crate::net::http;
use crate::{paths, version_compare};

const GITHUB_LATEST_URL: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const VERSION_FILE: &str = ".yt-dlp-version";
const CHECK_AT_FILE: &str = ".yt-dlp-check-at";
const MIN_YOUTUBE_YTDLP: &str = "2025.01.26";

fn log(sink: &dyn EventSink, msg: impl Into<String>) {
    sink.emit(ProgressEvent::Log(msg.into()));
}

pub fn asset_name() -> &'static str {
    #[cfg(windows)]
    {
        "yt-dlp.exe"
    }
    #[cfg(target_os = "macos")]
    {
        "yt-dlp_macos"
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        "yt-dlp"
    }
}

pub fn bundled_path() -> PathBuf {
    paths::tools_dir().join(asset_name())
}

fn skip_update() -> bool {
    std::env::var("QUARK_SKIP_YTDLP_UPDATE").as_deref() == Ok("1")
}

fn path_executable() -> Option<PathBuf> {
    crate::util::find_executable("yt-dlp")
}

pub fn youtube_url(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    u.contains("youtube.com") || u.contains("youtu.be")
}

/// Resolve the yt-dlp binary to use per the configured source.
pub fn ensure(source: ToolSource, sink: &dyn EventSink) -> Result<PathBuf, ToolError> {
    match source {
        ToolSource::Path => ensure_path_only(sink),
        ToolSource::Bundled => ensure_bundled(sink),
        ToolSource::Auto => ensure_auto(sink),
    }
}

fn ensure_auto(sink: &dyn EventSink) -> Result<PathBuf, ToolError> {
    if let Some(path) = path_executable() {
        if let Some(version) = read_version(&path) {
            if version_compare::at_least(&version, MIN_YOUTUBE_YTDLP) {
                log(sink, format!("Using yt-dlp from PATH: {}", path.display()));
                warn_youtube_js_runtime(sink);
                return Ok(path);
            }
            log(
                sink,
                format!("yt-dlp on PATH ({version}) is too old for YouTube; using bundled copy."),
            );
        }
    }
    ensure_bundled(sink)
}

fn ensure_path_only(sink: &dyn EventSink) -> Result<PathBuf, ToolError> {
    if let Some(path) = path_executable() {
        log(sink, format!("Using yt-dlp from PATH: {}", path.display()));
        warn_if_stale(&path, sink);
        warn_youtube_js_runtime(sink);
        return Ok(path);
    }
    Err(ToolError(
        "yt-dlp not found on PATH (quark-downloader.toml: yt_dlp = path).\n\
         Install yt-dlp and add it to PATH, or set yt_dlp = auto or bundled."
            .to_string(),
    ))
}

fn ensure_bundled(sink: &dyn EventSink) -> Result<PathBuf, ToolError> {
    let tools = paths::tools_dir();
    std::fs::create_dir_all(&tools).ok();
    let bundled = bundled_path();

    if !bundled.exists() {
        if skip_update() {
            return Err(ToolError(format!(
                "yt-dlp not found in tools/ (quark-downloader.toml: yt_dlp = bundled).\n\
                 Place {} in tools/ or unset QUARK_SKIP_YTDLP_UPDATE to allow download.",
                asset_name()
            )));
        }
        log(sink, "Downloading yt-dlp...");
        download_latest(sink)?;
        return Ok(bundled);
    }

    log(sink, format!("Using yt-dlp from: {}", bundled.display()));
    if check_due() {
        check_and_update_if_needed(sink);
    }
    warn_youtube_js_runtime(sink);
    Ok(bundled)
}

pub fn read_version(path: &Path) -> Option<String> {
    let output = std::process::Command::new(path)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace().next().map(|s| s.to_string())
}

fn warn_if_stale(path: &Path, sink: &dyn EventSink) {
    if let Some(version) = read_version(path) {
        if !version_compare::at_least(&version, MIN_YOUTUBE_YTDLP) {
            log(sink, "");
            log(
                sink,
                format!("Warning: yt-dlp {version} is likely too old for YouTube."),
            );
            log(
                sink,
                "  Update: pipx install -U yt-dlp   (or brew upgrade yt-dlp)",
            );
        }
    }
}

fn warn_youtube_js_runtime(sink: &dyn EventSink) {
    if js_runtime::detect().is_none() {
        log(sink, "Warning: No JavaScript runtime on PATH (deno, node, quickjs, or bun) - YouTube may fail until you install one (yt-dlp EJS wiki).");
    }
}

/// Raise an error if the URL is YouTube but no JS runtime is available.
pub fn preflight_youtube(url: &str) -> Result<(), ToolError> {
    if !youtube_url(url) || js_runtime::detect().is_some() {
        return Ok(());
    }
    Err(ToolError(
        "YouTube requires a JavaScript runtime for yt-dlp (EJS).\n\
         Install any one of: deno, node, quickjs, or bun.\n\
         \u{20}\u{20}- macOS (Homebrew): brew install deno   (or node)\n\
         \u{20}\u{20}- Linux: sudo apt install nodejs   (or your distro equivalent)\n\
         \u{20}\u{20}- More: https://github.com/yt-dlp/yt-dlp/wiki/EJS"
            .to_string(),
    ))
}

/// Extra args enabling the detected JS runtime for YouTube URLs.
pub fn extra_args(url: &str) -> Vec<String> {
    if !youtube_url(url) {
        return Vec::new();
    }
    match js_runtime::detect() {
        Some(runtime) => vec![
            "--js-runtimes".into(),
            runtime.into(),
            "--remote-components".into(),
            "ejs:github".into(),
        ],
        None => Vec::new(),
    }
}

pub fn youtube_failure_hints() -> String {
    let mut hints = String::from(
        "YouTube download failed. Common fixes:\n\
         \u{20}\u{20}- Let quark-downloader use a bundled yt-dlp (yt_dlp = auto in quark-downloader.toml)\n\
         \u{20}\u{20}- Or update PATH: pipx install -U yt-dlp   (or brew upgrade yt-dlp)",
    );
    if js_runtime::detect().is_none() {
        hints.push_str(
            "\n\u{20}\u{20}- Install a JS runtime for YouTube: sudo apt install nodejs\n\
             \u{20}\u{20}\u{20}\u{20}https://github.com/yt-dlp/yt-dlp/wiki/EJS",
        );
    }
    hints
}

// ---- update machinery ----------------------------------------------------

fn check_due() -> bool {
    let check_file = paths::tools_dir().join(CHECK_AT_FILE);
    let Ok(text) = std::fs::read_to_string(&check_file) else {
        return true;
    };
    let Ok(last) = text.trim().parse::<u64>() else {
        return true;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(last) >= CHECK_INTERVAL_SECS
}

fn record_check_time() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = std::fs::write(paths::tools_dir().join(CHECK_AT_FILE), now.to_string());
}

fn check_and_update_if_needed(sink: &dyn EventSink) {
    record_check_time();
    match try_check_and_update(sink) {
        Ok(()) => {}
        Err(e) => {
            if bundled_path().exists() {
                log(sink, format!("yt-dlp update skipped: {e}"));
            }
        }
    }
}

fn try_check_and_update(sink: &dyn EventSink) -> Result<(), ToolError> {
    let release = fetch_latest_release()?;
    let latest = tag_name(&release)?.trim_start_matches('v').to_string();
    let installed = installed_version();
    if let Some(installed) = &installed {
        if !version_compare::newer(&latest, installed) {
            return Ok(());
        }
    }
    log(
        sink,
        format!(
            "Updating yt-dlp ({} -> {latest})...",
            installed.as_deref().unwrap_or("none")
        ),
    );
    download_release(&release, sink)
}

fn download_latest(sink: &dyn EventSink) -> Result<(), ToolError> {
    let release = fetch_latest_release()?;
    download_release(&release, sink)
}

fn fetch_latest_release() -> Result<serde_json::Value, ToolError> {
    let body = http::fetch_body(GITHUB_LATEST_URL)?;
    serde_json::from_str(&body).map_err(|e| ToolError(format!("parsing release JSON: {e}")))
}

fn tag_name(release: &serde_json::Value) -> Result<String, ToolError> {
    release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError("release has no tag_name".into()))
}

fn download_release(release: &serde_json::Value, sink: &dyn EventSink) -> Result<(), ToolError> {
    let tag = tag_name(release)?;
    let asset = find_asset(release, asset_name())?;
    let url = asset
        .get("browser_download_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError("asset missing URL".into()))?;
    let name = asset
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(asset_name());

    let tools = paths::tools_dir();
    let tmp = tools.join(format!("{name}.download"));
    let has_sums = find_asset(release, "SHA2-256SUMS").is_ok();

    log(sink, format!("Fetching {name} ({tag})..."));
    http::download_file(url, &tmp)?;
    if has_sums {
        verify_checksum(release, name, &tmp)?;
    }
    install_binary(&tmp, &bundled_path())?;
    let _ = std::fs::write(tools.join(VERSION_FILE), tag.trim_start_matches('v'));
    log(sink, format!("yt-dlp ready ({tag})."));
    Ok(())
}

fn find_asset<'a>(
    release: &'a serde_json::Value,
    name: &str,
) -> Result<&'a serde_json::Value, ToolError> {
    release
        .get("assets")
        .and_then(|v| v.as_array())
        .and_then(|assets| {
            assets
                .iter()
                .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(name))
        })
        .ok_or_else(|| ToolError(format!("release has no asset named {name}")))
}

fn verify_checksum(
    release: &serde_json::Value,
    binary_name: &str,
    path: &Path,
) -> Result<(), ToolError> {
    let sums_asset = find_asset(release, "SHA2-256SUMS")?;
    let url = sums_asset
        .get("browser_download_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError("checksums asset missing URL".into()))?;
    let body = http::fetch_body(url)?;

    let mut expected: Option<String> = None;
    for line in body.lines() {
        let mut it = line.split_whitespace();
        let (Some(hash), Some(entry)) = (it.next(), it.next()) else {
            continue;
        };
        if hash.len() == 64 && (entry == binary_name || entry.ends_with(&format!("/{binary_name}")))
        {
            expected = Some(hash.to_ascii_lowercase());
            break;
        }
    }
    let expected = expected
        .ok_or_else(|| ToolError(format!("SHA2-256SUMS has no entry for {binary_name}")))?;

    let data =
        std::fs::read(path).map_err(|e| ToolError(format!("reading {}: {e}", path.display())))?;
    let actual = format!("{:x}", Sha256::digest(&data));
    if actual != expected {
        let _ = std::fs::remove_file(path);
        return Err(ToolError(format!("Checksum mismatch for {binary_name}")));
    }
    Ok(())
}

fn install_binary(tmp: &Path, dest: &Path) -> Result<(), ToolError> {
    if dest.exists() {
        let _ = std::fs::remove_file(dest);
    }
    if std::fs::rename(tmp, dest).is_err() {
        std::fs::copy(tmp, dest).map_err(|e| ToolError(format!("installing binary: {e}")))?;
        let _ = std::fs::remove_file(tmp);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

pub fn installed_version() -> Option<String> {
    let version_file = paths::tools_dir().join(VERSION_FILE);
    if let Ok(v) = std::fs::read_to_string(&version_file) {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(v);
        }
    }
    let bundled = bundled_path();
    if bundled.exists() {
        return read_version(&bundled);
    }
    None
}
