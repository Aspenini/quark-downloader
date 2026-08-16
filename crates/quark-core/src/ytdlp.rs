use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{self, Settings, ToolSource};
use crate::http;
use crate::json;
use crate::logs;
use crate::process;
use crate::version_cmp;

pub const MIN_YOUTUBE_YTDLP: &str = "2025.01.26";

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub fn tools_dir() -> PathBuf {
    config::app_dir().join("tools")
}

pub fn path_executable() -> Option<PathBuf> {
    process::which("yt-dlp")
}

pub fn ensure(settings: &Settings) -> Result<PathBuf, Error> {
    if cfg!(windows) {
        match settings.yt_dlp_source() {
            ToolSource::Path => ensure_path_only(),
            ToolSource::Bundled => ensure_bundled(),
            ToolSource::Auto => ensure_auto(),
        }
    } else {
        ensure_path_only()
    }
}

fn ensure_auto() -> Result<PathBuf, Error> {
    if let Some(path) = path_executable() {
        if let Some(version) = read_version(&path) {
            if version_cmp::at_least(&version, MIN_YOUTUBE_YTDLP) {
                logs::log_line(&format!("Using yt-dlp from PATH: {}", path.display()));
                warn_youtube_js_runtime();
                return Ok(path);
            }
            logs::log_line(&format!(
                "yt-dlp on PATH ({version}) is too old for YouTube; using bundled copy."
            ));
        }
    }
    ensure_bundled()
}

fn ensure_path_only() -> Result<PathBuf, Error> {
    if let Some(path) = path_executable() {
        logs::log_line(&format!("Using yt-dlp from PATH: {}", path.display()));
        warn_if_stale(&path);
        warn_youtube_js_runtime();
        return Ok(path);
    }
    Err(Error(not_found_message()))
}

fn ensure_bundled() -> Result<PathBuf, Error> {
    let dir = tools_dir();
    fs::create_dir_all(&dir).map_err(|ex| {
        Error(format!(
            "Cannot create tools directory:\n  {}\n{}\nInstall yt-dlp on PATH, or fix permissions on that folder.",
            dir.display(),
            ex
        ))
    })?;
    let bundled = bundled_path();
    if !bundled.exists() {
        if skip_update() {
            return Err(Error(format!(
                "yt-dlp not found in tools/ (quark-downloader.conf: yt_dlp = bundled).\nPlace {} in {} or unset QUARK_SKIP_YTDLP_UPDATE to allow download.",
                asset_name(),
                dir.display()
            )));
        }
        logs::log_line(&format!("Downloading yt-dlp to {}...", dir.display()));
        download_latest()?;
        return Ok(bundled);
    }
    logs::log_line(&format!("Using yt-dlp from: {}", bundled.display()));
    if check_due() {
        check_and_update_if_needed();
    }
    warn_youtube_js_runtime();
    Ok(bundled)
}

fn not_found_message() -> String {
    if cfg!(target_os = "macos") {
        "yt-dlp not found on PATH.\nInstall with Homebrew: brew install yt-dlp".into()
    } else if cfg!(target_os = "linux") {
        "yt-dlp not found on PATH.\nDistro packages (apt install yt-dlp) are often too old for YouTube.\nPrefer a current build: pipx install yt-dlp   or   pip install -U yt-dlp".into()
    } else {
        "yt-dlp not found on PATH.\nInstall yt-dlp, add it to PATH, or set yt_dlp = auto in quark-downloader.conf.".into()
    }
}

pub fn youtube_url(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    u.contains("youtube.com") || u.contains("youtu.be")
}

pub fn js_runtime() -> Option<&'static str> {
    if process::which("deno").is_some() {
        return Some("deno");
    }
    if process::which("node").is_some() {
        return Some("node");
    }
    None
}

pub fn preflight_youtube(url: &str) -> Result<(), Error> {
    if !youtube_url(url) || js_runtime().is_some() {
        return Ok(());
    }
    Err(Error(
        "YouTube requires a JavaScript runtime for yt-dlp (EJS).\n  - Node.js: sudo apt install nodejs   (or your distro equivalent)\n  - Deno: see https://github.com/yt-dlp/yt-dlp/wiki/EJS".into(),
    ))
}

