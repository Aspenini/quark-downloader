#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::BufRead;
use std::process::{Command, Stdio};

use quark_core::config::{self, GuiDownloadMode, Settings};
#[cfg(not(windows))]
use quark_core::frontend;
use quark_core::frontend::{Frontend, HelperFrontend, MessageKind};
use quark_core::progress::{ProgressCmd, ProgressRelay};
use quark_core::release;
use quark_core::result::DownloadResult;
use quark_core::session::{self, DownloadParams, MainAction, SettingsForm};
use quark_core::version;

fn main() {
    // Safety: process startup, single-threaded, before any other env reads.
    unsafe {
        std::env::set_var("QUARK_VERSION", version::VERSION);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("-h") | Some("--help") => {
            println!(
                "Usage: quark-downloader-gui [--help] [--check-updates] [--frontend <id> ...]\n\nOpens a native frontend and runs quark-downloader."
            );
        }
        Some("--frontend") => {
            let id = args.get(1).map(String::as_str).unwrap_or("");
            let rest = if args.len() > 2 { &args[2..] } else { &[] };
            std::process::exit(run_frontend(id, rest));
        }
        Some("--check-updates") => run_update_check(),
        _ => run_controller(),
    }
}

fn builtin_frontends() -> Vec<&'static str> {
    #[cfg(target_os = "linux")]
    {
        quark_gui_qt::available()
            .then_some("qt")
            .into_iter()
            .collect()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

fn run_frontend(id: &str, args: &[String]) -> i32 {
    let _ = args;
    match id {
        #[cfg(target_os = "linux")]
        "qt" => quark_gui_qt::invoke(args),
        other => {
            eprintln!("frontend '{other}' is not compiled into this binary");
            1
        }
    }
}

fn run_controller() {
    let Some(cli) = session::resolve_cli() else {
        show_missing_cli();
        return;
    };

    loop {
        let settings = match config::load(true) {
            Ok(s) => s,
            Err(e) => {
                show_error(&e.0);
                return;
            }
        };
        let default_output = session::default_output_dir();
        let session = match collect_session(&settings, &default_output.to_string_lossy()) {
            Ok(s) => s,
            Err(e) => {
                show_error(&e);
                return;
            }
        };

        if let Some(form) = session.settings_form {
            if !save_settings(form) {
                continue;
            }
        }

        match session.action {
            MainAction::Download(params) => {
                run_download(&cli.to_string_lossy(), &params);
                continue;
            }
            MainAction::Cancel => return,
            MainAction::Error(message) => {
                show_error(&message);
                return;
            }
        }
    }
}

fn save_settings(form: SettingsForm) -> bool {
    if form.download_dir.trim().is_empty() {
        show_error("Please choose a default download folder.");
        return false;
    }
    if let Err(e) = config::save(&form.to_settings()) {
        show_error(&e.0);
        return false;
    }
    true
}

fn collect_session(
    settings: &Settings,
    default_output: &str,
) -> Result<session::MainSessionResult, String> {
    if settings.gui_frontend.uses_inprocess_win32() {
        #[cfg(windows)]
        {
            return windows::collect_main_session(default_output, settings);
        }
        #[cfg(not(windows))]
        {
            return Err("Win32 frontend is only available on Windows.".into());
        }
    }
    let frontend = HelperFrontend::discover(settings, &builtin_frontends()).map_err(|e| e.0)?;
    Ok(frontend.collect_session(default_output, settings))
}

fn show_missing_cli() {
    show_error("quark-downloader was not found.\nInstall it next to this program or on PATH.");
}

fn show_error(message: &str) {
    #[cfg(windows)]
    {
        windows::message_box(message, true);
    }
    #[cfg(not(windows))]
    frontend::last_resort_error(message);
}

fn show_info(message: &str) {
    if let Some(fe) = helper_if_available() {
        fe.show_message(MessageKind::Ok, version::APP_NAME, message);
        return;
    }
    #[cfg(windows)]
    {
        windows::message_box(message, false);
    }
    #[cfg(not(windows))]
    println!("{message}");
}

fn helper_if_available() -> Option<HelperFrontend> {
    let settings = config::load(true).ok()?;
    if settings.gui_frontend.uses_inprocess_win32() {
        return None;
    }
    HelperFrontend::discover(&settings, &builtin_frontends()).ok()
}

fn show_completion(result: &DownloadResult) {
    let open_output = config::load(true)
        .map(|s| s.open_output_dir)
        .unwrap_or(false);
    if result.success() && open_output {
        open_folder(&result.output_dir);
    }
    if let Some(fe) = helper_if_available() {
        fe.show_completion(result);
        return;
    }
    #[cfg(windows)]
    {
        let _ = result;
    }
}

fn open_folder(path: &str) {
    if path.trim().is_empty() {
        return;
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(path).status();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).status();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).status();
    }
}

