//! Watches yt-dlp output for the files it writes (so naming rules can be
//! applied afterwards) and counts `ERROR:` lines for the failure summary.

/// Destination patterns yt-dlp emits, as `(prefix, suffix)` literal anchors.
/// Each line has the path between `prefix` and `suffix`.
const DESTINATION_PATTERNS: &[(&str, &str)] = &[
    ("[download] Destination: ", ""),
    ("[Merger] Merging formats into \"", "\""),
    ("[ExtractAudio] Destination: ", ""),
    ("[VideoConvertor] Destination: ", ""),
    ("[VideoRemuxer] Destination: ", ""),
    ("[download] ", " has already been downloaded"),
];

#[derive(Default)]
pub struct DestinationTracker {
    paths: Vec<String>,
    error_count: u32,
}

impl DestinationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// Observe one output line (which may carry `\r`-joined segments).
    pub fn observe(&mut self, line: &str) {
        for part in line.split('\r') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if part.starts_with("ERROR:") {
                self.error_count += 1;
                continue;
            }
            for (prefix, suffix) in DESTINATION_PATTERNS {
                if let Some(rest) = part.strip_prefix(prefix) {
                    let captured = if suffix.is_empty() {
                        Some(rest)
                    } else {
                        rest.strip_suffix(suffix)
                    };
                    if let Some(path) = captured {
                        let path = path.to_string();
                        if !self.paths.contains(&path) {
                            self.paths.push(path);
                        }
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_destinations_and_errors() {
        let mut t = DestinationTracker::new();
        t.observe("[download] Destination: /tmp/video.mp4");
        t.observe("[Merger] Merging formats into \"/tmp/out.mkv\"");
        t.observe("ERROR: something broke");
        t.observe("[download] /tmp/dupe.mp4 has already been downloaded");
        assert_eq!(
            t.paths(),
            &["/tmp/video.mp4", "/tmp/out.mkv", "/tmp/dupe.mp4"]
        );
        assert_eq!(t.error_count(), 1);
    }

    #[test]
    fn dedupes_paths() {
        let mut t = DestinationTracker::new();
        t.observe("[download] Destination: /tmp/a.mp4");
        t.observe("[download] Destination: /tmp/a.mp4");
        assert_eq!(t.paths().len(), 1);
    }
}
