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

    /// The backend's config-style name, e.g. `"slint"` or `"cocoa"`.
    fn name(&self) -> &'static str;
}

/// The available toolkits. Mirrors the config `gui_backend` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Pick the best native backend for the platform, else Slint.
    Auto,
    /// Slint (feature `slint`, on by default): pure Rust, all platforms.
    Slint,
    /// Native Win32 (feature `native-windows`, Windows only).
    Win32,
    /// Native AppKit (feature `native-cocoa`, macOS only).
    Cocoa,
    /// GTK 4 (feature `native-gtk`; needs the GTK 4 system libraries).
    Gtk,
    /// Qt Widgets (feature `native-kirigami`; needs Qt 6).
    Kirigami,
    /// Non-graphical: accepts defaults. For tests and headless reuse.
    Headless,
}

impl Backend {
    /// Parse a config-style name (e.g. `"slint"`, `"cocoa"`); unknown names
    /// yield [`Backend::Auto`].
    pub fn from_name(name: &str) -> Backend {
        match name.trim().to_ascii_lowercase().as_str() {
            "slint" => Backend::Slint,
            // "winui" is a legacy alias for the native Windows backend.
            "win32" | "winui" => Backend::Win32,
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
            Backend::Win32 => cfg!(all(windows, feature = "native-windows")),
            Backend::Cocoa => cfg!(all(target_os = "macos", feature = "native-cocoa")),
            // GTK4 and Qt are cross-platform; compiling their feature requires
            // the system libraries, so the feature flag alone gates them.
            Backend::Gtk => cfg!(feature = "native-gtk"),
            Backend::Kirigami => cfg!(feature = "native-kirigami"),
        }
    }
}

/// Construct a renderer for `preferred`, falling back to Slint (then Headless)
/// when the requested backend is unavailable. Returns the renderer and the
/// backend actually chosen so callers can report a fallback.
pub fn renderer(preferred: Backend) -> (Box<dyn Renderer>, Backend) {
    match preferred {
        Backend::Headless => {
            return (
                Box::new(crate::backends::headless::HeadlessRenderer),
                Backend::Headless,
            );
        }
        #[cfg(all(target_os = "macos", feature = "native-cocoa"))]
        Backend::Cocoa => {
            return (
                Box::new(crate::backends::cocoa::CocoaRenderer::new()),
                Backend::Cocoa,
            );
        }
        #[cfg(all(windows, feature = "native-windows"))]
        Backend::Win32 => {
            return (
                Box::new(crate::backends::win32::Win32Renderer::new()),
                Backend::Win32,
            );
        }
        #[cfg(feature = "native-gtk")]
        Backend::Gtk => {
            return (
                Box::new(crate::backends::gtk::GtkRenderer::new()),
                Backend::Gtk,
            );
        }
        #[cfg(feature = "native-kirigami")]
        Backend::Kirigami => {
            return (
                Box::new(crate::backends::kirigami::QtRenderer::new()),
                Backend::Kirigami,
            );
        }
        // Anything else (or an unavailable native backend) falls to Slint.
        _ => {}
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
