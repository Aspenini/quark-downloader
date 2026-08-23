use std::io;
use std::path::Path;

use crate::version::VERSION;

pub fn user_agent() -> String {
    format!("quark-downloader/{VERSION}")
}

pub fn fetch_body(url: &str) -> io::Result<String> {
    quark_platform::fetch_body(url, &user_agent())
}

pub fn download_file(url: &str, dest: &Path) -> io::Result<()> {
    quark_platform::download_file(url, dest, &user_agent())
}

#[derive(Debug)]
pub struct FetchError(pub String);

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FetchError {}
