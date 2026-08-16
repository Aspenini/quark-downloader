use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};

const UNSET: u8 = 0;
const ON: u8 = 1;
const OFF: u8 = 2;

static ENABLED: AtomicU8 = AtomicU8::new(UNSET);
static WINDOWS_VT_TRIED: AtomicU8 = AtomicU8::new(0);

pub fn enabled() -> bool {
    match ENABLED.load(Ordering::Relaxed) {
        ON => true,
        OFF => false,
        _ => {
            let value = detect_enabled();
            ENABLED.store(if value { ON } else { OFF }, Ordering::Relaxed);
            value
        }
    }
}

pub fn force(value: bool) {
    ENABLED.store(if value { ON } else { OFF }, Ordering::Relaxed);
}

pub fn reset() {
    ENABLED.store(UNSET, Ordering::Relaxed);
}

fn detect_enabled() -> bool {
    if std::env::var_os("FORCE_COLOR").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return true;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var_os("QUARK_GUI").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return false;
    }
    if std::env::var("TERM").ok().as_deref() == Some("dumb") {
        return false;
    }
    if !std::io::stdout().is_terminal() {
        return false;
    }
    #[cfg(windows)]
    crate::sys::windows::enable_virtual_terminal(&WINDOWS_VT_TRIED);
    #[cfg(not(windows))]
    let _ = &WINDOWS_VT_TRIED;
    true
}

fn wrap(code: &str, text: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn bold(text: &str) -> String {
    wrap("1", text)
}

pub fn dim(text: &str) -> String {
    wrap("2", text)
}

pub fn red(text: &str) -> String {
    wrap("31", text)
}

pub fn green(text: &str) -> String {
    wrap("32", text)
}

pub fn yellow(text: &str) -> String {
    wrap("33", text)
}

pub fn cyan(text: &str) -> String {
    wrap("36", text)
}
