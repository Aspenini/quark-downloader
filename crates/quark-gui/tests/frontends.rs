//! Run the oracle fixtures through every frontend `--script` binary present
//! on this host. Missing frontends are skipped, not failed.

use std::process::{Command, Stdio};

use quark_core::session::{self, MainAction};
use quark_gui::script::run;

fn fixtures() -> Vec<(&'static str, String)> {
    vec![
        (
            "two_urls",
            script(
                r#"[{"add_url":"https://example.com/a"},{"add_url":"https://example.com/b"},{"download":true}]"#,
            ),
        ),
        (
            "duplicate",
            script(
                r#"[{"add_url":"https://example.com/a"},{"add_url":"https://example.com/a"},{"download":true}]"#,
            ),
        ),
        ("empty_download", script(r#"[{"download":true}]"#)),
        (
            "empty_output",
            script(
                r#"[{"add_url":"https://example.com/a"},{"set_output":"  "},{"download":true}]"#,
            ),
        ),
        (
            "flush_field",
            script(r#"[{"set_url_field":"https://example.com/z"},{"download":true}]"#),
        ),
        (
            "audio_mp3",
            script(
                r#"[{"add_url":"https://example.com/a"},{"set_media":"audio"},{"set_format":"mp3"},{"download":true}]"#,
            ),
        ),
        ("cancel", script(r#"[{"cancel":true}]"#)),
        (
            "paste",
            script(
                r#"[{"paste":"https://example.com/a\nhttps://example.com/b"},{"download":true}]"#,
            ),
        ),
    ]
}

fn script(events: &str) -> String {
    format!(r#"{{"args":{{"default_dir":"/tmp/dl"}},"events":{events}}}"#)
}

struct FrontendCmd {
    name: &'static str,
    bin: String,
    prefix: Vec<String>,
}

fn frontend_bins() -> Vec<FrontendCmd> {
    let mut bins = Vec::new();
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_quark-downloader-gui-win32") {
        bins.push(FrontendCmd {
            name: "win32",
            bin: path,
            prefix: Vec::new(),
        });
    }
    // Integration tests in quark-gui cannot see other crates' CARGO_BIN_EXE.
    let mut candidates: Vec<(&str, &str, Vec<String>)> =
        vec![("win32", "quark-downloader-gui-win32", vec![])];
    if cfg!(target_os = "linux") {
        candidates.push((
            "qt",
            "quark-downloader-gui",
            vec!["--frontend".into(), "qt".into()],
        ));
    }
    if cfg!(target_os = "macos") {
        candidates.push((
            "appkit",
            "quark-downloader-gui",
            vec!["--frontend".into(), "appkit".into()],
        ));
    }
    for (name, file, prefix) in candidates {
        if bins.iter().any(|b| b.name == name) {
            continue;
        }
        if let Some(found) = discover_bin(file) {
            bins.push(FrontendCmd {
                name,
                bin: found,
                prefix,
            });
        }
    }
    bins
}

fn discover_bin(name: &str) -> Option<String> {
    let exe = if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::PathBuf::from));
    let target_dir = exe_dir.as_ref().and_then(|d| {
        if d.ends_with("deps") {
            d.parent().map(std::path::PathBuf::from)
        } else {
            Some(d.clone())
        }
    });
    let candidates = [
        exe_dir.map(|d| d.join(&exe)),
        target_dir.map(|d| d.join(&exe)),
        Some(std::path::PathBuf::from("target/debug").join(&exe)),
        Some(std::path::PathBuf::from("target/release").join(&exe)),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

fn run_frontend(cmd: &FrontendCmd, input: &str) -> (i32, String) {
    let mut child = Command::new(&cmd.bin)
        .args(&cmd.prefix)
        .arg("--script")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{} ({}): {e}", cmd.name, cmd.bin));
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    let code = out.status.code().unwrap_or(1);
    (code, String::from_utf8_lossy(&out.stdout).into_owned())
}

fn semantic_eq(oracle: &str, frontend: &str) {
    let a = session::parse(oracle);
    let b = session::parse(frontend.trim());
    match (&a.action, &b.action) {
        (MainAction::Download(x), MainAction::Download(y)) => {
            assert_eq!(x.urls, y.urls);
            assert_eq!(x.media_type, y.media_type);
            assert_eq!(x.format, y.format);
            assert_eq!(x.output_dir, y.output_dir);
        }
        (MainAction::Cancel, MainAction::Cancel) => {}
        (MainAction::Error(_), MainAction::Error(_)) => {}
        other => panic!("action mismatch: {other:?}\noracle={oracle}\nfrontend={frontend}"),
    }
    assert_eq!(
        a.settings_form.is_some(),
        b.settings_form.is_some(),
        "settings presence mismatch"
    );
}

#[test]
fn frontends_match_oracle() {
    let bins = frontend_bins();
    if bins.is_empty() {
        // Other crates' binaries are not guaranteed when this test package
        // is built alone; missing frontends are skipped, not failed.
        return;
    }
    for (name, input) in fixtures() {
        let oracle = run(&input).unwrap();
        let oracle_json = oracle.to_json();
        for cmd in &bins {
            let (code, stdout) = run_frontend(cmd, &input);
            assert_eq!(
                code,
                oracle.exit_code(),
                "{}/{} exit {code} vs {}",
                cmd.name,
                name,
                oracle.exit_code()
            );
            semantic_eq(&oracle_json, &stdout);
        }
    }
}
