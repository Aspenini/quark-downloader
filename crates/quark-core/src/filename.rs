use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpacesPolicy {
    #[default]
    Keep,
    Underscore,
    Dash,
    Remove,
}

pub const MAX_COMPONENT_LENGTH: usize = 180;

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Sanitizes a single path component (a file stem or a directory name).
pub fn sanitize_component(name: &str, ascii_only: bool, spaces: SpacesPolicy) -> String {
    let mut result = if ascii_only {
        apply_pre_table(name)
    } else {
        name.to_string()
    };
    result = remove_control_chars(&result);
    result = result.replace(['/', '\\'], "-");

    if ascii_only {
        result = replace_windows_invalid(&result);
        result = transliterate_ascii(&result);
        result = replace_windows_invalid(&result.replace(['/', '\\'], "-"));
    }

    result = collapse_whitespace(&result);
    result = apply_spaces_policy(&result, spaces);
    result = result.trim_matches([' ', '.']).to_string();
    if result.chars().count() > MAX_COMPONENT_LENGTH {
        result = result.chars().take(MAX_COMPONENT_LENGTH).collect();
        result = result.trim_matches([' ', '.']).to_string();
    }
    if WINDOWS_RESERVED
        .iter()
        .any(|reserved| result.eq_ignore_ascii_case(reserved))
    {
        result.push('_');
    }
    if result.is_empty() {
        result = "download".into();
    }
    result
}

pub fn sanitize_filename(filename: &str, ascii_only: bool, spaces: SpacesPolicy) -> String {
    let (stem, extension) = split_extension(filename);
    format!(
        "{}{extension}",
        sanitize_component(stem, ascii_only, spaces)
    )
}

pub fn collision_free(dir: &Path, filename: &str) -> Option<String> {
    if !dir.join(filename).exists() {
        return Some(filename.to_string());
    }
    let (stem, extension) = split_extension(filename);
    for n in 2..=99 {
        let candidate = format!("{stem} ({n}){extension}");
        if !dir.join(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

fn split_extension(filename: &str) -> (&str, &str) {
    match filename.rfind('.') {
        Some(i) if i > 0 && i + 1 < filename.len() => (&filename[..i], &filename[i..]),
        _ => (filename, ""),
    }
}

fn apply_pre_table(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '｜' | '：' | '＼' | '／' => out.push('-'),
            '？' | '＊' => {}
            '＂' | '“' | '”' | '‘' | '’' | '´' | '`' => out.push('\''),
            '＜' => out.push('('),
            '＞' => out.push(')'),
            '–' | '—' | '−' | '‐' | '・' => out.push('-'),
            '…' => out.push_str("..."),
            '×' => out.push('x'),
            other => out.push(other),
        }
    }
    out
}

fn remove_control_chars(text: &str) -> String {
    text.chars()
        .filter(|c| {
            let o = *c as u32;
            o > 0x1F && !(0x7F..=0x9F).contains(&o)
        })
        .collect()
}

fn replace_windows_invalid(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            ':' | '|' => out.push('-'),
            '<' | '>' | '"' | '?' | '*' => {}
            other => out.push(other),
        }
    }
    out
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
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

fn transliterate_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if let Some(mapped) = latin_map(c) {
            out.push_str(mapped);
            continue;
        }
        let o = c as u32;
        if (0x0300..=0x036F).contains(&o) {
            continue;
        }
        if o < 0x80 {
            out.push(c);
        }
    }
    out
}

