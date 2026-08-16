use std::io::{self, Read};
use std::process::{Command, Stdio};

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

pub fn read_pipe(mut reader: impl Read, mut on_line: impl FnMut(&str)) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let mut line = buf.drain(..=pos).collect::<Vec<_>>();
                    if line.last() == Some(&b'\n') {
                        line.pop();
                    }
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }
                    let text = String::from_utf8_lossy(&line);
                    on_line(&text);
                }
            }
            Err(_) => break,
        }
    }
    if !buf.is_empty() {
        let text = String::from_utf8_lossy(&buf);
        if !text.is_empty() {
            on_line(text.trim_end_matches(['\r', '\n']));
        }
    }
}

pub fn command_exists(name: &str) -> Option<std::path::PathBuf> {
    which(name)
}

pub fn which(name: &str) -> Option<std::path::PathBuf> {
    Command::new("sh")
        .args(["-c", &format!("command -v {}", shell_escape(name))])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if path.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(path))
                }
            } else {
                None
            }
        })
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
