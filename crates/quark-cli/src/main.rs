//! `quark-downloader` — interactive in a terminal, or scriptable with flags.

mod sink;

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use quark_core::config;
use quark_core::download::command::MediaType;
use quark_core::events::{EventSink, MultiSink, ProgressEvent};
use quark_core::logs::FileSink;
use quark_core::{paths, version, CancelToken, DownloadRequest};

use sink::CliSink;

#[derive(Parser)]
#[command(
    name = "quark-downloader",
    version,
    about = "Interactive yt-dlp wrapper. Run with no options for the interactive prompt."
)]
struct Cli {
    /// Video or playlist URL to download (repeatable).
    #[arg(long = "url", value_name = "URL")]
    urls: Vec<String>,

    /// File with one URL per line (# comments ignored).
    #[arg(long = "batch-file", value_name = "FILE")]
    batch_file: Option<PathBuf>,

    /// audio or video (default: video).
    #[arg(long = "type", value_name = "TYPE", default_value = "video")]
    media_type: String,

    /// Output format (default: original).
    #[arg(long = "format", value_name = "FORMAT", default_value = "original")]
    format: String,

    /// Output directory.
    #[arg(long = "output-dir", value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Do not wait for a key press before exiting (Windows).
    #[arg(long = "no-pause")]
    no_pause: bool,

    /// Print default output directory and exit.
    #[arg(long = "print-default-output-dir")]
    print_default_output_dir: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.print_default_output_dir {
        let settings = config::load(true).unwrap_or_default();
        let dir = settings.resolved_download_dir(&paths::default_downloads_dir());
        println!("{}", dir.display());
        return;
    }

    let mut urls = cli.urls;
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
        interactive_main(cli.no_pause)
    } else {
        let settings = config::load(true).unwrap_or_default();
        let media_type = MediaType::parse(&cli.media_type).unwrap_or(MediaType::Video);
        run(
            &settings,
            urls,
            media_type,
            cli.format,
            cli.output_dir,
            cli.no_pause,
        )
    };
    std::process::exit(exit_code);
}

fn interactive_main(no_pause: bool) -> i32 {
    let settings = config::load(false).unwrap_or_default();
    println!("{}", version::window_title());
    println!("----------------");

    let url = prompt_nonempty("Enter Video URL: ", None);
    let media = prompt_choice(
        "Download audio or video?",
        &["audio", "video"],
        Some("video"),
    );
    let media_type = MediaType::parse(&media).unwrap_or(MediaType::Video);

    let default_path = settings.resolved_download_dir(&paths::default_downloads_dir());
    let output_dir = prompt_nonempty(
        "Enter output directory",
        Some(&default_path.to_string_lossy()),
    );

    let format = match media_type {
        MediaType::Audio => {
            println!("\nAudio formats:");
            println!("  original / default.original");
            println!("  mp3, m4a, flac, wav, opus, vorbis");
            prompt_default("Choose audio format", "original")
        }
        MediaType::Video => {
            println!("\nVideo formats:");
            println!("  original / default.original");
            println!("  mp4, mkv, webm");
            prompt_default("Choose video format", "original")
        }
    };

    run(
        &settings,
        vec![url],
        media_type,
        format,
        Some(PathBuf::from(output_dir)),
        no_pause,
    )
}

fn run(
    settings: &quark_core::Settings,
    urls: Vec<String>,
    media_type: MediaType,
    format: String,
    output_dir: Option<PathBuf>,
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
    let cli_sink = Arc::new(CliSink::new(is_tty));

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

fn read_line() -> String {
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return String::new();
    }
    line.trim().to_string()
}

fn prompt_nonempty(prompt: &str, default: Option<&str>) -> String {
    loop {
        match default {
            Some(d) => print!("{prompt} [default: {d}]: "),
            None => print!("{prompt}"),
        }
        let _ = std::io::stdout().flush();
        let value = read_line();
        if value.is_empty() {
            if let Some(d) = default {
                return d.to_string();
            }
            println!("Value cannot be empty.");
            continue;
        }
        return value;
    }
}

fn prompt_default(prompt: &str, default: &str) -> String {
    print!("{prompt} [default: {default}]: ");
    let _ = std::io::stdout().flush();
    let value = read_line();
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_ascii_lowercase()
    }
}

fn prompt_choice(prompt: &str, choices: &[&str], default: Option<&str>) -> String {
    let joined = choices.join("/");
    loop {
        match default {
            Some(d) => print!("{prompt} ({joined}) [default: {d}]: "),
            None => print!("{prompt} ({joined}): "),
        }
        let _ = std::io::stdout().flush();
        let value = read_line();
        if value.is_empty() {
            if let Some(d) = default {
                return d.to_string();
            }
        }
        let lower = value.to_ascii_lowercase();
        if let Some(found) = choices.iter().find(|c| c.eq_ignore_ascii_case(&lower)) {
            return (*found).to_string();
        }
        println!("Invalid choice. Try again.");
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
