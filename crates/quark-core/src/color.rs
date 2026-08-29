use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};

const UNSET: u8 = 0;
const ON: u8 = 1;
const OFF: u8 = 2;

static ENABLED: AtomicU8 = AtomicU8::new(UNSET);

pub fn enabled() -> bool {
    match ENABLED.load(Ordering::Relaxed) {
        ON => true,
        OFF => false,
        _ => {
            let value = detect_enabled();
            ENABLED.store(if value { ON } else { OFF }, Ordering::Relaxed);
            value
        }
    }
}

pub fn force(value: bool) {
    ENABLED.store(if value { ON } else { OFF }, Ordering::Relaxed);
}

pub fn reset() {
    ENABLED.store(UNSET, Ordering::Relaxed);
}

fn detect_enabled() -> bool {
    if std::env::var_os("FORCE_COLOR").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return true;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var_os("QUARK_GUI").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return false;
    }
    if std::env::var("TERM").ok().as_deref() == Some("dumb") {
        return false;
    }
    if !std::io::stdout().is_terminal() {
        return false;
    }
    quark_platform::enable_virtual_terminal();
    true
}

fn wrap(code: &str, text: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn bold(text: &str) -> String {
    wrap("1", text)
}

pub fn dim(text: &str) -> String {
    wrap("2", text)
}

pub fn red(text: &str) -> String {
    wrap("31", text)
}

pub fn green(text: &str) -> String {
    wrap("32", text)
}

pub fn yellow(text: &str) -> String {
    wrap("33", text)
}

pub fn cyan(text: &str) -> String {
    wrap("36", text)
}

pub fn title(text: &str) -> String {
    wrap("1;36", text)
}

/// Drop CSI sequences (`ESC[...m`) so log files stay plain text.
pub fn strip(text: &str) -> String {
    if !text.as_bytes().contains(&0x1b) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        if let Some('[') = chars.next() {
            for x in chars.by_ref() {
                if x.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    out
}

/// Color a relayed yt-dlp/ffmpeg line for the CLI. No-ops when color is off.
pub fn tool_line(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.starts_with("ERROR:") || trimmed.starts_with("error:") {
        red(line)
    } else if trimmed.starts_with("WARNING:") || trimmed.starts_with("WARNING ") {
        yellow(line)
    } else if trimmed.starts_with("[download]") {
        cyan(line)
    } else if postprocess_line(trimmed) {
        green(line)
    } else if trimmed.starts_with('[') || trimmed.starts_with("Deleting original file") {
        dim(line)
    } else {
        line.to_string()
    }
}

fn postprocess_line(line: &str) -> bool {
    const TAGS: &[&str] = &[
        "[Merger]",
        "[ExtractAudio]",
        "[VideoConvertor]",
        "[VideoRemuxer]",
        "[Recode]",
        "[Metadata]",
        "[EmbedSubtitle]",
        "[EmbedThumbnail]",
        "[SponsorBlock]",
        "[ModifyChapters]",
        "[SplitChapters]",
    ];
    TAGS.iter().any(|t| line.starts_with(t)) || line.starts_with("[Fixup")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    struct ResetOnDrop;
    impl Drop for ResetOnDrop {
        fn drop(&mut self) {
            reset();
        }
    }

    #[test]
    fn wraps_and_strips_ansi() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _reset = ResetOnDrop;
        force(true);
        let painted = red("oops");
        assert!(painted.contains("\x1b[31m"), "{painted:?}");
        assert!(painted.contains("oops"), "{painted:?}");
        assert_eq!(strip(&painted), "oops");
        assert_eq!(strip("plain"), "plain");
        assert_eq!(title("Quark"), "\x1b[1;36mQuark\x1b[0m");
    }

    #[test]
    fn tool_line_colors_by_kind() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _reset = ResetOnDrop;
        force(true);
        assert!(tool_line("ERROR: boom").contains("\x1b[31m"));
        assert!(tool_line("WARNING: x").contains("\x1b[33m"));
        assert!(tool_line("[download]  10.0% of 1.00MiB").contains("\x1b[36m"));
        assert!(tool_line("[Merger] Merging formats").contains("\x1b[32m"));
        assert!(tool_line("[youtube] id: Downloading webpage").contains("\x1b[2m"));
        assert!(tool_line("Deleting original file x").contains("\x1b[2m"));
        force(false);
        assert_eq!(tool_line("ERROR: boom"), "ERROR: boom");
        assert_eq!(tool_line("[download]  10.0%"), "[download]  10.0%");
    }
}
