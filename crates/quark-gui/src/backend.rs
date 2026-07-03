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

    /// The backend's config-style name, e.g. `"win32"` or `"cocoa"`.
    fn name(&self) -> &'static str;
}

/// The available toolkits. Mirrors the config `gui_backend` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Pick the default native backend for the platform.
    Auto,
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
    /// Parse a config-style name (e.g. `"win32"`, `"cocoa"`); unknown names
    /// yield [`Backend::Auto`].
    pub fn from_name(name: &str) -> Backend {
        match name.trim().to_ascii_lowercase().as_str() {
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
            Backend::Win32 => cfg!(all(windows, feature = "native-windows")),
            Backend::Cocoa => cfg!(all(target_os = "macos", feature = "native-cocoa")),
            Backend::Gtk => cfg!(all(
                any(target_os = "macos", target_os = "linux"),
                feature = "native-gtk"
            )),
            Backend::Kirigami => cfg!(feature = "native-kirigami"),
        }
    }

    /// The native backend used when no explicit backend is configured.
    pub fn platform_default() -> Backend {
        #[cfg(windows)]
        {
            Backend::Win32
        }
        #[cfg(target_os = "macos")]
        {
            Backend::Cocoa
        }
        #[cfg(target_os = "linux")]
        {
            Backend::Gtk
        }
        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        {
            Backend::Headless
        }
    }
}

/// Construct a renderer for `preferred`, falling back to the platform's native
/// default and then another compiled native backend when unavailable.
pub fn renderer(preferred: Backend) -> (Box<dyn Renderer>, Backend) {
    let requested = if preferred == Backend::Auto {
        Backend::platform_default()
    } else {
        preferred
    };
    if let Some(renderer) = renderer_for(requested) {
        return (renderer, requested);
    }

    for fallback in platform_backends() {
        if *fallback == requested {
            continue;
        }
        if let Some(renderer) = renderer_for(*fallback) {
            return (renderer, *fallback);
        }
    }

    (
        Box::new(crate::backends::headless::HeadlessRenderer),
        Backend::Headless,
    )
}

fn renderer_for(backend: Backend) -> Option<Box<dyn Renderer>> {
    match backend {
        Backend::Headless => {
            return Some(Box::new(crate::backends::headless::HeadlessRenderer));
        }
        #[cfg(all(target_os = "macos", feature = "native-cocoa"))]
        Backend::Cocoa => {
            return Some(Box::new(crate::backends::cocoa::CocoaRenderer::new()));
        }
        #[cfg(all(windows, feature = "native-windows"))]
        Backend::Win32 => {
            return Some(Box::new(crate::backends::win32::Win32Renderer::new()));
        }
        #[cfg(all(any(target_os = "macos", target_os = "linux"), feature = "native-gtk"))]
        Backend::Gtk => {
            return Some(Box::new(crate::backends::gtk::GtkRenderer::new()));
        }
        #[cfg(feature = "native-kirigami")]
        Backend::Kirigami => {
            return Some(Box::new(crate::backends::kirigami::QtRenderer::new()));
        }
        _ => {}
    }
    None
}

fn platform_backends() -> &'static [Backend] {
    #[cfg(windows)]
    {
        &[Backend::Win32, Backend::Kirigami]
    }
    #[cfg(target_os = "macos")]
    {
        &[Backend::Cocoa, Backend::Kirigami, Backend::Gtk]
    }
    #[cfg(target_os = "linux")]
    {
        &[Backend::Gtk, Backend::Kirigami]
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        &[]
    }
}

#[cfg(all(
    test,
    any(
        all(windows, feature = "native-windows"),
        all(target_os = "macos", feature = "native-cocoa"),
        all(target_os = "linux", feature = "native-gtk")
    )
))]
mod tests {
    use super::*;

    #[cfg(all(windows, feature = "native-windows"))]
    #[test]
    fn auto_uses_win32_on_windows() {
        assert_eq!(renderer(Backend::Auto).1, Backend::Win32);
    }

    #[cfg(all(target_os = "macos", feature = "native-cocoa"))]
    #[test]
    fn auto_uses_cocoa_on_macos() {
        assert_eq!(renderer(Backend::Auto).1, Backend::Cocoa);
    }

    #[cfg(all(target_os = "linux", feature = "native-gtk"))]
    #[test]
    fn auto_uses_gtk_on_linux() {
        assert_eq!(renderer(Backend::Auto).1, Backend::Gtk);
    }
}