pub fn extra_args(url: &str) -> Vec<String> {
    if !youtube_url(url) {
        return Vec::new();
    }
    let mut args = Vec::new();
    if let Some(runtime) = js_runtime() {
        args.extend([
            "--remote-components".into(),
            "ejs".into(),
            "--js-runtimes".into(),
            runtime.into(),
        ]);
    }
    args
}

pub fn youtube_failure_hints() -> String {
    let mut hints = if cfg!(target_os = "macos") {
        "YouTube download failed. Common fixes:\n  - brew upgrade yt-dlp\n  - Install a JS runtime: brew install node   (or deno)\n"
            .to_string()
    } else if cfg!(target_os = "linux") {
        "YouTube download failed. Common fixes:\n  - Update yt-dlp: pipx install -U yt-dlp   (or your package manager)\n  - Distro packages are often too old for YouTube\n"
            .to_string()
    } else {
        "YouTube download failed. Common fixes:\n  - Let quark-downloader use a bundled yt-dlp (yt_dlp = auto in quark-downloader.conf)\n  - Or update PATH: pipx install -U yt-dlp\n"
            .to_string()
    };
    if js_runtime().is_none() {
        if cfg!(target_os = "macos") {
            hints.push_str(
                "\n  - Install a JS runtime: brew install node\n    https://github.com/yt-dlp/yt-dlp/wiki/EJS\n",
            );
        } else {
            hints.push_str(
                "\n  - Install a JS runtime for YouTube: sudo apt install nodejs   (or brew install node)\n    https://github.com/yt-dlp/yt-dlp/wiki/EJS\n",
            );
        }
    }
    hints
}

