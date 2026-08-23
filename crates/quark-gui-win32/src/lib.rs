//! In-process Win32 frontend.

#[cfg(windows)]
mod win32;

#[cfg(windows)]
pub use win32::{collect_main_session, confirm_open_url, message_box, run_progress};

pub fn run_script() -> i32 {
    quark_gui::assert_frontend_binds(|event| {
        let _ = event;
    });
    quark_gui::run_script_stdio()
}

#[cfg(not(windows))]
pub fn collect_main_session(
    _default_output: &str,
    _settings: &quark_core::config::Settings,
) -> Result<quark_core::session::MainSessionResult, String> {
    Err("Win32 frontend is only supported on Windows".into())
}

#[cfg(not(windows))]
pub fn message_box(_message: &str, _error: bool) {}

#[cfg(not(windows))]
pub fn confirm_open_url(_message: &str, _url: &str) {}

#[cfg(not(windows))]
pub fn run_progress(_command: &str, _cmd_args: &[String]) -> i32 {
    1
}
