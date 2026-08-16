use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quark-downloader-gui"))
}

#[test]
fn help_exits_zero() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("quark-downloader-gui") || stdout.contains("Usage"),
        "stdout was {stdout:?}"
    );
}

#[test]
fn unknown_frontend_exits_nonzero() {
    let out = bin().args(["--frontend", "nope"]).output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("nope") || err.contains("frontend"),
        "stderr was {err:?}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cosmic_script_cancel() {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = bin()
        .args(["--frontend", "cosmic", "--script"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"args":{"default_dir":"/tmp/dl"},"events":[{"cancel":true}]}"#)
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"action\":\"cancel\"") || stdout.contains("cancel"),
        "stdout was {stdout:?}"
    );
}

#[test]
fn unknown_flag_does_not_crash() {
    // Dispatcher treats unknown args as "open the UI", but --help is the
    // non-interactive smoke path. A second flag after --help is ignored.
    let out = bin().args(["--help", "--nope"]).output().unwrap();
    assert!(out.status.success());
}
