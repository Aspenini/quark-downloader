// The clap definition. `build.rs` `include!`s this file to generate shell
// completions and the man page, so it must stay self-contained (only std and
// clap imports) and use plain `//` comments (inner `//!` doc comments cannot
// survive `include!`).

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "quark-downloader",
    version,
    about = "Interactive yt-dlp wrapper. Run with no arguments for the interactive prompt."
)]
pub struct Cli {
    /// Video or playlist URLs to download.
    #[arg(value_name = "URL")]
    pub urls: Vec<String>,

    /// Additional URL (repeatable; same as passing URLs positionally).
    #[arg(long = "url", value_name = "URL")]
    pub url_flags: Vec<String>,

    /// File with one URL per line (# comments ignored).
    #[arg(long = "batch-file", value_name = "FILE")]
    pub batch_file: Option<std::path::PathBuf>,

    /// audio or video (default: video).
    #[arg(long = "type", value_name = "TYPE", default_value = "video")]
    pub media_type: String,

    /// Shorthand for --type audio.
    #[arg(long, conflicts_with = "media_type")]
    pub audio: bool,

    /// Output format (default: original).
    #[arg(long = "format", value_name = "FORMAT", default_value = "original")]
    pub format: String,

    /// Output directory.
    #[arg(long = "output-dir", value_name = "DIR")]
    pub output_dir: Option<std::path::PathBuf>,

    /// Show diagnostic detail (tool paths, the exact yt-dlp command).
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Do not wait for a key press before exiting (Windows).
    #[arg(long = "no-pause")]
    pub no_pause: bool,

    /// Print default output directory and exit.
    #[arg(long = "print-default-output-dir")]
    pub print_default_output_dir: bool,
}
