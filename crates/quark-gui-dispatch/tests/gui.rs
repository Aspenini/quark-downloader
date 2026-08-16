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
fn unknown_flag_does_not_crash() {
    // Dispatcher treats unknown args as "open the UI", but --help is the
    // non-interactive smoke path. A second flag after --help is ignored.
    let out = bin().args(["--help", "--nope"]).output().unwrap();
    assert!(out.status.success());
}
