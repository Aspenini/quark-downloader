//! Backend selection. [`Renderer`] is the one trait every backend implements;
//! everything else in the crate is backend-independent.

use crate::event::ProgressChannel;
use crate::model::{FormOutcome, FormSpec, MessageKind, ProgressSpec};

/// Renders QuarkGUI views with a concrete toolkit. Implementations run their
/// toolkit's event loop internally and return when the view is dismissed.
pub trait Renderer {
    /// Show a form and block until the user submits, presses an extra button,
    /// or cancels.
    fn run_form(&self, spec: FormSpec) -> FormOutcome;

    /// Show a progress view, consuming updates from `channel` until a
    /// [`crate::event::ProgressUpdate::Done`] arrives or the window is closed.
    /// Returns the final exit code.
    fn run_progress(&self, spec: ProgressSpec, channel: ProgressChannel) -> i32;

    /// Show a modal message dialog.
    fn message(&self, kind: MessageKind, title: &str, body: &str);

    fn name(&self) -> &'static str;
}

/// The available toolkits. Mirrors the config `gui_backend` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Pick the best native backend for the platform, else Slint.
    Auto,
    Slint,
    Win32,
    WinUi,
    Cocoa,
    Gtk,
    Kirigami,
    /// Non-graphical: accepts defaults. For tests and headless reuse.
    Headless,
}

impl Backend {
    pub fn from_name(name: &str) -> Backend {
        match name.trim().to_ascii_lowercase().as_str() {
            "slint" => Backend::Slint,
            "win32" => Backend::Win32,
            "winui" => Backend::WinUi,
            "cocoa" => Backend::Cocoa,
            "gtk" => Backend::Gtk,
            "kirigami" => Backend::Kirigami,
            "headless" => Backend::Headless,
            _ => Backend::Auto,
        }
    }

    /// Whether this backend is compiled in and usable on this platform.
    pub fn available(self) -> bool {
        match self {
            Backend::Headless | Backend::Auto => true,
            Backend::Slint => cfg!(feature = "slint"),
            Backend::Win32 | Backend::WinUi => cfg!(all(windows, feature = "native-windows")),
            Backend::Cocoa => cfg!(all(target_os = "macos", feature = "native-cocoa")),
            Backend::Gtk => cfg!(all(target_os = "linux", feature = "native-gtk")),
            Backend::Kirigami => cfg!(all(target_os = "linux", feature = "native-kirigami")),
        }
    }
}

/// Construct a renderer for `preferred`, falling back to Slint (then Headless)
/// when the requested backend is unavailable. Returns the renderer and the
/// backend actually chosen so callers can report a fallback.
pub fn renderer(preferred: Backend) -> (Box<dyn Renderer>, Backend) {
    // Native backends are scaffolded behind features; none are wired yet, so
    // any non-Slint request currently resolves down the fallback chain.
    if preferred == Backend::Headless {
        return (
            Box::new(crate::backends::headless::HeadlessRenderer),
            Backend::Headless,
        );
    }

    #[cfg(feature = "slint")]
    {
        let _ = preferred;
        (
            Box::new(crate::backends::slint::SlintRenderer::new()),
            Backend::Slint,
        )
    }
    #[cfg(not(feature = "slint"))]
    {
        let _ = preferred;
        (
            Box::new(crate::backends::headless::HeadlessRenderer),
            Backend::Headless,
        )
    }
}
