//! Detection of a JavaScript runtime for yt-dlp's EJS (YouTube).

use std::sync::OnceLock;

use crate::util::find_executable;

/// Runtimes yt-dlp's EJS supports, highest priority first (the order yt-dlp
/// itself prefers). Each maps the runtime name yt-dlp expects to the binaries
/// to look for on PATH. `bun` is last because yt-dlp has deprecated it.
const JS_RUNTIMES: &[(&str, &[&str])] = &[
    ("deno", &["deno"]),
    ("node", &["node"]),
    ("quickjs", &["qjs", "quickjs"]),
    ("bun", &["bun"]),
];

/// The yt-dlp runtime name for whichever JS engine is on PATH, if any.
/// The PATH scan runs once per process; callers may probe repeatedly.
pub fn detect() -> Option<&'static str> {
    static DETECTED: OnceLock<Option<&'static str>> = OnceLock::new();
    *DETECTED.get_or_init(|| {
        for (name, binaries) in JS_RUNTIMES {
            if binaries.iter().any(|bin| find_executable(bin).is_some()) {
                return Some(name);
            }
        }
        None
    })
}
