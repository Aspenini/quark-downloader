pub fn parse(v: &str) -> (i32, i32, i32) {
    let trimmed = v.strip_prefix('v').unwrap_or(v);
    let mut parts = trimmed.split('.').map(|p| p.parse::<i32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// Returns -1 if a < b, 0 if equal, 1 if a > b.
pub fn compare(a: &str, b: &str) -> i32 {
    let pa = parse(a);
    let pb = parse(b);
    match pa.cmp(&pb) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

pub fn at_least(installed: &str, minimum: &str) -> bool {
    compare(installed, minimum) >= 0
}

pub fn newer(latest: &str, installed: &str) -> bool {
    compare(latest, installed) > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_tuples() {
        assert_eq!(compare("0.3.0", "0.4.0"), -1);
        assert_eq!(compare("0.4.0", "0.3.0"), 1);
        assert_eq!(compare("0.3.0", "0.3.0"), 0);
        assert_eq!(compare("v0.3.0", "0.3.0"), 0);
    }

    #[test]
    fn reports_newer_and_at_least() {
        assert!(newer("0.4.0", "0.3.0"));
        assert!(at_least("0.3.0", "0.3.0"));
    }
}
