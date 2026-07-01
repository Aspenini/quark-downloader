//! Best-effort launch of the CLI in a visible terminal (external_cli mode).

use std::path::PathBuf;
use std::process::Command;

use crate::download::DownloadParams;

fn resolve_cli() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(windows) {
        "quark-downloader.exe"
    } else {
        "quark-downloader"
    };
    if let Some(dir) = exe.parent() {
        let beside = dir.join(name);
        if beside.exists() {
            return Some(beside);
        }
    }
    quark_core::util::find_executable("quark-downloader")
}

fn cli_args(params: &DownloadParams) -> Vec<String> {
    let mut args = Vec::new();
    for url in &params.urls {
        args.push("--url".into());
        args.push(url.clone());
    }
    args.push("--type".into());
    args.push(params.media_type.as_str().into());
    args.push("--format".into());
    args.push(params.format.clone());
    args.push("--output-dir".into());
    args.push(params.output_dir.clone());
    args
}

pub fn launch_cli(params: &DownloadParams) -> std::io::Result<()> {
    let cli = resolve_cli().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "quark-downloader CLI not found",
        )
    })?;
    let args = cli_args(params);

    #[cfg(target_os = "macos")]
    {
        // Build a shell command and ask Terminal to run it.
        let mut command = shell_quote(&cli.to_string_lossy());
        for a in &args {
            command.push(' ');
            command.push_str(&shell_quote(a));
        }
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"",
            command.replace('"', "\\\"")
        );
        Command::new("osascript").arg("-e").arg(script).spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        for term in ["x-terminal-emulator", "gnome-terminal", "konsole", "xterm"] {
            if quark_core::util::find_executable(term).is_some() {
                let mut cmd = Command::new(term);
                cmd.arg("-e").arg(&cli).args(&args);
                if cmd.spawn().is_ok() {
                    return Ok(());
                }
            }
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no terminal emulator found",
        ));
    }

    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd");
        cmd.arg("/c").arg("start").arg("").arg(&cli).args(&args);
        cmd.spawn()?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(target_os = "macos")]
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