fn latin_map(c: char) -> Option<&'static str> {
    Some(match c {
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' => "A",
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'Ç' | 'Ć' | 'Ĉ' | 'Ċ' | 'Č' => "C",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'Ð' | 'Ď' | 'Đ' => "D",
        'ð' | 'ď' | 'đ' => "d",
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ĕ' | 'Ė' | 'Ę' | 'Ě' => "E",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'Ĝ' | 'Ğ' | 'Ġ' | 'Ģ' => "G",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'Ĥ' | 'Ħ' => "H",
        'ĥ' | 'ħ' => "h",
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ĩ' | 'Ī' | 'Ĭ' | 'Į' | 'İ' => "I",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'Ĵ' => "J",
        'ĵ' => "j",
        'Ķ' => "K",
        'ķ' => "k",
        'Ĺ' | 'Ļ' | 'Ľ' | 'Ŀ' | 'Ł' => "L",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'Ñ' | 'Ń' | 'Ņ' | 'Ň' => "N",
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŉ' => "n",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' | 'Ŏ' | 'Ő' => "O",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'Ŕ' | 'Ŗ' | 'Ř' => "R",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'Ś' | 'Ŝ' | 'Ş' | 'Š' => "S",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'Ţ' | 'Ť' | 'Ŧ' => "T",
        'ţ' | 'ť' | 'ŧ' => "t",
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ũ' | 'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' => "U",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'Ŵ' => "W",
        'ŵ' => "w",
        'Ý' | 'Ŷ' | 'Ÿ' => "Y",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'Ź' | 'Ż' | 'Ž' => "Z",
        'ź' | 'ż' | 'ž' => "z",
        'Æ' => "AE",
        'æ' => "ae",
        'Œ' => "OE",
        'œ' => "oe",
        'Þ' => "Th",
        'þ' => "th",
        'ß' => "ss",
        'ẞ' => "SS",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn maps_fullwidth_substitutes() {
        let cases = [
            ("A｜B", "A-B"),
            ("A：B", "A-B"),
            ("A＼B", "A-B"),
            ("A／B", "A-B"),
            ("A？B", "AB"),
            ("A＊B", "AB"),
            ("A＂B＂", "A'B'"),
            ("A＜B＞", "A(B)"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                sanitize_component(input, true, SpacesPolicy::Keep),
                expected
            );
        }
    }

    #[test]
    fn maps_typographic_punctuation() {
        assert_eq!(
            sanitize_component("a – b — c − d", true, SpacesPolicy::Keep),
            "a - b - c - d"
        );
        assert_eq!(
            sanitize_component("“quote” and ‘this’", true, SpacesPolicy::Keep),
            "'quote' and 'this'"
        );
        assert_eq!(
            sanitize_component("wait… now", true, SpacesPolicy::Keep),
            "wait... now"
        );
        assert_eq!(
            sanitize_component("wait…", true, SpacesPolicy::Keep),
            "wait"
        );
        assert_eq!(
            sanitize_component("1080×720", true, SpacesPolicy::Keep),
            "1080x720"
        );
    }

    #[test]
    fn handles_example_title() {
        let title = "The Big Bang Theory Season 6 ｜ Bloopers vs Actual Scene";
        assert_eq!(
            sanitize_component(title, true, SpacesPolicy::Keep),
            "The Big Bang Theory Season 6 - Bloopers vs Actual Scene"
        );
    }

    #[test]
    fn transliterates_accents_and_drops_other_non_ascii() {
        assert_eq!(
            sanitize_component("Café Crème", true, SpacesPolicy::Keep),
            "Cafe Creme"
        );
        assert_eq!(
            sanitize_component("abc 日本語 def", true, SpacesPolicy::Keep),
            "abc def"
        );
    }

    #[test]
    fn keeps_non_ascii_when_disabled() {
        assert_eq!(
            sanitize_component("Café 日本語", false, SpacesPolicy::Keep),
            "Café 日本語"
        );
    }

    #[test]
    fn always_removes_path_separators_and_controls() {
        assert_eq!(
            sanitize_component("a/b\\c", false, SpacesPolicy::Keep),
            "a-b-c"
        );
        assert_eq!(
            sanitize_component("a \u{0001}bcd", false, SpacesPolicy::Keep),
            "a bcd"
        );
    }

    #[test]
    fn replaces_windows_invalid() {
        assert_eq!(
            sanitize_component(r#"a:b|c<d>e"f?g*h"#, true, SpacesPolicy::Keep),
            "a-b-cdefgh"
        );
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(
            sanitize_component("a \t  b\n c", true, SpacesPolicy::Keep),
            "a b c"
        );
    }

    #[test]
    fn applies_each_spaces_policy() {
        assert_eq!(
            sanitize_component("a b c", true, SpacesPolicy::Keep),
            "a b c"
        );
        assert_eq!(
            sanitize_component("a b c", true, SpacesPolicy::Underscore),
            "a_b_c"
        );
        assert_eq!(
            sanitize_component("a b c", true, SpacesPolicy::Dash),
            "a-b-c"
        );
        assert_eq!(
            sanitize_component("a b c", true, SpacesPolicy::Remove),
            "abc"
        );
    }

    #[test]
    fn trims_spaces_and_dots() {
        assert_eq!(
            sanitize_component("  name.. ", true, SpacesPolicy::Keep),
            "name"
        );
        assert_eq!(
            sanitize_component(". hidden .", true, SpacesPolicy::Keep),
            "hidden"
        );
    }

    #[test]
    fn suffixes_windows_reserved_names() {
        assert_eq!(sanitize_component("CON", true, SpacesPolicy::Keep), "CON_");
        assert_eq!(
            sanitize_component("com1", true, SpacesPolicy::Keep),
            "com1_"
        );
        assert_eq!(
            sanitize_component("CONCERT", true, SpacesPolicy::Keep),
            "CONCERT"
        );
    }

    #[test]
    fn empty_falls_back() {
        assert_eq!(sanitize_component("", true, SpacesPolicy::Keep), "download");
        assert_eq!(
            sanitize_component("？＊", true, SpacesPolicy::Keep),
            "download"
        );
    }

    #[test]
    fn truncates_long_components() {
        let long = "a".repeat(400);
        assert_eq!(
            sanitize_component(&long, true, SpacesPolicy::Keep).len(),
            MAX_COMPONENT_LENGTH
        );
    }

    #[test]
    fn sanitizes_filename_stem() {
        assert_eq!(
            sanitize_filename("Video ｜ Clip [x].mp4", true, SpacesPolicy::Keep),
            "Video - Clip [x].mp4"
        );
        assert_eq!(
            sanitize_filename("A B.webm", true, SpacesPolicy::Underscore),
            "A_B.webm"
        );
        assert_eq!(
            sanitize_filename("plain ｜ name", true, SpacesPolicy::Keep),
            "plain - name"
        );
    }

    #[test]
    fn collision_free_numbers() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("quark-sanitize-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(collision_free(&dir, "a.mp4").as_deref(), Some("a.mp4"));
        fs::write(dir.join("a.mp4"), b"").unwrap();
        assert_eq!(collision_free(&dir, "a.mp4").as_deref(), Some("a (2).mp4"));
        fs::write(dir.join("a (2).mp4"), b"").unwrap();
        assert_eq!(collision_free(&dir, "a.mp4").as_deref(), Some("a (3).mp4"));
        let _ = fs::remove_dir_all(&dir);
    }
}
