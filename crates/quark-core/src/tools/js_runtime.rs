//! Detection of a JavaScript runtime for yt-dlp's EJS (YouTube).

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
pub fn detect() -> Option<&'static str> {
    for (name, binaries) in JS_RUNTIMES {
        if binaries.iter().any(|bin| find_executable(bin).is_some()) {
            return Some(name);
        }
    }
    None
}
