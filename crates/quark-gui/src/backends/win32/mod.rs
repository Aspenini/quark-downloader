//! Native Windows (Win32) backend: form, progress, and message dialogs built
//! directly on the Win32 API via `windows-sys`. No Slint delegation.

mod form;
mod progress;
mod util;

use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
};

use crate::backend::Renderer;
use crate::event::ProgressChannel;
use crate::model::{FormOutcome, FormSpec, MessageKind, ProgressSpec};

pub struct Win32Renderer;

impl Win32Renderer {
    pub fn new() -> Self {
        util::init_common_controls();
        Win32Renderer
    }
}

impl Default for Win32Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for Win32Renderer {
    fn run_form(&self, spec: FormSpec) -> FormOutcome {
        form::run_form(spec)
    }

    fn run_progress(&self, spec: ProgressSpec, channel: ProgressChannel) -> i32 {
        progress::run_progress(spec, channel)
    }

    fn message(&self, kind: MessageKind, title: &str, body: &str) {
        let icon = match kind {
            MessageKind::Error => MB_ICONERROR,
            MessageKind::Info => MB_ICONINFORMATION,
        };
        let title = util::wide(title);
        let body = util::wide(body);
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                body.as_ptr(),
                title.as_ptr(),
                MB_OK | icon,
            );
        }
    }

    fn name(&self) -> &'static str {
        "win32"
    }
}
