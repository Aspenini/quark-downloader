//! `--script` runner that uses the shared reducer.
//! The visual AppKit UI remains the Swift helper compiled by scripts/unix/build.sh.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--script") => {
            quark_gui::assert_frontend_binds(|event| {
                let _ = event;
            });
            std::process::exit(quark_gui::run_script_stdio());
        }
        Some("-h") | Some("--help") => {
            println!(
                "Usage: quark-downloader-gui-appkit-script --script\n\nVisual UI: quark-downloader-gui-appkit (Swift)."
            );
        }
        _ => {
            eprintln!("usage: quark-downloader-gui-appkit-script --script");
            std::process::exit(2);
        }
    }
}
