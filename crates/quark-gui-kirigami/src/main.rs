//! Kirigami frontend. `--script` is the shared reducer.
//! Visual UI is a tiny C++ helper dynamically linked to system Qt 6.

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
                "Usage: quark-downloader-gui-kirigami --session|--progress|--message|--script\n\nKirigami UI links system Qt 6; Kirigami QML comes from the distro."
            );
        }
        _ => std::process::exit(run_ui(&args)),
    }
}

fn run_ui(args: &[String]) -> i32 {
    let Some(ui) = find_ui_helper() else {
        eprintln!(
            "Kirigami UI helper was not built.\nInstall Qt 6 + Kirigami (e.g. apt install qt6-declarative-dev qml6-module-org-kde-kirigami) and rebuild."
        );
        return 1;
    };
    let status = std::process::Command::new(ui).args(args).status().ok();
    status.and_then(|s| s.code()).unwrap_or(1)
}

fn find_ui_helper() -> Option<std::path::PathBuf> {
    let name = if cfg!(windows) {
        "quark-downloader-gui-kirigami-ui.exe"
    } else {
        "quark-downloader-gui-kirigami-ui"
    };
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let sibling = parent.join(name);
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    for dir in ["build", "target/debug", "target/release"] {
        let p = std::path::PathBuf::from(dir).join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
