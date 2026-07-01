//! Manual check for the native macOS backend:
//!   cargo run -p quark-gui --example cocoa_alert --features native-cocoa
//! Shows a native NSAlert. Not part of the automated tests (it blocks on a
//! modal dialog).

use quark_gui::model::MessageKind;
use quark_gui::{App, Backend};

fn main() {
    let app = App::new(Backend::Cocoa);
    println!("backend: {:?}", app.backend());
    app.message(MessageKind::Info, "QuarkGUI", "Native Cocoa alert works.");
    println!("dismissed");
}
