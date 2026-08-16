//! Shared GUI protocol, catalog, and session reducer.
//!
//! Frontends bind widgets to [`reduce`]; they must not emit session JSON by hand.

pub mod catalog;
pub mod copy;
pub mod event;
pub mod ffi;
pub mod reduce;
pub mod script;

pub use catalog::{
    ALL_FRONTEND_IDS, AUDIO_FORMATS, MODES, SPACES, THEMES, TOOL_SOURCES, VIDEO_FORMATS,
    supported_frontends,
};
pub use copy::{ERR_EMPTY_DOWNLOAD_DIR, ERR_EMPTY_OUTPUT, ERR_EMPTY_QUEUE};
pub use event::{REQUIRED_ACTIONS, UiEffect, UiEvent, View};
pub use reduce::{UiState, reduce};
pub use script::{ScriptOutput, run as run_script};

/// Headless `--script` entry used by every frontend binary.
pub fn run_script_stdio() -> i32 {
    use std::io::{self, Read, Write};
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        let _ = writeln!(io::stderr(), "{e}");
        return 2;
    }
    match script::run(&input) {
        Ok(out) => {
            println!("{}", out.to_json());
            let _ = io::stdout().flush();
            out.exit_code()
        }
        Err(e) => {
            let _ = writeln!(io::stderr(), "{e}");
            2
        }
    }
}

/// Bindings must mention every required action so a missing control fails to compile.
pub fn assert_frontend_binds(bind: impl Fn(UiEvent)) {
    for make in REQUIRED_ACTIONS {
        bind(make());
    }
}
