use crate::http;
use crate::json::{self, Value};
use crate::version::VERSION;
use crate::version_cmp;

pub const GITHUB_REPO: &str = "Aspenini/quark-downloader";
pub const INSTALLER_NAME_PREFIX: &str = "quark-downloader";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    UpToDate,
    Ahead,
    Behind,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehindInfo {
    pub latest_tag: String,
    pub latest_version: String,
    pub download_url: String,
}

pub fn installer_download_url(tag_name: &str) -> String {
    let version = tag_name.trim_start_matches('v');
    format!(
        "https://github.com/{GITHUB_REPO}/releases/download/{tag_name}/{INSTALLER_NAME_PREFIX}-{version}-setup.exe"
    )
}

pub fn normalize_tag(tag_name: &str) -> String {
    tag_name.trim_start_matches('v').to_string()
}

pub fn status_from_release(
    release: &Value,
    installed: &str,
) -> (Status, Option<String>, Option<BehindInfo>) {
    let tag = release.get_str("tag_name").unwrap_or("").to_string();
    let latest = normalize_tag(&tag);
    match version_cmp::compare(installed, &latest) {
        0 => (Status::UpToDate, Some(latest), None),
        1 => (Status::Ahead, Some(latest), None),
        _ => {
            let behind = BehindInfo {
                download_url: installer_download_url(&tag),
                latest_tag: tag,
                latest_version: latest.clone(),
            };
            (Status::Behind, Some(latest), Some(behind))
        }
    }
}

pub fn check() -> (Status, Option<String>, Option<BehindInfo>) {
    match fetch_latest_release() {
        Ok(release) => status_from_release(&release, VERSION),
        Err(_) => (Status::Failed, None, None),
    }
}

pub fn check_with_error() -> (Status, Option<String>, Option<BehindInfo>, Option<String>) {
    match fetch_latest_release() {
        Ok(release) => {
            let (status, latest, behind) = status_from_release(&release, VERSION);
            (status, latest, behind, None)
        }
        Err(ex) => (Status::Failed, None, None, Some(ex)),
    }
}

fn fetch_latest_release() -> Result<Value, String> {
    let body = http::fetch_body(&format!(
        "https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
    ))
    .map_err(|e| e.to_string())?;
    json::parse(&body).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_installer_url() {
        assert_eq!(
            installer_download_url("v0.4.0"),
            "https://github.com/Aspenini/quark-downloader/releases/download/v0.4.0/quark-downloader-0.4.0-setup.exe"
        );
    }

    #[test]
    fn classifies_release_status() {
        let release = json::parse(r#"{"tag_name":"v0.4.0"}"#).unwrap();
        let (status, latest, behind) = status_from_release(&release, "0.3.0");
        assert_eq!(status, Status::Behind);
        assert_eq!(latest.as_deref(), Some("0.4.0"));
        assert_eq!(
            behind.unwrap().download_url,
            installer_download_url("v0.4.0")
        );

        let (status, latest, _) = status_from_release(&release, "0.4.0");
        assert_eq!(status, Status::UpToDate);
        assert_eq!(latest.as_deref(), Some("0.4.0"));

        let (status, latest, _) = status_from_release(&release, "0.5.0");
        assert_eq!(status, Status::Ahead);
        assert_eq!(latest.as_deref(), Some("0.4.0"));
    }
}
