use std::path::PathBuf;

use quark_core::color;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub urls: Vec<String>,
    pub media_type: String,
    pub format: String,
    pub output_dir: Option<PathBuf>,
    pub no_pause: bool,
    pub print_default_dir: bool,
    pub emit_result: bool,
    pub help: bool,
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Cli, String> {
    let mut cli = Cli {
        media_type: "video".into(),
        format: "original".into(),
        ..Cli::default()
    };
    let mut args = args.into_iter().peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => cli.help = true,
            "--url" => {
                cli.urls.push(
                    args.next()
                        .ok_or_else(|| "missing value for --url".to_string())?,
                );
            }
            "--batch-file" => {
                let path = args
                    .next()
                    .ok_or_else(|| "missing value for --batch-file".to_string())?;
                load_batch(&path, &mut cli.urls)?;
            }
            "--type" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --type".to_string())?;
                cli.media_type = quark_core::MediaType::parse(&value)?.as_str().into();
            }
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --format".to_string())?;
                cli.format = quark_core::Format::parse(&value)?.as_str().into();
            }
            "--output-dir" => {
                cli.output_dir =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        "missing value for --output-dir".to_string()
                    })?));
            }
            "--no-pause" => cli.no_pause = true,
            "--print-default-output-dir" => cli.print_default_dir = true,
            "--emit-result-json" => cli.emit_result = true,
            other => return Err(format!("unknown option: {other}")),
        }
    }
    Ok(cli)
}

pub fn load_batch(path: &str, urls: &mut Vec<String>) -> Result<(), String> {
    let text =
        std::fs::read_to_string(path).map_err(|_| format!("Batch file not found: {path}"))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        urls.push(line.to_string());
    }
    Ok(())
}

pub const HELP: &str = "\
Usage: quark-downloader [options]\n\n\
Interactive when run with no options.\n\n\
    --url URL                      Video or playlist URL to download (repeatable)\n\
    --batch-file FILE              File with one URL per line (# comments ignored)\n\
    --type TYPE                    audio or video (default: video)\n\
    --format FORMAT                Output format (default: original)\n\
    --output-dir DIR               Output directory\n\
    --no-pause                     Do not wait for a key press before exiting (Windows)\n\
    --print-default-output-dir     Print default output directory and exit\n\
    --emit-result-json             Print a final __RESULT__ JSON line for GUI/tools\n\
    -h, --help                     Show help";

pub fn print_help() {
    for line in HELP.lines() {
        println!("{}", colorize_help_line(line));
    }
}

fn colorize_help_line(line: &str) -> String {
    if line.starts_with("Usage:") {
        return color::bold(line);
    }
    let spaces = line.bytes().take_while(|b| *b == b' ').count();
    let trimmed = &line[spaces..];
    if !trimmed.starts_with('-') {
        if line.is_empty() {
            return String::new();
        }
        return color::dim(line);
    }
    let indent = &line[..spaces];
    if let Some(idx) = trimmed.find("  ") {
        let flags = trimmed[..idx].trim_end();
        let rest = &trimmed[flags.len()..];
        format!("{indent}{}{}", color::cyan(flags), color::dim(rest))
    } else {
        format!("{indent}{}", color::cyan(trimmed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_slice(args: &[&str]) -> Result<Cli, String> {
        parse(args.iter().map(|s| (*s).to_string()))
    }

    #[test]
    fn parses_repeatable_urls_and_flags() {
        let cli = parse_slice(&[
            "--url",
            "https://a",
            "--url",
            "https://b",
            "--type",
            "audio",
            "--format",
            "mp3",
            "--output-dir",
            "/tmp/out",
            "--no-pause",
            "--emit-result-json",
        ])
        .unwrap();
        assert_eq!(cli.urls, ["https://a", "https://b"]);
        assert_eq!(cli.media_type, "audio");
        assert_eq!(cli.format, "mp3");
        assert_eq!(
            cli.output_dir.as_deref(),
            Some(std::path::Path::new("/tmp/out"))
        );
        assert!(cli.no_pause);
        assert!(cli.emit_result);
    }

    #[test]
    fn rejects_unknown_and_missing() {
        assert!(parse_slice(&["--nope"]).is_err());
        assert!(parse_slice(&["--url"]).is_err());
        assert!(parse_slice(&["--batch-file", "no-such-file.txt"]).is_err());
        assert!(parse_slice(&["--type", "image"]).is_err());
        assert!(parse_slice(&["--format", "avi"]).is_err());
    }

    #[test]
    fn colorize_help_keeps_flag_text() {
        let usage = colorize_help_line("Usage: quark-downloader [options]");
        assert!(usage.contains("Usage: quark-downloader"));
        let flag = colorize_help_line("    --no-pause                     Do not wait");
        assert!(flag.contains("--no-pause"));
        assert!(colorize_help_line("").is_empty());
    }

    #[test]
    fn loads_batch_skipping_comments() {
        let dir = std::env::temp_dir().join(format!(
            "quark-batch-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&dir, "# c\n\nhttps://one\nhttps://two\n").unwrap();
        let cli = parse_slice(&["--batch-file", &dir.to_string_lossy()]).unwrap();
        assert_eq!(cli.urls, ["https://one", "https://two"]);
        let _ = std::fs::remove_file(&dir);
    }
}
