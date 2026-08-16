use std::io;
use std::path::Path;

use crate::version::VERSION;

pub fn user_agent() -> String {
    format!("quark-downloader/{VERSION}")
}

pub fn fetch_body(url: &str) -> io::Result<String> {
    let ua = user_agent();
    #[cfg(windows)]
    {
        crate::sys::windows::fetch_body(url, &ua)
    }
    #[cfg(not(windows))]
    {
        crate::sys::unix::fetch_body(url, &ua)
    }
}

pub fn download_file(url: &str, dest: &Path) -> io::Result<()> {
    let ua = user_agent();
    #[cfg(windows)]
    {
        crate::sys::windows::download_file(url, dest, &ua)
    }
    #[cfg(not(windows))]
    {
        crate::sys::unix::download_file(url, dest, &ua)
    }
}

#[derive(Debug)]
pub struct FetchError(pub String);

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FetchError {}
