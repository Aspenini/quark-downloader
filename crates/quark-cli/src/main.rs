mod args;

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

    let format_choices = if media_type == "audio" {
        quark_core::Format::choices(quark_core::MediaType::Audio)
    } else {
        quark_core::Format::choices(quark_core::MediaType::Video)
    };
    let format = prompt_choice("Choose format", format_choices, Some("original"));
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

fn main() -> ExitCode {
    #[cfg(unix)]
    {
        ctrl_c_cancel();
    }

    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.is_empty() {
        return exit(interactive_main());
    }

    let cli = match args::parse(argv) {
        Ok(c) => c,
        Err(msg) => return abort(&msg),
    };
    if cli.help {
        println!("{}", args::HELP);
        return ExitCode::SUCCESS;
    }
    if cli.print_default_dir {
        println!("{}", download::default_output_dir().display());
        return ExitCode::SUCCESS;
    }
    if cli.urls.is_empty() {
        exit(interactive_main())
    } else {
        exit(download::run_all(
            &cli.urls,
            &cli.media_type,
            &cli.format,
            cli.output_dir.as_deref(),
            cli.no_pause,
            cli.emit_result,
        ))
    }
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
        quark_core::process::request_interrupt();
    }
    unsafe {
        signal(SIGINT, handler as *const () as usize);
    }
}
