use std::io::Write;
use std::sync::Mutex;

pub const SETUP_PROGRESS_MAX: f64 = 8.0;
pub const SETUP_PROGRESS_STEP: f64 = 1.25;
pub const STATUS_DISPLAY_MAX: usize = 72;
pub const INACTIVITY_NOTICE_MS: u64 = 15_000;

pub fn parse_progress_percent(line: &str) -> Option<f64> {
    for part in line.split('\r').rev() {
        if let Some(pct) = extract_percent(part) {
            return Some(pct);
        }
    }
    None
}

fn extract_percent(part: &str) -> Option<f64> {
    let idx = part.find("[download]")?;
    let rest = &part[idx + "[download]".len()..];
    let rest = rest.trim_start();
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if num.is_empty() {
        return None;
    }
    let after = &rest[num.len()..];
    if after.starts_with('%') {
        num.parse().ok()
    } else {
        None
    }
}

pub fn parse_eta(line: &str) -> Option<String> {
    for part in line.split('\r').rev() {
        if let Some(eta) = extract_eta(part) {
            return Some(eta);
        }
    }
    None
}

fn extract_eta(part: &str) -> Option<String> {
    let lower = part.to_ascii_lowercase();
    let idx = lower.find("eta")?;
    // word boundary: start or non-alnum before
    if idx > 0 {
        let before = part[..idx].chars().next_back()?;
        if before.is_ascii_alphanumeric() {
            return None;
        }
    }
    let rest = part[idx + 3..].trim_start();
    let token = rest.split_whitespace().next()?;
    let token_l = token.to_ascii_lowercase();
    if token_l == "--:--" || token_l == "unknown" || token_l == "n/a" || looks_like_eta(token) {
        return Some(token.to_string());
    }
    None
}

