pub mod headless;

#[cfg(feature = "slint")]
pub mod slint;

// Native backends are scaffolded behind features; each will provide a
// `Renderer` implementation mapping QuarkGUI widgets to its toolkit.
#[cfg(all(target_os = "macos", feature = "native-cocoa"))]
pub mod cocoa;
#[cfg(all(target_os = "linux", feature = "native-gtk"))]
pub mod gtk;
#[cfg(all(windows, feature = "native-windows"))]
pub mod win32;
