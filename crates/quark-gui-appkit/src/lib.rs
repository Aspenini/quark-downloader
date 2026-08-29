//! AppKit frontend for macOS. Visual UI is Objective-C compiled by build.rs.

pub fn available() -> bool {
    cfg!(appkit_ui)
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
    #[cfg(appkit_ui)]
    {
        run_embedded(args)
    }
    #[cfg(not(appkit_ui))]
    {
        let _ = args;
        eprintln!("AppKit UI was not compiled into this binary.");
        1
    }
}

#[cfg(appkit_ui)]
fn run_embedded(args: &[String]) -> i32 {
    use std::ffi::CString;
    use std::os::raw::c_char;

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
    unsafe { appkit_ui_run(ptrs.len() as i32, ptrs.as_mut_ptr()) }
}

#[cfg(appkit_ui)]
unsafe extern "C" {
    fn appkit_ui_run(argc: i32, argv: *mut *mut std::os::raw::c_char) -> i32;
}
