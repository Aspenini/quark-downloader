use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use quark_core::color;
use quark_core::config;
use quark_core::download;
use quark_core::version;

fn prompt_choice(prompt: &str, choices: &[&str], default: Option<&str>) -> String {
    let choices_lower: Vec<String> = choices.iter().map(|c| c.to_ascii_lowercase()).collect();
    loop {
        if let Some(default) = default {
            print!(
                "{} ({}) [{}]: ",
                color::bold(prompt),
                choices.join("/"),
                color::dim(&format!("default: {default}"))
            );
        } else {
            print!("{} ({}): ", color::bold(prompt), choices.join("/"));
        }
        let _ = io::stdout().flush();
        let mut value = String::new();
        if io::stdin().read_line(&mut value).is_err() {
            value.clear();
        }
        let value = value.trim();
        if value.is_empty() {
            if let Some(default) = default {
                return default.to_string();
            }
            println!("{}", color::red("Value cannot be empty."));
            continue;
        }
        let lower = value.to_ascii_lowercase();
        if let Some(idx) = choices_lower.iter().position(|c| c == &lower) {
            return choices[idx].to_string();
        }
        println!("{}", color::red("Invalid choice. Try again."));
    }
}

fn prompt_nonempty(prompt: &str, default: Option<&str>) -> String {
    loop {
        if let Some(default) = default {
            print!(
                "{} [{}]: ",
                color::bold(prompt),
                color::dim(&format!("default: {default}"))
            );
        } else {
            print!("{}: ", color::bold(prompt));
        }
        let _ = io::stdout().flush();
        let mut value = String::new();
        if io::stdin().read_line(&mut value).is_err() {
            value.clear();
        }
        let value = value.trim();
        if value.is_empty() {
            if let Some(default) = default {
                return default.to_string();
            }
            println!("{}", color::red("Value cannot be empty."));
            continue;
        }
        return value.to_string();
    }
}

fn interactive_main() -> i32 {
    let _ = config::load(false);
    println!("{}", color::bold(&version::window_title()));
    println!("{}", color::dim(&"─".repeat(40)));
    println!();

    let url = prompt_nonempty("Video or playlist URL", None);
    println!();
    let media_type = prompt_choice(
        "Download audio or video?",
        &["audio", "video"],
        Some("video"),
    )
    .to_ascii_lowercase();
    println!();
    let default_path = {
        let settings = config::load(true).unwrap_or_default();
        settings
            .download_dir_expanded(&download::default_downloads_dir())
            .to_string_lossy()
            .into_owned()
    };
    let output_dir = prompt_nonempty("Output directory", Some(&default_path));
    println!();

    let format = if media_type == "audio" {
        println!("{}", color::bold("Audio formats"));
        println!("{}", color::dim("  original  (keep source audio)"));
        println!("{}", color::dim("  mp3, m4a, flac, wav, opus, vorbis"));
        print!("Choose format [{}]: ", color::dim("default: original"));
        let _ = io::stdout().flush();
        let mut value = String::new();
        let _ = io::stdin().read_line(&mut value);
        value.trim().to_ascii_lowercase()
    } else {
        println!("{}", color::bold("Video formats"));
        println!("{}", color::dim("  original  (keep source video)"));
        println!("{}", color::dim("  mp4, mkv, webm"));
        print!("Choose format [{}]: ", color::dim("default: original"));
        let _ = io::stdout().flush();
        let mut value = String::new();
        let _ = io::stdin().read_line(&mut value);
        value.trim().to_ascii_lowercase()
    };
    let format = if format.is_empty() {
        "original".into()
    } else {
        format
    };
    println!();
    download::run(
        &url,
        &media_type,
        &format,
        Some(&PathBuf::from(output_dir)),
        false,
        false,
    )
}

fn print_help() {
    println!(
        "Usage: quark-downloader [options]\n\nInteractive when run with no options.\n\n    --url URL                      Video or playlist URL to download (repeatable)\n    --batch-file FILE              File with one URL per line (# comments ignored)\n    --type TYPE                    audio or video (default: video)\n    --format FORMAT                Output format (default: original)\n    --output-dir DIR               Output directory\n    --no-pause                     Do not wait for a key press before exiting (Windows)\n    --print-default-output-dir     Print default output directory and exit\n    --emit-result-json             Print a final __RESULT__ JSON line for GUI/tools\n    -h, --help                     Show help"
    );
}

fn main() -> ExitCode {
    #[cfg(unix)]
    {
        ctrl_c_cancel();
    }

    let mut args = std::env::args().skip(1).peekable();
    if args.peek().is_none() {
        return exit(interactive_main());
    }

    let mut urls = Vec::new();
    let mut media_type = "video".to_string();
    let mut format = "original".to_string();
    let mut output_dir = None;
    let mut no_pause = false;
    let mut print_default_dir = false;
    let mut emit_result = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "--url" => match args.next() {
                Some(v) => urls.push(v),
                None => return abort("missing value for --url"),
            },
            "--batch-file" => match args.next() {
                Some(v) => {
                    if let Err(msg) = load_batch(&v, &mut urls) {
                        return abort(&msg);
                    }
                }
                None => return abort("missing value for --batch-file"),
            },
            "--type" => match args.next() {
                Some(v) => media_type = v,
                None => return abort("missing value for --type"),
            },
            "--format" => match args.next() {
                Some(v) => format = v,
                None => return abort("missing value for --format"),
            },
            "--output-dir" => match args.next() {
                Some(v) => output_dir = Some(PathBuf::from(v)),
                None => return abort("missing value for --output-dir"),
            },
            "--no-pause" => no_pause = true,
            "--print-default-output-dir" => print_default_dir = true,
            "--emit-result-json" => emit_result = true,
            other => return abort(&format!("unknown option: {other}")),
        }
    }

    if print_default_dir {
        println!("{}", download::default_output_dir().display());
        return ExitCode::SUCCESS;
    }

    if urls.is_empty() {
        exit(interactive_main())
    } else {
        exit(download::run_all(
            &urls,
            &media_type,
            &format,
            output_dir.as_deref(),
            no_pause,
            emit_result,
        ))
    }
}

fn load_batch(path: &str, urls: &mut Vec<String>) -> Result<(), String> {
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

fn abort(message: &str) -> ExitCode {
    eprintln!("{}", color::red(message));
    ExitCode::from(1)
}

fn exit(code: i32) -> ExitCode {
    ExitCode::from(code.clamp(0, 255) as u8)
}

#[cfg(unix)]
fn ctrl_c_cancel() {
    unsafe extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
    }
    const SIGINT: i32 = 2;
    extern "C" fn handler(_: i32) {
        println!("\n{}", color::yellow("Cancelled."));
        download::press_any_key(false, "Press any key to exit...");
        std::process::exit(130);
    }
    unsafe {
        signal(SIGINT, handler as usize);
    }
}
