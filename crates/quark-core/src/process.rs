use std::path::PathBuf;
use std::process::ExitStatus;

pub fn exit_code(status: Option<ExitStatus>, fallback: i32) -> i32 {
    match status {
        Some(s) if s.success() => 0,
        Some(s) => s.code().unwrap_or(fallback),
        None => fallback,
    }
}

pub fn which(name: &str) -> Option<PathBuf> {
    quark_platform::which(name)
}

#[cfg(windows)]
pub use quark_platform::{HiddenProcess, read_handle_lines, spawn_cmd_start_wait};
