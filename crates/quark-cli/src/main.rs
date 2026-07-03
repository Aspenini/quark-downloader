//! `quark-downloader` — interactive in a terminal, or scriptable with flags.

mod cli;
mod sink;

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use cli::Cli;
use quark_core::config;
use quark_core::download::command::MediaType;
use quark_core::events::{EventSink, MultiSink, ProgressEvent};
use quark_core::logs::FileSink;
use quark_core::{paths, version, CancelToken, DownloadRequest};

use sink::CliSink;

const VIDEO_FORMATS: &[&str] = &["original", "mp4", "mkv", "webm"];
const AUDIO_FORMATS: &[&str] = &["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"];

fn main() {
    let cli = Cli::parse();

    if cli.print_default_output_dir {
        let settings = config::load(true).unwrap_or_default();
        let dir = settings.resolved_download_dir(&paths::default_downloads_dir());
        println!("{}", dir.display());
        return;
    }

    let mut urls = cli.urls;
    urls.extend(cli.url_flags);
    if let Some(batch) = &cli.batch_file {
        match std::fs::read_to_string(batch) {
            Ok(text) => {
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    urls.push(line.to_string());
                }
            }
            Err(_) => {
                eprintln!("Batch file not found: {}", batch.display());
                std::process::exit(1);
            }
        }
    }

    let exit_code = if urls.is_empty() {
        interactive_main(cli.verbose, cli.no_pause)
    } else {
        let settings = config::load(true).unwrap_or_default();
        let media_type = if cli.audio {
            MediaType::Audio
        } else {
            MediaType::parse(&cli.media_type).unwrap_or(MediaType::Video)
        };
        run(
            &settings,
            urls,
            media_type,
            cli.format,
            cli.output_dir,
            cli.verbose,
            cli.no_pause,
        )
    };
    std::process::exit(exit_code);
}

fn interactive_main(verbose: bool, no_pause: bool) -> i32 {
    let settings = config::load(false).unwrap_or_default();
    let title = version::window_title();
    println!("{}", style(&title, "1;36"));
    println!("{}", style(&"─".repeat(title.chars().count()), "2"));

    let mut urls = vec![prompt_nonempty("Video or playlist URL", None)];
    loop {
        let more = prompt_line("Another URL (blank to continue)", None);
        if more.is_empty() {
            break;
        }
        urls.push(more);
    }

    let media = prompt_choice("Download", &["video", "audio"], Some("video"));
    let media_type = MediaType::parse(&media).unwrap_or(MediaType::Video);

    let format = match media_type {
        MediaType::Audio => prompt_choice("Audio format", AUDIO_FORMATS, Some("original")),
        MediaType::Video => prompt_choice("Video format", VIDEO_FORMATS, Some("original")),
    };

    let default_path = settings.resolved_download_dir(&paths::default_downloads_dir());
    let output_dir = prompt_nonempty("Output folder", Some(&default_path.to_string_lossy()));

    println!();
    run(
        &settings,
        urls,
        media_type,
        format,
        Some(PathBuf::from(output_dir)),
        verbose,
        no_pause,
    )
}

#[allow(clippy::too_many_arguments)]
fn run(
    settings: &quark_core::Settings,
    urls: Vec<String>,
    media_type: MediaType,
    format: String,
    output_dir: Option<PathBuf>,
    verbose: bool,
    no_pause: bool,
) -> i32 {
    let cancel = CancelToken::new();
    {
        let cancel = cancel.clone();
        let _ = ctrlc::set_handler(move || {
            eprintln!("\nCancelled.");
            cancel.cancel();
        });
    }

    let is_tty = std::io::stderr().is_terminal();
    let cli_sink = Arc::new(CliSink::new(is_tty, verbose));

    // Compose the terminal sink with a rotated log file when enabled.
    let sink: Box<dyn EventSink> = if settings.download_logs {
        match FileSink::open() {
            Some(file) => Box::new(MultiSink::new(vec![
                Box::new(SharedCli(cli_sink.clone())),
                Box::new(file),
            ])),
            None => Box::new(SharedCli(cli_sink.clone())),
        }
    } else {
        Box::new(SharedCli(cli_sink.clone()))
    };

    let request = DownloadRequest {
        urls,
        media_type,
        format,
        output_dir,
        hidden_console: false,
    };

    let code = quark_core::run(&request, settings, sink.as_ref(), &cancel);
    cli_sink.finish();

    press_any_key(no_pause);
    if cancel.is_cancelled() {
        130
    } else {
        code
    }
}

/// Wraps the shared CLI sink so it can live in a `MultiSink`.
struct SharedCli(Arc<CliSink>);
impl EventSink for SharedCli {
    fn emit(&self, event: ProgressEvent) {
        self.0.emit(event);
    }
}

// ---- prompts -------------------------------------------------------------

/// Wrap `s` in an ANSI style when stdout is a terminal.
fn style(s: &str, code: &str) -> String {
    if std::io::stdout().is_terminal() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn read_line() -> String {
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return String::new();
    }
    line.trim().to_string()
}

/// Print a styled `label [hint]: ` prompt and read one line (may be empty).
fn prompt_line(label: &str, hint: Option<&str>) -> String {
    match hint {
        Some(h) => print!("{} {} ", style(label, "1"), style(&format!("[{h}]:"), "2")),
        None => print!("{} ", style(&format!("{label}:"), "1")),
    }
    let _ = std::io::stdout().flush();
    read_line()
}

fn prompt_nonempty(label: &str, default: Option<&str>) -> String {
    loop {
        let hint = default.map(|d| format!("default: {d}"));
        let value = prompt_line(label, hint.as_deref());
        if !value.is_empty() {
            return value;
        }
        if let Some(d) = default {
            return d.to_string();
        }
        println!("Value cannot be empty.");
    }
}

fn prompt_choice(label: &str, choices: &[&str], default: Option<&str>) -> String {
    let joined = choices.join("/");
    loop {
        let hint = match default {
            Some(d) => format!("{joined} · default: {d}"),
            None => joined.clone(),
        };
        let value = prompt_line(label, Some(&hint));
        if value.is_empty() {
            if let Some(d) = default {
                return d.to_string();
            }
        }
        if let Some(found) = choices.iter().find(|c| c.eq_ignore_ascii_case(&value)) {
            return (*found).to_string();
        }
        println!(
            "{}",
            style(&format!("Please choose one of: {joined}"), "33")
        );
    }
}

#[cfg(windows)]
fn press_any_key(no_pause: bool) {
    if no_pause {
        return;
    }
    println!("\nPress Enter to exit...");
    let _ = read_line();
}

#[cfg(not(windows))]
fn press_any_key(_no_pause: bool) {}
