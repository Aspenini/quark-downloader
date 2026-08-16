use std::io;
use std::process::Command;

unsafe extern "C" {
    fn getuid() -> u32;
}

pub fn uid() -> u32 {
    // Safety: getuid is always available on Unix and has no preconditions.
    unsafe { getuid() }
}

pub fn is_root() -> bool {
    uid() == 0
}

pub fn local_offset_secs() -> i64 {
    unsafe extern "C" {
        fn time(t: *mut i64) -> i64;
        fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    }
    #[repr(C)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
        tm_gmtoff: i64,
        tm_zone: *const i8,
    }
    unsafe {
        let mut t = 0i64;
        time(&mut t);
        let mut tm = std::mem::zeroed::<Tm>();
        if localtime_r(&t, &mut tm).is_null() {
            0
        } else {
            tm.tm_gmtoff
        }
    }
}

pub fn fetch_body(url: &str, user_agent: &str) -> io::Result<String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--max-redirs", "5", "-A", user_agent, url])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!("curl failed ({})", output.status)));
    }
    if output.stdout.is_empty() {
        return Err(io::Error::other(format!("Empty response from {url}")));
    }
    String::from_utf8(output.stdout).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn download_file(url: &str, dest: &std::path::Path, user_agent: &str) -> io::Result<()> {
    let part = dest.with_file_name(format!(
        "{}.part",
        dest.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("download")
    ));
    let _ = std::fs::remove_file(&part);
    let _ = std::fs::remove_file(dest);
    let status = Command::new("curl")
        .args([
            "-fL",
            "--max-redirs",
            "5",
            "-A",
            user_agent,
            "-o",
            &part.to_string_lossy(),
            url,
        ])
        .status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(io::Error::other(format!("curl download failed ({status})")));
    }
    std::fs::rename(&part, dest).or_else(|_| {
        std::fs::copy(&part, dest)?;
        std::fs::remove_file(&part)
    })?;
    Ok(())
}
