pub mod headless;

// Native backends behind features; each provides a `Renderer` implementation
// mapping QuarkGUI widgets to its toolkit. GTK and Qt are cross-platform and
// gated only by their feature (compiling them requires the system libraries).
#[cfg(all(target_os = "macos", feature = "native-cocoa"))]
pub mod cocoa;
#[cfg(all(any(target_os = "macos", target_os = "linux"), feature = "native-gtk"))]
pub mod gtk;
#[cfg(feature = "native-kirigami")]
pub mod kirigami;
#[cfg(all(windows, feature = "native-windows"))]
pub mod win32;
