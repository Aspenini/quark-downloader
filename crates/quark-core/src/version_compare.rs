//! Numeric `major.minor.patch` comparison, tolerant of yt-dlp's date-style
//! versions (e.g. `2025.01.26`). Extra components are ignored.

fn parse(v: &str) -> (i64, i64, i64) {
    let v = v.trim_start_matches('v');
    let mut parts = v.split('.').map(|p| p.trim().parse::<i64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// -1 if a < b, 0 if equal, 1 if a > b.
pub fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    parse(a).cmp(&parse(b))
}

pub fn at_least(installed: &str, minimum: &str) -> bool {
    compare(installed, minimum) != std::cmp::Ordering::Less
}

pub fn newer(latest: &str, installed: &str) -> bool {
    compare(latest, installed) == std::cmp::Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn date_style_versions() {
        assert_eq!(compare("2025.01.26", "2025.01.25"), Ordering::Greater);
        assert_eq!(compare("2024.12.01", "2025.01.26"), Ordering::Less);
        assert_eq!(compare("v2025.01.26", "2025.01.26"), Ordering::Equal);
        assert!(at_least("2025.02.01", "2025.01.26"));
        assert!(newer("2025.02.01", "2025.01.26"));
    }
}
