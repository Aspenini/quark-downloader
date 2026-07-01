//! Compare the running version against the latest GitHub release.

use crate::net::http;
use crate::{version, version_compare};

pub const GITHUB_REPO: &str = "Aspenini/quark-downloader";
const INSTALLER_NAME_PREFIX: &str = "quark-downloader";

fn latest_url() -> String {
    format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    UpToDate,
    Ahead,
    Behind,
    Failed,
}

#[derive(Debug, Clone)]
pub struct BehindInfo {
    pub latest_tag: String,
    pub latest_version: String,
    pub download_url: String,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: Status,
    pub latest: Option<String>,
    pub behind: Option<BehindInfo>,
    pub error: Option<String>,
}

pub fn installer_download_url(tag_name: &str) -> String {
    let version = tag_name.trim_start_matches('v');
    format!("https://github.com/{GITHUB_REPO}/releases/download/{tag_name}/{INSTALLER_NAME_PREFIX}-{version}-setup.exe")
}

pub fn normalize_tag(tag_name: &str) -> String {
    tag_name.trim_start_matches('v').to_string()
}

pub fn status_from_release(release: &serde_json::Value, installed: &str) -> CheckResult {
    let tag = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let latest = normalize_tag(tag);
    match version_compare::compare(installed, &latest) {
        std::cmp::Ordering::Equal => CheckResult {
            status: Status::UpToDate,
            latest: Some(latest),
            behind: None,
            error: None,
        },
        std::cmp::Ordering::Greater => CheckResult {
            status: Status::Ahead,
            latest: Some(latest),
            behind: None,
            error: None,
        },
        std::cmp::Ordering::Less => {
            let behind = BehindInfo {
                latest_tag: tag.to_string(),
                latest_version: latest.clone(),
                download_url: installer_download_url(tag),
            };
            CheckResult {
                status: Status::Behind,
                latest: Some(latest),
                behind: Some(behind),
                error: None,
            }
        }
    }
}

/// Fetch the latest release and classify it against the running version.
pub fn check() -> CheckResult {
    match fetch_latest_release() {
        Ok(release) => status_from_release(&release, version::VERSION),
        Err(e) => CheckResult {
            status: Status::Failed,
            latest: None,
            behind: None,
            error: Some(e),
        },
    }
}

fn fetch_latest_release() -> Result<serde_json::Value, String> {
    let body = http::fetch_body(&latest_url()).map_err(|e| e.0)?;
    serde_json::from_str(&body).map_err(|e| format!("parsing release JSON: {e}"))
}
