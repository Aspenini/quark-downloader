//! Kirigami frontend. Visual UI is system Qt 6 + Kirigami QML, linked
//! into quark-downloader-gui when Qt is present at build time.

pub fn available() -> bool {
    cfg!(kirigami_ui)
}

pub fn invoke(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("--script") => {
            quark_gui::assert_frontend_binds(|event| {
                let _ = event;
            });
            quark_gui::run_script_stdio()
        }
        Some("-h") | Some("--help") => {
            println!("Usage: --session|--progress|--message|--script");
            0
        }
        _ => run_ui(args),
    }
}

fn run_ui(args: &[String]) -> i32 {
    #[cfg(kirigami_ui)]
    {
        return run_embedded(args);
    }
    #[cfg(not(kirigami_ui))]
    {
        let _ = args;
        eprintln!(
            "Kirigami UI was not compiled into this binary.\nInstall Qt 6 + Kirigami (qt6-declarative, kirigami) and rebuild."
        );
        1
    }
}

#[cfg(kirigami_ui)]
fn run_embedded(args: &[String]) -> i32 {
    use std::ffi::CString;
    use std::os::raw::c_char;

    set_qml_env();

    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "quark-downloader-gui".into());

    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push(CString::new(exe).unwrap_or_default());
    for a in args {
        argv.push(CString::new(a.as_str()).unwrap_or_default());
    }
    let mut ptrs: Vec<*mut c_char> = argv.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    unsafe { kirigami_ui_run(ptrs.len() as i32, ptrs.as_mut_ptr()) }
}

#[cfg(kirigami_ui)]
fn set_qml_env() {
    if std::env::var_os("QUARK_KIRIGAMI_QML").is_some() {
        return;
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let qml = dir.join("qml");
        if qml.is_dir() {
            // Safety: called once before QGuiApplication; no other threads yet.
            unsafe {
                std::env::set_var("QUARK_KIRIGAMI_QML", qml);
            }
        }
    }
}

#[cfg(kirigami_ui)]
unsafe extern "C" {
    fn kirigami_ui_run(argc: i32, argv: *mut *mut std::os::raw::c_char) -> i32;
}
