use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::{Settings, ToolSource};
use crate::http;
use crate::json;
use crate::logs;
use crate::process;
use crate::ytdlp;

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

static DETECTED: AtomicBool = AtomicBool::new(false);

pub fn executable_name() -> String {
    quark_platform::exe("ffmpeg")
}

pub fn ffprobe_name() -> String {
    quark_platform::exe("ffprobe")
}

pub fn tools_dir() -> PathBuf {
    ytdlp::tools_dir()
}

pub fn bundled_path() -> PathBuf {
    tools_dir().join(executable_name())
}

fn skip_download() -> bool {
    std::env::var_os("QUARK_SKIP_FFMPEG_DOWNLOAD").as_deref() == Some(std::ffi::OsStr::new("1"))
}

pub fn bundled() -> bool {
    quark_platform::allows_bundled_tools() && bundled_path().exists()
}

pub fn path_executable() -> Option<PathBuf> {
    process::which("ffmpeg")
}

pub fn locate(settings: &Settings) -> Option<(bool, PathBuf)> {
    let source = if quark_platform::allows_bundled_tools() {
        settings.ffmpeg_source()
    } else {
        ToolSource::Auto
    };
    let path_exe = matches!(source, ToolSource::Path | ToolSource::Auto)
        .then(path_executable)
        .flatten();
    if let Some(exe) = path_exe {
        return Some((true, exe));
    }
    if quark_platform::allows_bundled_tools()
        && matches!(source, ToolSource::Bundled | ToolSource::Auto)
        && bundled()
    {
        return Some((false, bundled_path()));
    }
    None
}

pub fn detect(settings: &Settings) {
    if let Some((from_path, path)) = locate(settings) {
        if from_path {
            logs::log_line(&format!("Using ffmpeg from PATH: {}", path.display()));
        } else {
            logs::log_line(&format!("Using ffmpeg from: {}", path.display()));
        }
    } else {
        warn_not_found();
    }
    DETECTED.store(true, Ordering::Relaxed);
}

pub fn ensure(settings: &Settings) -> Result<PathBuf, Error> {
    if quark_platform::allows_bundled_tools() {
        match settings.ffmpeg_source() {
            ToolSource::Path => ensure_path_only(),
            ToolSource::Bundled => ensure_bundled(),
            ToolSource::Auto => {
                if let Some(exe) = path_executable() {
                    if !DETECTED.load(Ordering::Relaxed) {
                        logs::log_line(&format!("Using ffmpeg from PATH: {}", exe.display()));
                    }
                    return Ok(exe
                        .parent()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| PathBuf::from(".")));
                }
                ensure_bundled()
            }
        }
    } else if let Some(exe) = path_executable() {
        if !DETECTED.load(Ordering::Relaxed) {
            logs::log_line(&format!("Using ffmpeg from PATH: {}", exe.display()));
        }
        Ok(exe
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")))
    } else {
        Err(Error(not_found_message()))
    }
}

fn ensure_path_only() -> Result<PathBuf, Error> {
    if let Some(exe) = path_executable() {
        if !DETECTED.load(Ordering::Relaxed) {
            logs::log_line(&format!("Using ffmpeg from PATH: {}", exe.display()));
        }
        return Ok(exe
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")));
    }
    Err(Error(
        "ffmpeg not found on PATH (quark-downloader.conf: ffmpeg = path).\nInstall ffmpeg and add it to PATH, or set ffmpeg = auto or bundled.".into(),
    ))
}

fn ensure_bundled() -> Result<PathBuf, Error> {
    let dir = tools_dir();
    fs::create_dir_all(&dir).map_err(|e| Error(e.to_string()))?;
    if bundled() {
        if !DETECTED.load(Ordering::Relaxed) {
            logs::log_line(&format!("Using ffmpeg from: {}", bundled_path().display()));
        }
        return Ok(dir);
    }
    if !skip_download() {
        logs::log_line("Downloading ffmpeg...");
        download_latest()?;
        return Ok(dir);
    }
    Err(Error(
        "ffmpeg not found in tools/ (quark-downloader.conf: ffmpeg = bundled).\nPlace ffmpeg.exe in tools/ or allow a network download when converting formats.".into(),
    ))
}

fn warn_not_found() {
    logs::log_line("");
    logs::log_line("Warning: ffmpeg not found on PATH.");
    if quark_platform::is_macos() {
        logs::log_line("  Install with Homebrew: brew install ffmpeg");
    } else if quark_platform::is_linux() {
        logs::log_line("  Install with your package manager, e.g. apt install ffmpeg");
    } else {
        logs::log_line(
            "  Install ffmpeg, add it to PATH, place binaries in bundled-tools/ and rebuild,",
        );
        logs::log_line("  or allow a network download when converting formats.");
    }
    logs::log_line("  Original-format downloads may still work; conversion requires ffmpeg.");
}

pub fn append_to_cmd(cmd: &mut Vec<String>, settings: &Settings) -> Result<(), Error> {
    cmd.push("--ffmpeg-location".into());
    cmd.push(ensure(settings)?.to_string_lossy().into_owned());
    Ok(())
}

fn not_found_message() -> String {
    if quark_platform::is_macos() {
        "ffmpeg not found on PATH.\nInstall with Homebrew: brew install ffmpeg".into()
    } else if quark_platform::is_linux() {
        "ffmpeg not found on PATH.\nInstall with your package manager, e.g. apt install ffmpeg"
            .into()
    } else {
        "ffmpeg not found on PATH.\nInstall ffmpeg, add it to PATH, place binaries in bundled-tools/ and rebuild, or allow a network download on next run.".into()
    }
}

fn download_latest() -> Result<(), Error> {
    if !quark_platform::allows_bundled_tools() {
        return Err(Error("ffmpeg auto-download is Windows-only".into()));
    }
    let release = fetch_btbn_release()?;
    let asset = find_btbn_asset(&release)?;
    let url = asset
        .get_str("browser_download_url")
        .ok_or_else(|| Error("missing ffmpeg url".into()))?;
    let name = asset.get_str("name").unwrap_or("ffmpeg.zip");
    let tag = release.get_str("tag_name").unwrap_or("unknown");
    logs::log_line(&format!("Fetching {name}..."));
    let archive = tools_dir().join(name);
    http::download_file(url, &archive).map_err(|e| Error(e.to_string()))?;
    if let Err(error) = verify_checksum(&release, name, &archive) {
        let _ = fs::remove_file(&archive);
        return Err(error);
    }
    extract_and_install(&archive, tag)?;
    let _ = fs::remove_file(&archive);
    Ok(())
}

fn fetch_btbn_release() -> Result<json::Value, Error> {
    let body = http::fetch_body("https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest")
        .map_err(|e| Error(e.to_string()))?;
    json::parse(&body).map_err(|e| Error(e.to_string()))
}

fn btbn_asset_name() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "ffmpeg-master-latest-winarm64-gpl.zip"
    } else {
        "ffmpeg-master-latest-win64-gpl.zip"
    }
}