pub fn read_version(path: &Path) -> Option<String> {
    let output = Command::new(path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .map(str::to_string)
}

fn warn_if_stale(path: &Path) {
    let Some(version) = read_version(path) else {
        return;
    };
    if version_cmp::at_least(&version, MIN_YOUTUBE_YTDLP) {
        return;
    }
    logs::log_line("");
    logs::log_line(&format!(
        "Warning: yt-dlp {version} is likely too old for YouTube."
    ));
    logs::log_line("  Update: pipx install -U yt-dlp   (or brew upgrade yt-dlp)");
}

fn warn_youtube_js_runtime() {
    if js_runtime().is_some() {
        return;
    }
    logs::log_line(
        "Warning: No Node.js or Deno on PATH - YouTube may fail until you install one (yt-dlp EJS wiki).",
    );
}

fn asset_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

fn bundled_path() -> PathBuf {
    tools_dir().join(asset_name())
}

fn skip_update() -> bool {
    std::env::var_os("QUARK_SKIP_YTDLP_UPDATE").as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn check_due() -> bool {
    let check_file = tools_dir().join(".yt-dlp-check-at");
    let Ok(text) = fs::read_to_string(&check_file) else {
        return true;
    };
    let Ok(last) = text.trim().parse::<i64>() else {
        return true;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    now - last >= 24 * 3600
}

fn record_check_time() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into());
    let _ = fs::write(tools_dir().join(".yt-dlp-check-at"), now);
}

fn check_and_update_if_needed() {
    record_check_time();
    match fetch_latest_release() {
        Ok(release) => {
            let latest = release
                .get_str("tag_name")
                .unwrap_or("")
                .trim_start_matches('v')
                .to_string();
            let installed = installed_version();
            if let Some(installed) = &installed
                && !version_cmp::newer(&latest, installed)
            {
                return;
            }
            logs::log_line(&format!(
                "Updating yt-dlp ({} -> {latest})...",
                installed.as_deref().unwrap_or("none")
            ));
            if let Err(ex) = download_release(&release) {
                if bundled_path().exists() {
                    logs::log_line(&format!("yt-dlp update skipped: {ex}"));
                } else {
                    logs::log_line(&format!("yt-dlp update failed: {ex}"));
                }
            }
        }
        Err(ex) => {
            if bundled_path().exists() {
                logs::log_line(&format!("yt-dlp update skipped: {ex}"));
            }
        }
    }
}

fn download_latest() -> Result<(), Error> {
    let release = fetch_latest_release()?;
    download_release(&release)
}

fn fetch_latest_release() -> Result<json::Value, Error> {
    let body = http::fetch_body("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
        .map_err(|e| Error(e.to_string()))?;
    json::parse(&body).map_err(|e| Error(e.to_string()))
}

fn download_release(release: &json::Value) -> Result<(), Error> {
    let tag = release.get_str("tag_name").unwrap_or("").to_string();
    let asset = find_asset(release)?;
    let url = asset
        .get_str("browser_download_url")
        .ok_or_else(|| Error("missing download url".into()))?;
    let name = asset.get_str("name").unwrap_or(asset_name());
    let tmp = tools_dir().join(format!("{name}.download"));
    logs::log_line(&format!("Fetching {name} ({tag})..."));
    http::download_file(url, &tmp).map_err(|e| Error(e.to_string()))?;
    if find_checksums_asset(release).is_some() {
        verify_checksum(release, name, &tmp)?;
    }
    install_binary(&tmp, &bundled_path())?;
    let _ = fs::write(
        tools_dir().join(".yt-dlp-version"),
        tag.trim_start_matches('v'),
    );
    logs::log_line(&format!("yt-dlp ready ({tag})."));
    Ok(())
}

fn find_asset(release: &json::Value) -> Result<&json::Value, Error> {
    let target = asset_name();
    release
        .get("assets")
        .and_then(json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|asset| asset.get_str("name") == Some(target))
        .ok_or_else(|| {
            Error(format!(
                "Release {} has no asset named {target}",
                release.get_str("tag_name").unwrap_or("?")
            ))
        })
}

fn find_checksums_asset(release: &json::Value) -> Option<&json::Value> {
    release
        .get("assets")
        .and_then(json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|asset| asset.get_str("name") == Some("SHA2-256SUMS"))
}

fn verify_checksum(release: &json::Value, binary_name: &str, path: &Path) -> Result<(), Error> {
    let Some(sums_asset) = find_checksums_asset(release) else {
        return Ok(());
    };
    let url = sums_asset
        .get_str("browser_download_url")
        .ok_or_else(|| Error("missing checksum url".into()))?;
    let sums_body = http::fetch_body(url).map_err(|e| Error(e.to_string()))?;
    let mut expected = None;
    for line in sums_body.lines() {
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else { continue };
        let Some(entry_name) = parts.next() else {
            continue;
        };
        if hash.len() == 64
            && hash.chars().all(|c| c.is_ascii_hexdigit())
            && (entry_name == binary_name || entry_name.ends_with(&format!("/{binary_name}")))
        {
            expected = Some(hash.to_ascii_lowercase());
            break;
        }
    }
    let expected =
        expected.ok_or_else(|| Error(format!("SHA2-256SUMS has no entry for {binary_name}")))?;
    let actual = sha256_file(path)?;
    if actual != expected {
        let _ = fs::remove_file(path);
        return Err(Error(format!("Checksum mismatch for {binary_name}")));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, Error> {
    quark_platform::sha256_hex(path).map_err(|e| Error(e.to_string()))
}

fn install_binary(tmp: &Path, dest: &Path) -> Result<(), Error> {
    let _ = fs::remove_file(dest);
    fs::rename(tmp, dest)
        .or_else(|_| {
            fs::copy(tmp, dest)?;
            fs::remove_file(tmp)
        })
        .map_err(|e| Error(e.to_string()))?;
    Ok(())
}

fn installed_version() -> Option<String> {
    let version_file = tools_dir().join(".yt-dlp-version");
    if let Ok(v) = fs::read_to_string(version_file) {
        let v = v.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    let bundled = bundled_path();
    if bundled.exists() {
        return read_version(&bundled);
    }
    None
}
