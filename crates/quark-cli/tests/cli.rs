use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quark-downloader"))
}

#[test]
fn help_exits_zero() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage: quark-downloader"));
    assert!(stdout.contains("--emit-result-json"));
}

#[test]
fn print_default_output_dir() {
    let out = bin().arg("--print-default-output-dir").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.trim().is_empty());
}

#[test]
fn unknown_option_fails() {
    let out = bin().arg("--not-a-flag").output().unwrap();
    assert!(!out.status.success());
}

#[test]
fn missing_batch_file_fails() {
    let out = bin()
        .arg("--batch-file")
        .arg("no-such-batch.txt")
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn invalid_url_exits_nonzero_without_hanging() {
    let dir = std::env::temp_dir().join(format!(
        "quark-cli-smoke-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let out = bin()
        .args([
            "--url",
            "not-a-url",
            "--type",
            "video",
            "--format",
            "original",
            "--output-dir",
            &dir.to_string_lossy(),
            "--no-pause",
            "--emit-result-json",
        ])
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!out.status.success());
}
