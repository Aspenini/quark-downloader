//! Minimal HTTP helpers for fetching release metadata and downloading tools.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

pub const USER_AGENT: &str = "quark-downloader";

#[derive(Debug)]
pub struct HttpError(pub String);

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for HttpError {}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .timeout_read(Duration::from_secs(120))
        .redirects(5)
        .user_agent(USER_AGENT)
        .build()
}

/// GET a URL and return the body as a string (follows redirects).
pub fn fetch_body(url: &str) -> Result<String, HttpError> {
    let resp = agent()
        .get(url)
        .call()
        .map_err(|e| HttpError(format!("HTTP request failed: {e}")))?;
    let body = resp
        .into_string()
        .map_err(|e| HttpError(format!("reading {url}: {e}")))?;
    if body.is_empty() {
        return Err(HttpError(format!("Empty response from {url}")));
    }
    Ok(body)
}

/// GET a URL and return the raw bytes (follows redirects).
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, HttpError> {
    let resp = agent()
        .get(url)
        .call()
        .map_err(|e| HttpError(format!("HTTP request failed: {e}")))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| HttpError(format!("reading {url}: {e}")))?;
    if buf.is_empty() {
        return Err(HttpError(format!("Empty response from {url}")));
    }
    Ok(buf)
}

/// Download a URL to a file, replacing any existing file. Streams the body
/// straight to disk, so large tool downloads never sit in memory.
pub fn download_file(url: &str, dest: &Path) -> Result<(), HttpError> {
    let resp = agent()
        .get(url)
        .call()
        .map_err(|e| HttpError(format!("HTTP request failed: {e}")))?;
    if dest.exists() {
        let _ = std::fs::remove_file(dest);
    }
    let mut file = std::fs::File::create(dest)
        .map_err(|e| HttpError(format!("writing {}: {e}", dest.display())))?;
    let written = std::io::copy(&mut resp.into_reader(), &mut file).map_err(|e| {
        let _ = std::fs::remove_file(dest);
        HttpError(format!("writing {}: {e}", dest.display()))
    })?;
    if written == 0 {
        let _ = std::fs::remove_file(dest);
        return Err(HttpError(format!("Empty response from {url}")));
    }
    Ok(())
}
