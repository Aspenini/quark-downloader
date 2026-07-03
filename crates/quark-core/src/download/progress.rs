//! Parsing of yt-dlp output: our structured `QPROG` progress line plus the
//! playlist/post-processing markers the stall watchdog and UI care about.

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

/// Post-processor tags yt-dlp prints as `[Tag] ...` while producing no
/// download output (`Fixup*` matches all fixup variants).
const POSTPROCESS_TAGS: &[&str] = &[
    "Merger",
    "ExtractAudio",
    "VideoConvertor",
    "VideoRemuxer",
    "Recode",
    "Metadata",
    "EmbedSubtitle",
    "EmbedThumbnail",
    "SponsorBlock",
    "ModifyChapters",
    "SplitChapters",
];

/// Split a leading run of ASCII digits off `s` as a number.
fn leading_number(s: &str) -> Option<(u32, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    Some((s[..end].parse().ok()?, &s[end..]))
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
    let rest = line.strip_prefix("[download] Downloading item ")?;
    let (item, rest) = leading_number(rest)?;
    let (total, _) = leading_number(rest.strip_prefix(" of ")?)?;
    Some((item, total))
}

pub fn is_postprocessing(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('[') else {
        return false;
    };
    let Some(end) = rest.find(']') else {
        return false;
    };
    let tag = &rest[..end];
    POSTPROCESS_TAGS.contains(&tag)
        || tag
            .strip_prefix("Fixup")
            .is_some_and(|s| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
}

pub fn is_resume(line: &str) -> bool {
    line.starts_with("[download]") || line.contains("Extracting URL")
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