fn find_btbn_asset(release: &json::Value) -> Result<&json::Value, Error> {
    let target = btbn_asset_name();
    release
        .get("assets")
        .and_then(json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|asset| asset.get_str("name") == Some(target))
        .ok_or_else(|| Error(format!("FFmpeg release has no asset named {target}")))
}

fn find_checksums_asset(release: &json::Value) -> Option<&json::Value> {
    release
        .get("assets")
        .and_then(json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|asset| asset.get_str("name") == Some("checksums.sha256"))
}

fn verify_checksum(release: &json::Value, archive_name: &str, path: &Path) -> Result<(), Error> {
    let sums = find_checksums_asset(release)
        .ok_or_else(|| Error("FFmpeg release is missing checksums.sha256".into()))?;
    let url = sums
        .get_str("browser_download_url")
        .ok_or_else(|| Error("missing FFmpeg checksum URL".into()))?;
    let body = http::fetch_body(url).map_err(|e| Error(e.to_string()))?;
    let expected = checksum_for(&body, archive_name)
        .ok_or_else(|| Error(format!("checksums.sha256 has no entry for {archive_name}")))?;
    let actual = quark_platform::sha256_hex(path).map_err(|e| Error(e.to_string()))?;
    if actual.eq_ignore_ascii_case(&expected) {
        Ok(())
    } else {
        Err(Error(format!("Checksum mismatch for {archive_name}")))
    }
}

fn checksum_for(body: &str, archive_name: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let hash = fields.next()?;
        let name = fields.next()?.trim_start_matches('*');
        (name == archive_name && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

fn extract_and_install(archive: &Path, version_label: &str) -> Result<(), Error> {
    let extract_dir = tools_dir().join(".ffmpeg-extract");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir).map_err(|e| Error(e.to_string()))?;
    if !extract_archive(archive, &extract_dir) {
        return Err(Error(format!("Failed to extract {}", archive.display())));
    }
    let ffmpeg_src = find_in_tree(&extract_dir, &executable_name()).ok_or_else(|| {
        Error(format!(
            "Extracted archive did not contain {}",
            executable_name()
        ))
    })?;
    install_binary(&ffmpeg_src, &bundled_path())?;
    if let Some(probe_src) = find_in_tree(&extract_dir, &ffprobe_name()) {
        install_binary(&probe_src, &tools_dir().join(ffprobe_name()))?;
    }
    let _ = fs::remove_dir_all(&extract_dir);
    let _ = fs::write(tools_dir().join(".ffmpeg-version"), version_label);
    logs::log_line(&format!("ffmpeg ready ({version_label})."));
    Ok(())
}

fn extract_archive(archive: &Path, dest: &Path) -> bool {
    if archive.extension().and_then(|s| s.to_str()) == Some("zip") {
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force",
                &archive.to_string_lossy(),
                &dest.to_string_lossy(),
            ])
            .status();
        return status.map(|s| s.success()).unwrap_or(false);
    }
    std::process::Command::new("tar")
        .args([
            "-xf",
            &archive.to_string_lossy(),
            "-C",
            &dest.to_string_lossy(),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn find_in_tree(dir: &Path, filename: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, filename: &str) -> Option<PathBuf> {
        for entry in fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = walk(&path, filename) {
                    return Some(found);
                }
            } else if path.file_name().and_then(|s| s.to_str()) == Some(filename) {
                return Some(path);
            }
        }
        None
    }
    walk(dir, filename)
}

fn install_binary(src: &Path, dest: &Path) -> Result<(), Error> {
    let _ = fs::remove_file(dest);
    fs::copy(src, dest).map_err(|e| Error(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::checksum_for;

    #[test]
    fn parses_btbn_checksum_lines() {
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let body = format!("{hash}  ffmpeg-master-latest-win64-gpl.zip\n{hash} *other.zip\n");
        assert_eq!(
            checksum_for(&body, "ffmpeg-master-latest-win64-gpl.zip").as_deref(),
            Some(hash)
        );
        assert_eq!(checksum_for(&body, "missing.zip"), None);
    }
}
