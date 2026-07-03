//! A scripted progress window that drives itself to completion and exits —
//! handy for demoing `run_progress` and for smoke-testing a backend without
//! any interaction:
//!
//! ```text
//! QUARK_GUI_BACKEND=cocoa cargo run -p quark-gui --example progress \
//!     --features native-cocoa                                     # macOS
//! QUARK_GUI_BACKEND=gtk cargo run -p quark-gui --example progress \
//!     --no-default-features --features native-gtk                 # GTK 4
//! QUARK_GUI_BACKEND=kirigami cargo run -p quark-gui --example progress \
//!     --no-default-features --features native-kirigami            # Qt
//! ```

use std::thread;
use std::time::Duration;

use quark_gui::model::{ProgressSpec, Theme, WindowSpec};
use quark_gui::{App, Backend, ProgressChannel, ProgressUpdate};

fn main() {
    let backend = std::env::var("QUARK_GUI_BACKEND").unwrap_or_default();
    let app = App::new(Backend::from_name(&backend));
    println!("QuarkGUI backend in use: {:?}", app.backend());

    let (tx, rx) = crossbeam_channel::unbounded();
    let channel = ProgressChannel::new(rx);
    let cancel = channel.cancel_flag();

    let producer = thread::spawn(move || {
        let _ = tx.send(ProgressUpdate::Queue("Item 1 of 1".into()));
        for step in 0..=20 {
            if cancel.is_cancelled() {
                return;
            }
            let percent = f64::from(step) * 5.0;
            let _ = tx.send(ProgressUpdate::Percent(percent));
            let _ = tx.send(ProgressUpdate::Status(format!("Working... {percent:.0}%")));
            let _ = tx.send(ProgressUpdate::Eta(Some(format!("{}s", 20 - step))));
            thread::sleep(Duration::from_millis(100));
        }
        let _ = tx.send(ProgressUpdate::Status("Finished".into()));
        let _ = tx.send(ProgressUpdate::Done(0));
    });

    let spec = ProgressSpec {
        window: WindowSpec::new("QuarkGUI Progress Demo", Theme::Light),
        initial_status: "Starting...".into(),
    };
    let code = app.run_progress(spec, channel);
    producer.join().expect("producer thread");
    println!("exit code: {code}");
    std::process::exit(code);
}
