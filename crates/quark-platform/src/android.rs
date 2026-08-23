//! Android has Bionic, no curl(1), and no user HOME layout.
//! Tool downloads and HTTP live in the Kotlin host for v1.

use std::io;
use std::path::Path;

pub fn is_root() -> bool {
    false
}

pub fn local_offset_secs() -> i64 {
    0
}

pub fn fetch_body(_url: &str, _user_agent: &str) -> io::Result<String> {
    Err(unsupported("HTTP fetch"))
}

pub fn download_file(_url: &str, _dest: &Path, _user_agent: &str) -> io::Result<()> {
    Err(unsupported("HTTP download"))
}

pub fn sha256_hex(_path: &Path) -> io::Result<String> {
    Err(unsupported("sha256"))
}

fn unsupported(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{what} is handled by the Android app host, not quark-platform"),
    )
}