fn looks_like_eta(token: &str) -> bool {
    let parts: Vec<&str> = token.split(':').collect();
    (parts.len() == 2 || parts.len() == 3)
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

pub fn parse_status_line(line: &str) -> Option<String> {
    let stripped = line.trim();
    if stripped.is_empty() || stripped == "Done." || stripped.starts_with("Deleting original file")
    {
        return None;
    }
    if stripped.chars().count() > STATUS_DISPLAY_MAX {
        let take = STATUS_DISPLAY_MAX.saturating_sub(3);
        let mut s: String = stripped.chars().take(take).collect();
        s.push_str("...");
        Some(s)
    } else {
        Some(stripped.to_string())
    }
}

pub fn display_download_percent(percent: f64) -> f64 {
    let bounded = percent.clamp(0.0, 100.0);
    SETUP_PROGRESS_MAX + (bounded * (100.0 - SETUP_PROGRESS_MAX) / 100.0)
}

pub fn next_setup_progress(current: f64, line: &str) -> Option<f64> {
    if parse_progress_percent(line).is_some() {
        return None;
    }
    parse_status_line(line)?;
    if current >= SETUP_PROGRESS_MAX {
        return None;
    }
    Some((current + SETUP_PROGRESS_STEP).min(SETUP_PROGRESS_MAX))
}

pub fn time_left_text(eta: Option<&str>) -> String {
    match eta {
        Some(e) => format!("{e} left"),
        None => "estimating...".into(),
    }
}

pub fn eta_status_text(eta: Option<&str>) -> String {
    format!("Time left: {}", time_left_text(eta))
}

pub fn inactivity_status(elapsed_ms: u64) -> Option<String> {
    if elapsed_ms < INACTIVITY_NOTICE_MS {
        return None;
    }
    let seconds = elapsed_ms / 1_000;
    Some(format!(
        "Waiting for network/server response ({seconds}s without output)..."
    ))
}

pub fn format_duration(mut total_seconds: i64) -> String {
    if total_seconds < 0 {
        total_seconds = 0;
    }
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

pub fn playlist_eta_text(item: Option<i32>, total: Option<i32>, elapsed_ms: u64) -> Option<String> {
    let item = item?;
    let total = total?;
    if total <= 1 {
        return None;
    }
    let completed = item - 1;
    if completed < 1 || elapsed_ms == 0 {
        return None;
    }
    let remaining = total - completed;
    if remaining <= 0 {
        return None;
    }
    let per_item_ms = elapsed_ms as f64 / f64::from(completed);
    let eta_seconds = (f64::from(remaining) * per_item_ms / 1_000.0) as i64;
    Some(format!("Playlist: ~{} left", format_duration(eta_seconds)))
}

#[derive(Debug, Clone)]
pub enum ProgressCmd {
    Progress(f64),
    Status(String),
    Eta(String),
    Queue(String),
    Done(i32),
}

impl ProgressCmd {
    pub fn encode(&self) -> String {
        match self {
            Self::Progress(p) => format!("PROGRESS\t{p}"),
            Self::Status(s) => format!("STATUS\t{s}"),
            Self::Eta(e) => format!("ETA\t{e}"),
            Self::Queue(q) => format!("QUEUE\t{q}"),
            Self::Done(code) => format!("DONE\t{code}"),
        }
    }
}

pub struct ProgressRelay {
    setup_percent: Mutex<f64>,
    download_started: Mutex<bool>,
    url_text: Mutex<String>,
    item_text: Mutex<String>,
}

impl Default for ProgressRelay {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressRelay {
    pub fn new() -> Self {
        Self {
            setup_percent: Mutex::new(0.0),
            download_started: Mutex::new(false),
            url_text: Mutex::new(String::new()),
            item_text: Mutex::new(String::new()),
        }
    }

    pub fn relay(&self, line: &str, output: &mut dyn Write) -> std::io::Result<()> {
        if let Some((cur, total)) = parse_queue_url(line) {
            if let Ok(mut u) = self.url_text.lock() {
                *u = format!("URL {cur} of {total}");
            }
            if let Ok(mut i) = self.item_text.lock() {
                i.clear();
            }
            if let Ok(mut d) = self.download_started.lock() {
                *d = false;
            }
            self.emit_queue(output)?;
            writeln!(output, "PROGRESS\t0.0")?;
            writeln!(output, "ETA\t")?;
            output.flush()?;
            if let Ok(mut s) = self.setup_percent.lock() {
                *s = 0.0;
            }
            return Ok(());
        }

        if let Some((item, total)) = parse_playlist_item(line) {
            if let Ok(mut i) = self.item_text.lock() {
                *i = format!("item {item} of {total}");
            }
            if let Ok(mut d) = self.download_started.lock() {
                *d = false;
            }
            if let Ok(mut s) = self.setup_percent.lock() {
                *s = 0.0;
            }
            self.emit_queue(output)?;
            writeln!(output, "ETA\t")?;
            output.flush()?;
        }

        let eta = parse_eta(line);
        if let Some(percent) = parse_progress_percent(line) {
            if let Ok(mut d) = self.download_started.lock() {
                *d = true;
            }
            writeln!(output, "PROGRESS\t{}", display_download_percent(percent))?;
            if let Some(eta) = eta {
                writeln!(output, "ETA\t{eta}")?;
            }
            output.flush()?;
            return Ok(());
        }

        let Some(status) = parse_status_line(line) else {
            return Ok(());
        };
        let started = self.download_started.lock().map(|d| *d).unwrap_or(false);
        if !started {
            let current = self.setup_percent.lock().map(|s| *s).unwrap_or(0.0);
            if let Some(next) = next_setup_progress(current, line) {
                if let Ok(mut s) = self.setup_percent.lock() {
                    *s = next;
                }
                writeln!(output, "PROGRESS\t{next}")?;
            }
        }
        writeln!(output, "STATUS\t{status}")?;
        if let Some(eta) = eta {
            writeln!(output, "ETA\t{eta}")?;
        }
        output.flush()
    }

    fn emit_queue(&self, output: &mut dyn Write) -> std::io::Result<()> {
        let url = self.url_text.lock().map(|u| u.clone()).unwrap_or_default();
        let item = self.item_text.lock().map(|i| i.clone()).unwrap_or_default();
        let parts: Vec<&str> = [url.as_str(), item.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            return Ok(());
        }
        writeln!(output, "QUEUE\t{}", parts.join(" - "))?;
        output.flush()
    }
}

fn parse_queue_url(line: &str) -> Option<(i32, i32)> {
    let rest = line.strip_prefix("==> URL ")?;
    let mut parts = rest.split_whitespace();
    let cur = parts.next()?.parse().ok()?;
    if parts.next() != Some("of") {
        return None;
    }
    let total = parts.next()?.trim_end_matches(':').parse().ok()?;
    Some((cur, total))
}

fn parse_playlist_item(line: &str) -> Option<(i32, i32)> {
    let idx = line.find("[download] Downloading item ")?;
    let rest = &line[idx + "[download] Downloading item ".len()..];
    let mut parts = rest.split_whitespace();
    let item = parts.next()?.parse().ok()?;
    if parts.next() != Some("of") {
        return None;
    }
    let total = parts.next()?.parse().ok()?;
    Some((item, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_percentages() {
        assert_eq!(
            parse_progress_percent("[download]  42.5% of 10.00MiB"),
            Some(42.5)
        );
        assert_eq!(
            parse_progress_percent("noise\r[download]  99.9% of 10.00MiB"),
            Some(99.9)
        );
    }

    #[test]
    fn parses_eta_values() {
        let line = "[download]  40.5% of 1.80MiB at 256.29KiB/s ETA 00:04";
        assert_eq!(parse_eta(line).as_deref(), Some("00:04"));
        assert_eq!(eta_status_text(Some("00:04")), "Time left: 00:04 left");
        assert_eq!(eta_status_text(None), "Time left: estimating...");
    }

    #[test]
    fn parses_newest_eta_from_cr() {
        let line = "[download]   1.0% of 1.00MiB ETA 01:00\r[download]   2.0% of 1.00MiB ETA 00:30";
        assert_eq!(parse_progress_percent(line), Some(2.0));
        assert_eq!(parse_eta(line).as_deref(), Some("00:30"));
    }

    #[test]
    fn inactivity_and_duration() {
        assert!(inactivity_status(14_999).is_none());
        assert_eq!(
            inactivity_status(15_000).as_deref(),
            Some("Waiting for network/server response (15s without output)...")
        );
        assert_eq!(format_duration(0), "0:00");
        assert_eq!(format_duration(75), "1:15");
        assert_eq!(format_duration(3_725), "1:02:05");
        assert_eq!(format_duration(-5), "0:00");
    }

    #[test]
    fn playlist_eta() {
        assert_eq!(
            playlist_eta_text(Some(4), Some(100), 90_000).as_deref(),
            Some("Playlist: ~48:30 left")
        );
        assert!(playlist_eta_text(Some(1), Some(100), 5_000).is_none());
        assert!(playlist_eta_text(None, Some(100), 5_000).is_none());
        assert!(playlist_eta_text(Some(6), Some(5), 40_000).is_none());
    }

    #[test]
    fn status_filter_and_setup() {
        assert!(parse_status_line("").is_none());
        assert!(parse_status_line("Done.").is_none());
        assert!(parse_status_line("Deleting original file x").is_none());
        let long = "a".repeat(100);
        let status = parse_status_line(&long).unwrap();
        assert_eq!(status.chars().count(), STATUS_DISPLAY_MAX);
        assert!(status.ends_with("..."));
        assert_eq!(display_download_percent(-1.0), SETUP_PROGRESS_MAX);
        assert_eq!(display_download_percent(0.0), SETUP_PROGRESS_MAX);
        assert_eq!(display_download_percent(50.0), 54.0);
        assert_eq!(display_download_percent(100.0), 100.0);
        assert_eq!(display_download_percent(120.0), 100.0);
    }

    #[test]
    fn setup_progress_nudges() {
        let mut setup = 0.0;
        for _ in 0..10 {
            if let Some(next) = next_setup_progress(setup, "[youtube] abc: Downloading webpage") {
                setup = next;
            }
        }
        assert_eq!(setup, SETUP_PROGRESS_MAX);
        assert!(next_setup_progress(setup, "[youtube] abc: Downloading webpage").is_none());
        assert!(next_setup_progress(2.0, "[download]  5.0% of 1.00MiB").is_none());
        assert!(next_setup_progress(2.0, "Done.").is_none());
    }

    #[test]
    fn relays_eta_and_queue() {
        let mut output = Vec::new();
        let relay = ProgressRelay::new();
        relay
            .relay(
                "[download]  25.0% of 1.00MiB at 100KiB/s ETA 00:12",
                &mut output,
            )
            .unwrap();
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("PROGRESS\t"));
        assert_eq!(lines[1], "ETA\t00:12");
    }

    #[test]
    fn emits_queue_context() {
        let mut output = Vec::new();
        let relay = ProgressRelay::new();
        relay
            .relay("==> URL 2 of 5: https://example.com/a", &mut output)
            .unwrap();
        relay
            .relay("[download] Downloading item 3 of 12", &mut output)
            .unwrap();
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines.iter().any(|l| *l == "QUEUE\tURL 2 of 5"));
        assert!(
            lines
                .iter()
                .any(|l| *l == "QUEUE\tURL 2 of 5 - item 3 of 12")
        );
        assert_eq!(lines.iter().filter(|l| l.starts_with("ETA")).count(), 2);
    }

    #[test]
    fn resets_bar_on_new_url() {
        let mut output = Vec::new();
        let relay = ProgressRelay::new();
        relay
            .relay("[download]  90.0% of 1.00MiB", &mut output)
            .unwrap();
        relay
            .relay("==> URL 2 of 2: https://example.com/b", &mut output)
            .unwrap();
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        assert_eq!(lines[lines.len() - 2], "PROGRESS\t0.0");
        assert_eq!(lines.last().copied(), Some("ETA"));
    }

    #[test]
    fn no_queue_for_single() {
        let mut output = Vec::new();
        let relay = ProgressRelay::new();
        relay
            .relay("[youtube] abc: Downloading webpage", &mut output)
            .unwrap();
        relay
            .relay("[download]  25.0% of 1.00MiB", &mut output)
            .unwrap();
        assert!(
            !String::from_utf8(output)
                .unwrap()
                .lines()
                .any(|l| l.starts_with("QUEUE"))
        );
    }

    #[test]
    fn playlist_item_restarts_setup() {
        let mut output = Vec::new();
        let relay = ProgressRelay::new();
        relay
            .relay("[download]  100.0% of 1.00MiB", &mut output)
            .unwrap();
        relay
            .relay("[download] Downloading item 2 of 3", &mut output)
            .unwrap();
        relay
            .relay("[youtube] abc: Downloading webpage", &mut output)
            .unwrap();
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines.iter().any(|l| *l == "QUEUE\titem 2 of 3"));
        assert!(lines.last().unwrap().starts_with("STATUS\t"));
    }
}
