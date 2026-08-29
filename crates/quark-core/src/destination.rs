use std::sync::Mutex;

pub struct DestinationTracker {
    paths: Mutex<Vec<String>>,
    errors: Mutex<Vec<String>>,
    error_count: Mutex<u32>,
}

impl Default for DestinationTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DestinationTracker {
    pub fn new() -> Self {
        Self {
            paths: Mutex::new(Vec::new()),
            errors: Mutex::new(Vec::new()),
            error_count: Mutex::new(0),
        }
    }

    pub fn observe(&self, line: &str) {
        for part in line.split('\r') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if part.starts_with("ERROR:") {
                if let Ok(mut count) = self.error_count.lock() {
                    *count += 1;
                }
                let new_error = self
                    .errors
                    .lock()
                    .ok()
                    .filter(|errors| !errors.iter().any(|e| e == part));
                if let Some(mut errors) = new_error {
                    errors.push(part.to_string());
                }
                continue;
            }
            let Some(path) = extract_destination(part) else {
                continue;
            };
            let Ok(mut paths) = self.paths.lock() else {
                continue;
            };
            if !paths.iter().any(|p| p == path) {
                paths.push(path.to_string());
            }
        }
    }

    pub fn paths(&self) -> Vec<String> {
        self.paths.lock().map(|p| p.clone()).unwrap_or_default()
    }

    pub fn errors(&self) -> Vec<String> {
        self.errors.lock().map(|e| e.clone()).unwrap_or_default()
    }

    pub fn error_count(&self) -> u32 {
        self.error_count.lock().map(|c| *c).unwrap_or(0)
    }
}

fn extract_destination(part: &str) -> Option<&str> {
    if let Some(rest) = part.strip_prefix("[download] Destination: ") {
        return Some(rest);
    }
    if let Some(path) = part
        .strip_prefix("[Merger] Merging formats into \"")
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return Some(path);
    }
    if let Some(rest) = part.strip_prefix("[ExtractAudio] Destination: ") {
        return Some(rest);
    }
    if let Some(rest) = part.strip_prefix("[VideoConvertor] Destination: ") {
        return Some(rest);
    }
    if let Some(rest) = part.strip_prefix("[VideoRemuxer] Destination: ") {
        return Some(rest);
    }
    if let Some(path) = part
        .strip_prefix("[download] ")
        .and_then(|rest| rest.strip_suffix(" has already been downloaded"))
    {
        return Some(path);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_each_destination_line_form() {
        let tracker = DestinationTracker::new();
        tracker.observe("[download] Destination: /tmp/Video Title.f137.mp4");
        tracker.observe("[Merger] Merging formats into \"/tmp/Video Title.mp4\"");
        tracker.observe("[ExtractAudio] Destination: /tmp/Audio Title.mp3");
        tracker.observe("[VideoConvertor] Destination: /tmp/Clip.webm");
        tracker.observe("[VideoRemuxer] Destination: /tmp/Clip.mp4");
        tracker.observe("[download] /tmp/Old Video.mp4 has already been downloaded");
        assert_eq!(
            tracker.paths(),
            [
                "/tmp/Video Title.f137.mp4",
                "/tmp/Video Title.mp4",
                "/tmp/Audio Title.mp3",
                "/tmp/Clip.webm",
                "/tmp/Clip.mp4",
                "/tmp/Old Video.mp4",
            ]
        );
    }

    #[test]
    fn ignores_unrelated_and_dedupes() {
        let tracker = DestinationTracker::new();
        tracker.observe("[download]  42.0% of 10.00MiB at 2.00MiB/s ETA 00:05");
        tracker.observe("[youtube] KF5gdofOO2k: Downloading webpage");
        tracker.observe("[download] Destination: /tmp/a.mp4");
        tracker.observe("[download] Destination: /tmp/a.mp4");
        assert_eq!(tracker.paths(), ["/tmp/a.mp4"]);
    }

    #[test]
    fn handles_carriage_return_packed_lines() {
        let tracker = DestinationTracker::new();
        tracker.observe("[download]  10%\r[download] Destination: /tmp/b.mp4");
        assert_eq!(tracker.paths(), ["/tmp/b.mp4"]);
    }

    #[test]
    fn counts_error_lines() {
        let tracker = DestinationTracker::new();
        tracker.observe("ERROR: [youtube] abc: Video unavailable");
        tracker.observe("ERROR: [youtube] def: Private video");
        tracker.observe("[download] Destination: /tmp/c.mp4");
        assert_eq!(tracker.error_count(), 2);
        assert_eq!(tracker.paths(), ["/tmp/c.mp4"]);
        assert_eq!(
            tracker.errors(),
            [
                "ERROR: [youtube] abc: Video unavailable",
                "ERROR: [youtube] def: Private video",
            ]
        );
    }
}
