use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quark-downloader-gui-gtk"))
}

#[test]
fn missing_mode_exits_nonzero() {
    let out = bin().output().unwrap();
    assert!(!out.status.success());
}

#[cfg(not(target_os = "linux"))]
#[test]
fn non_linux_is_stub() {
    let out = bin().arg("--help").output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Linux"), "stderr was {err:?}");
}

#[cfg(target_os = "linux")]
#[test]
fn usage_without_display() {
    let out = bin().output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("usage") || err.contains("session") || !out.status.success(),
        "stderr was {err:?}"
    );
}
