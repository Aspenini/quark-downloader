//! Filename sanitization, reserved-name handling, and collision-free renaming.

use std::path::Path;

use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacesPolicy {
    Keep,
    Underscore,
    Dash,
    Remove,
}

const MAX_COMPONENT_LENGTH: usize = 180;

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Substitutions applied before Unicode normalization. NFKD would otherwise
/// fold the fullwidth forms yt-dlp uses (e.g. `／` for `/`) back into
/// filename-invalid characters.
fn pre_substitute(c: char) -> Option<&'static str> {
    Some(match c {
        '｜' | '：' | '＼' | '／' => "-",
        '？' | '＊' => "",
        '＂' | '“' | '”' | '‘' | '’' | '´' | '`' => "'",
        '＜' => "(",
        '＞' => ")",
        '–' | '—' | '−' | '‐' | '・' => "-",
        '…' => "...",
        '×' => "x",
        _ => return None,
    })
}

fn is_control(c: char) -> bool {
    let o = c as u32;
    o <= 0x1F || (0x7F..=0x9F).contains(&o)
}

fn replace_windows_invalid(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            ':' | '|' => Some('-'),
            '<' | '>' | '"' | '?' | '*' => None,
            other => Some(other),
        })
        .collect()
}

fn replace_separators(text: &str) -> String {
    text.chars()
        .map(|c| if c == '/' || c == '\\' { '-' } else { c })
        .collect()
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

fn apply_spaces_policy(text: &str, spaces: SpacesPolicy) -> String {
    match spaces {
        SpacesPolicy::Keep => text.to_string(),
        SpacesPolicy::Underscore => text.replace(' ', "_"),
        SpacesPolicy::Dash => text.replace(' ', "-"),
        SpacesPolicy::Remove => text.replace(' ', ""),
    }
}

fn trim_dots_spaces(text: &str) -> &str {
    text.trim_matches(|c| c == ' ' || c == '.')
}

/// Sanitize a single path component (a file stem or directory name).
pub fn sanitize_component(name: &str, ascii_only: bool, spaces: SpacesPolicy) -> String {
    let mut result = name.to_string();

    if ascii_only {
        result = result
            .chars()
            .flat_map(|c| match pre_substitute(c) {
                Some(s) => s.chars().collect::<Vec<_>>(),
                None => vec![c],
            })
            .collect();
    }

    result = result.chars().filter(|c| !is_control(*c)).collect();
    result = replace_separators(&result);

    if ascii_only {
        result = replace_windows_invalid(&result);
        result = result.nfkd().filter(|c| (*c as u32) < 0x80).collect();
        // NFKD can reintroduce invalid characters (e.g. fraction slash -> /).
        result = replace_windows_invalid(&replace_separators(&result));
    }

    result = collapse_whitespace(&result);
    result = apply_spaces_policy(&result, spaces);
    result = trim_dots_spaces(&result).to_string();

    if result.chars().count() > MAX_COMPONENT_LENGTH {
        let truncated: String = result.chars().take(MAX_COMPONENT_LENGTH).collect();
        result = trim_dots_spaces(&truncated).to_string();
    }

    if WINDOWS_RESERVED_NAMES.contains(&result.to_ascii_uppercase().as_str()) {
        result.push('_');
    }
    if result.is_empty() {
        result = "download".to_string();
    }
    result
}

/// Sanitize a filename, leaving the extension untouched.
pub fn sanitize_filename(filename: &str, ascii_only: bool, spaces: SpacesPolicy) -> String {
    let path = Path::new(filename);
    let ext = path.extension().and_then(|e| e.to_str());
    match ext {
        Some(ext) => {
            let stem = &filename[..filename.len() - ext.len() - 1];
            format!("{}.{}", sanitize_component(stem, ascii_only, spaces), ext)
        }
        None => sanitize_component(filename, ascii_only, spaces),
    }
}

/// Returns `filename` if free in `dir`, else the first `name (2..99).ext`
/// variant that is. `None` when exhausted.
pub fn collision_free(dir: &Path, filename: &str) -> Option<String> {
    if !dir.join(filename).exists() {
        return Some(filename.to_string());
    }
    let path = Path::new(filename);
    let (stem, ext) = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => (
            &filename[..filename.len() - ext.len() - 1],
            format!(".{ext}"),
        ),
        None => (filename, String::new()),
    };
    for n in 2..=99 {
        let candidate = format!("{stem} ({n}){ext}");
        if !dir.join(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullwidth_pipe_becomes_dash() {
        assert_eq!(sanitize_component("a｜b", true, SpacesPolicy::Keep), "a-b");
    }

    #[test]
    fn windows_invalid_removed() {
        assert_eq!(
            sanitize_component("a:b?c*d", true, SpacesPolicy::Keep),
            "a-bcd"
        );
    }

    #[test]
    fn accents_transliterated() {
        assert_eq!(sanitize_component("Café", true, SpacesPolicy::Keep), "Cafe");
    }

    #[test]
    fn spaces_policy_applied() {
        assert_eq!(
            sanitize_component("a  b   c", true, SpacesPolicy::Underscore),
            "a_b_c"
        );
        assert_eq!(sanitize_component("a b", true, SpacesPolicy::Remove), "ab");
    }

    #[test]
    fn reserved_names_suffixed() {
        assert_eq!(sanitize_component("CON", true, SpacesPolicy::Keep), "CON_");
        assert_eq!(sanitize_component("nul", true, SpacesPolicy::Keep), "nul_");
    }

    #[test]
    fn empty_becomes_download() {
        assert_eq!(
            sanitize_component("???", true, SpacesPolicy::Keep),
            "download"
        );
    }

    #[test]
    fn extension_preserved() {
        assert_eq!(
            sanitize_filename("My: Video.mp4", true, SpacesPolicy::Keep),
            "My- Video.mp4"
        );
    }

    #[test]
    fn non_ascii_kept_when_not_ascii_only() {
        assert_eq!(
            sanitize_component("Café", false, SpacesPolicy::Keep),
            "Café"
        );
    }
}