fn run_download(cli: &str, params: &DownloadParams) -> i32 {
    let settings = config::load(true).unwrap_or_default();
    let args = session::build_cli_args(cli, params);
    let command = &args[0];
    let cmd_args = &args[1..];
    match settings.gui_download_mode {
        GuiDownloadMode::ExternalCli => run_download_external_cli(command, cmd_args),
        GuiDownloadMode::Progress => run_download_with_progress(&settings, command, cmd_args),
    }
}

fn run_download_with_progress(settings: &Settings, command: &str, cmd_args: &[String]) -> i32 {
    if settings.gui_frontend.uses_inprocess_win32() {
        #[cfg(windows)]
        {
            return windows::run_progress(command, cmd_args);
        }
    }
    run_download_with_progress_helper(settings, command, cmd_args)
}

fn run_download_with_progress_helper(
    settings: &Settings,
    command: &str,
    cmd_args: &[String],
) -> i32 {
    let frontend = match HelperFrontend::discover(settings, &builtin_frontends()) {
        Ok(f) => f,
        Err(e) => {
            show_error(&e.0);
            return 1;
        }
    };
    let mut progress = match frontend.open_progress(settings.gui_theme) {
        Ok(p) => p,
        Err(e) => {
            show_error(&e.0);
            return 1;
        }
    };
    let relay = ProgressRelay::new();
    let mut child = match Command::new(command)
        .args(cmd_args)
        .env("QUARK_GUI", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            show_error(&e.to_string());
            return 1;
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let spawn_relay = |pipe: Option<std::process::ChildStdout>,
                       tx: std::sync::mpsc::Sender<String>| {
        if let Some(pipe) = pipe {
            std::thread::spawn(move || {
                for line in std::io::BufReader::new(pipe).lines().map_while(Result::ok) {
                    let _ = tx.send(line);
                }
            });
        }
    };
    // stdout and stderr have different types; handle separately.
    if let Some(pipe) = stdout {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
    }
    if let Some(pipe) = stderr {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
    }
    drop(tx);
    let _ = spawn_relay;
    let mut result_holder: Option<DownloadResult> = None;
    let mut user_closed = false;
    loop {
        if progress.wait_closed() {
            user_closed = true;
            let _ = child.kill();
            break;
        }
        let line = match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(line) => line,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if let Some(parsed) = DownloadResult::parse_emit_line(&line) {
            result_holder = Some(parsed);
            continue;
        }
        let mut buf = Vec::new();
        let _ = relay.relay(&line, &mut buf);
        for encoded in String::from_utf8_lossy(&buf).lines() {
            let cmd = decode_progress_line(encoded);
            if let Some(cmd) = cmd
                && progress.send(&cmd).is_err()
            {
                user_closed = true;
                let _ = child.kill();
                break;
            }
        }
    }
    let status = child.wait().ok();
    let exit_code = quark_core::process::exit_code(status, 1);
    if user_closed {
        return 1;
    }
    let _ = progress.send(&ProgressCmd::Done(exit_code));
    while !progress.wait_closed() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let mut result = result_holder.unwrap_or(DownloadResult {
        exit_code,
        ..DownloadResult::default()
    });
    if result.exit_code == 0 && exit_code != 0 {
        result.exit_code = exit_code;
    }
    show_completion(&result);
    exit_code
}

fn decode_progress_line(line: &str) -> Option<ProgressCmd> {
    let (kind, rest) = line.split_once('\t').unwrap_or((line, ""));
    match kind {
        "PROGRESS" => rest.parse().ok().map(ProgressCmd::Progress),
        "STATUS" => Some(ProgressCmd::Status(rest.to_string())),
        "ETA" => Some(ProgressCmd::Eta(rest.to_string())),
        "QUEUE" => Some(ProgressCmd::Queue(rest.to_string())),
        "DONE" => rest.parse().ok().map(ProgressCmd::Done),
        _ => None,
    }
}

fn run_download_external_cli(command: &str, cmd_args: &[String]) -> i32 {
    #[cfg(windows)]
    {
        quark_core::process::spawn_cmd_start_wait("Quark Downloader", command, cmd_args)
    }
    #[cfg(target_os = "macos")]
    {
        run_in_macos_terminal(command, cmd_args)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some((path, prefix)) = find_terminal() {
            return run_in_terminal(&path, &prefix, command, cmd_args);
        }
        let status = Command::new(command).args(cmd_args).status().ok();
        quark_core::process::exit_code(status, 1)
    }
}

#[cfg(target_os = "macos")]
fn run_in_macos_terminal(command: &str, args: &[String]) -> i32 {
    let mut inner = std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .map(shell_escape)
        .collect::<Vec<_>>()
        .join(" ");
    inner.push_str("; echo; read -r -p 'Press Enter to close...' _");
    let status = Command::new("open")
        .args(["-a", "Terminal", command])
        .status()
        .ok();
    // `open -a Terminal file` does not pass argv. Use osascript for a real command line.
    let script = format!(
        "tell application \"Terminal\" to do script {}",
        shell_escape(&inner)
    );
    let _ = status;
    let status = Command::new("osascript")
        .args(["-e", &script])
        .status()
        .ok();
    quark_core::process::exit_code(status, 1)
}

#[cfg(not(windows))]
fn find_terminal() -> Option<(String, Vec<String>)> {
    const CANDIDATES: &[(&str, &[&str])] = &[
        ("x-terminal-emulator", &["-e"]),
        ("gnome-terminal", &["--wait", "--"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-e"]),
        ("tilix", &["-e"]),
        ("terminator", &["-x"]),
        ("kitty", &["-e"]),
        ("wezterm", &["start", "--"]),
        ("ghostty", &["-e"]),
        ("alacritty", &["-e"]),
        ("foot", &["-e"]),
        ("ptyxis", &["--", "sh", "-c"]),
        ("kgx", &["-e"]),
    ];
    for (name, prefix) in CANDIDATES {
        if let Some(path) = quark_core::process::which(name) {
            return Some((
                path.to_string_lossy().into_owned(),
                prefix.iter().map(|s| (*s).to_string()).collect(),
            ));
        }
    }
    None
}

#[cfg(not(windows))]
fn run_in_terminal(path: &str, prefix: &[String], command: &str, args: &[String]) -> i32 {
    let mut inner = std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .map(shell_escape)
        .collect::<Vec<_>>()
        .join(" ");
    inner.push_str("; echo; read -r -p 'Press Enter to close...' _");
    let mut cmd_args = prefix.to_vec();
    cmd_args.extend(["sh".into(), "-c".into(), inner]);
    let status = Command::new(path).args(cmd_args).status().ok();
    quark_core::process::exit_code(status, 1)
}

#[cfg(not(windows))]
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn run_update_check() {
    let (status, latest, behind, error) = release::check_with_error();
    match status {
        release::Status::UpToDate => {
            show_info(&format!("You are up to date ({}).", version::VERSION));
        }
        release::Status::Ahead => {
            show_info(&format!(
                "You are running {} (newer than the latest release {}).",
                version::VERSION,
                latest.unwrap_or_default()
            ));
        }
        release::Status::Behind => {
            if let Some(info) = behind {
                present_behind(&info);
            }
        }
        release::Status::Failed => {
            show_error(&format!(
                "Could not check for updates:\n{}",
                error.unwrap_or_else(|| "unknown error".into())
            ));
        }
    }
}

fn present_behind(info: &release::BehindInfo) {
    #[cfg(windows)]
    {
        let message = format!(
            "A newer version ({}) is available. You have {}.\n\nDownload the latest installer?",
            info.latest_version,
            version::VERSION
        );
        windows::confirm_open_url(&message, &info.download_url);
    }
    #[cfg(target_os = "macos")]
    {
        show_info(&format!(
            "A newer version ({}) is available. You have {}.\n\nDownload the latest release from github.com/Aspenini/quark-downloader/releases (or update with your package manager).",
            info.latest_version,
            version::VERSION
        ));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        show_info(&format!(
            "A newer version ({}) is available. You have {}.\n\nUpdate with your package manager (e.g. yay -Syu or the AUR).",
            info.latest_version,
            version::VERSION
        ));
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use quark_core::session::MainSessionResult;

    pub fn collect_main_session(
        default_output: &str,
        settings: &Settings,
    ) -> Result<MainSessionResult, String> {
        quark_gui_win32::collect_main_session(default_output, settings)
    }

    pub fn message_box(message: &str, error: bool) {
        quark_gui_win32::message_box(message, error);
    }

    pub fn confirm_open_url(message: &str, url: &str) {
        quark_gui_win32::confirm_open_url(message, url);
    }

    pub fn run_progress(command: &str, cmd_args: &[String]) -> i32 {
        quark_gui_win32::run_progress(command, cmd_args)
    }
}
