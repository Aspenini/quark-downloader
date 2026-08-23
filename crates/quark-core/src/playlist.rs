use std::process::{Command, Stdio};

use crate::json;
use crate::url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeResult {
    pub title: String,
    pub count: Option<i32>,
}

/// Heuristic: true for URLs that point at a whole playlist. Watch URLs that
/// merely carry a list= parameter (watch?v=..&list=..) stay single-video.
pub fn playlist_url(url: &str) -> bool {
    let Some(parts) = url::split(url) else {
        return false;
    };
    let host = parts.host.to_ascii_lowercase();
    let path = parts.path.to_ascii_lowercase();
    if host == "youtu.be" || host.ends_with(".youtu.be") {
        return false;
    }
    if path.contains("/playlist") || path.contains("/playlists/") || path.contains("/sets/") {
        return true;
    }
    (url::query_has(parts.query, "list") || url::query_has(parts.query, "p"))
        && !url::query_has(parts.query, "v")
}

pub fn probe(ytdlp: &str, url: &str, extra_args: &[String]) -> Option<ProbeResult> {
    let mut cmd = Command::new(ytdlp);
    cmd.args(["--flat-playlist", "-I", "1:1", "-J", "--no-warnings"]);
    cmd.args(extra_args);
    cmd.arg(url);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = json::parse(&String::from_utf8_lossy(&output.stdout)).ok()?;
    if value.get_str("_type") != Some("playlist") {
        return None;
    }
    let title = value.get_str("title")?.trim();
    if title.is_empty() {
        return None;
    }
    Some(ProbeResult {
        title: title.to_string(),
        count: value.get_i32("playlist_count"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_playlist_urls() {
        assert!(playlist_url("https://www.youtube.com/playlist?list=PLx"));
        assert!(playlist_url("https://youtube.com/playlist?list=PLx&si=abc"));
        assert!(playlist_url("https://www.youtube.com/watch?list=PLx"));
        assert!(playlist_url("https://soundcloud.com/artist/sets/my-mix"));
        assert!(playlist_url("https://example.com/playlists/123"));
    }

    #[test]
    fn watch_with_list_is_single_video() {
        assert!(!playlist_url(
            "https://www.youtube.com/watch?v=KF5gdofOO2k&list=PLx"
        ));
        assert!(!playlist_url("https://www.youtube.com/watch?v=KF5gdofOO2k"));
    }

    #[test]
    fn short_links_and_plain_urls_are_single() {
        assert!(!playlist_url("https://youtu.be/KF5gdofOO2k"));
        assert!(!playlist_url("https://youtu.be/KF5gdofOO2k?list=PLx"));
        assert!(!playlist_url("https://vimeo.com/12345"));
        assert!(!playlist_url("not a url"));
    }
}
