//! Playlist URL detection and a cheap flat-playlist probe.

use std::path::Path;
use std::process::Command;

use url::Url;

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub title: String,
    pub count: Option<i64>,
}

/// True for URLs that point at a whole playlist. A watch URL that merely
/// carries a `list=` parameter (`watch?v=..&list=..`) stays a single video.
pub fn is_playlist_url(input: &str) -> bool {
    let url = match Url::parse(input) {
        Ok(u) => u,
        Err(_) => return false,
    };
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    let path = url.path().to_ascii_lowercase();

    if host == "youtu.be" || host.ends_with(".youtu.be") {
        return false;
    }
    if path.contains("/playlist") || path.contains("/playlists/") || path.contains("/sets/") {
        return true;
    }

    let mut has_list = false;
    let mut has_v = false;
    for (k, _) in url.query_pairs() {
        match k.as_ref() {
            "list" | "p" => has_list = true,
            "v" => has_v = true,
            _ => {}
        }
    }
    has_list && !has_v
}

/// Fetch the playlist title (and item count when reported) via a flat,
/// single-item extraction. Returns `None` on any failure.
pub fn probe(ytdlp: &Path, url: &str, extra_args: &[String]) -> Option<ProbeResult> {
    let mut cmd = Command::new(ytdlp);
    cmd.args(["--flat-playlist", "-I", "1:1", "-J", "--no-warnings"]);
    cmd.args(extra_args);
    cmd.arg(url);

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    if json.get("_type").and_then(|v| v.as_str()) != Some("playlist") {
        return None;
    }
    let title = json
        .get("title")
        .and_then(|v| v.as_str())?
        .trim()
        .to_string();
    if title.is_empty() {
        return None;
    }
    let count = json.get("playlist_count").and_then(|v| v.as_i64());
    Some(ProbeResult { title, count })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_playlist_urls() {
        assert!(is_playlist_url(
            "https://www.youtube.com/playlist?list=PL123"
        ));
        assert!(is_playlist_url("https://soundcloud.com/user/sets/my-set"));
        assert!(is_playlist_url("https://www.youtube.com/watch?list=PL123"));
    }

    #[test]
    fn watch_with_v_is_single() {
        assert!(!is_playlist_url(
            "https://www.youtube.com/watch?v=abc&list=PL123"
        ));
        assert!(!is_playlist_url("https://youtu.be/abc"));
        assert!(!is_playlist_url("not a url"));
    }
}
