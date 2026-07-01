//! Parsing of yt-dlp output: our structured `QPROG` progress line plus the
//! playlist/post-processing markers the stall watchdog and UI care about.

use std::sync::OnceLock;

use regex::Regex;

/// The progress-template prefix we instruct yt-dlp to print. Owning the format
/// makes percent/ETA deterministic instead of scraping human output.
pub const PROGRESS_TOKEN: &str = "QPROG";
pub const PROGRESS_TEMPLATE: &str =
    "download:QPROG\t%(progress._percent_str)s\t%(progress.eta)s\t%(progress._total_bytes_estimate_str)s";

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadProgress {
    pub percent: f64,
    pub eta: Option<String>,
}

fn playlist_item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\[download\] Downloading item (\d+) of (\d+)").unwrap())
}

fn postprocess_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^\[(?:Merger|ExtractAudio|VideoConvertor|VideoRemuxer|Recode|Fixup\w*|Metadata|EmbedSubtitle|EmbedThumbnail|SponsorBlock|ModifyChapters|SplitChapters)\]",
        )
        .unwrap()
    })
}

fn resume_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\[download\]|Extracting URL").unwrap())
}

/// Parse a `QPROG\t<percent>\t<eta>\t<bytes>` line into structured progress.
pub fn parse_progress(line: &str) -> Option<DownloadProgress> {
    let rest = line.strip_prefix(PROGRESS_TOKEN)?.strip_prefix('\t')?;
    let mut fields = rest.splitn(3, '\t');
    let percent_str = fields.next()?.trim();
    let percent = percent_str
        .trim_end_matches('%')
        .trim()
        .parse::<f64>()
        .ok()?;
    let eta = fields.next().map(str::trim).and_then(normalize_eta);
    Some(DownloadProgress { percent, eta })
}

fn normalize_eta(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.eq_ignore_ascii_case("na")
        || raw.eq_ignore_ascii_case("n/a")
        || raw.eq_ignore_ascii_case("none")
        || raw.eq_ignore_ascii_case("unknown")
        || raw == "--:--"
    {
        return None;
    }
    Some(raw.to_string())
}

/// `(item, total)` if the line announces a new playlist item.
pub fn parse_playlist_item(line: &str) -> Option<(u32, u32)> {
    let caps = playlist_item_re().captures(line)?;
    Some((caps[1].parse().ok()?, caps[2].parse().ok()?))
}

pub fn is_postprocessing(line: &str) -> bool {
    postprocess_re().is_match(line)
}

pub fn is_resume(line: &str) -> bool {
    resume_re().is_match(line)
}

/// A short, display-worthy status line, or `None` to skip.
pub fn status_line(line: &str) -> Option<String> {
    let stripped = line.trim();
    if stripped.is_empty()
        || stripped == "Done."
        || stripped.starts_with("Deleting original file")
        || stripped.starts_with(PROGRESS_TOKEN)
    {
        return None;
    }
    Some(stripped.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_progress() {
        let p = parse_progress("QPROG\t 42.5%\t00:13\t10.5MiB").unwrap();
        assert!((p.percent - 42.5).abs() < 1e-9);
        assert_eq!(p.eta.as_deref(), Some("00:13"));
    }

    #[test]
    fn unknown_eta_becomes_none() {
        let p = parse_progress("QPROG\t100%\tNA\t1MiB").unwrap();
        assert_eq!(p.eta, None);
    }

    #[test]
    fn non_progress_line_ignored() {
        assert!(parse_progress("[download] Destination: x.mp4").is_none());
    }

    #[test]
    fn playlist_item_and_markers() {
        assert_eq!(
            parse_playlist_item("[download] Downloading item 3 of 12"),
            Some((3, 12))
        );
        assert!(is_postprocessing("[Merger] Merging formats"));
        assert!(is_resume("[download] Destination: x"));
        assert!(status_line("QPROG\t1%\t1\t1").is_none());
    }
}
