fn main() {
    println!("cargo:rerun-if-changed=../../src/gui/kirigami");
    #[cfg(target_os = "linux")]
    linux::try_compile();
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::PathBuf;
    use std::process::Command;

    pub fn try_compile() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let src = manifest.join("../../src/gui/kirigami");
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let bin = out.join("quark-downloader-gui-kirigami-ui");
        let cflags = pkg_config(["--cflags", "Qt6Core", "Qt6Gui", "Qt6Qml", "Qt6Quick"]);
        let libs = pkg_config(["--libs", "Qt6Core", "Qt6Gui", "Qt6Qml", "Qt6Quick"]);
        if cflags.is_none() || libs.is_none() {
            println!(
                "cargo:warning=Qt 6 QML not found via pkg-config; Kirigami UI helper will not be built (install qt6-declarative-dev / qml6-module-org-kde-kirigami)"
            );
            return;
        }
        let mut cmd = Command::new("c++");
        cmd.arg("-O2")
            .arg("-fPIC")
            .arg(src.join("main.cpp"))
            .arg("-o")
            .arg(&bin)
            .arg(format!("-DQUARK_KIRIGAMI_QML=\"{}\"", src.display()));
        for flag in cflags.unwrap() {
            cmd.arg(flag);
        }
        for flag in libs.unwrap() {
            cmd.arg(flag);
        }
        match cmd.status() {
            Ok(s) if s.success() => {
                if let Some(dest) = target_sibling("quark-downloader-gui-kirigami-ui") {
                    let _ = std::fs::copy(&bin, dest);
                }
            }
            _ => println!("cargo:warning=failed to compile Kirigami UI helper with system Qt"),
        }
    }

    fn pkg_config(args: &[&str]) -> Option<Vec<String>> {
        let out = Command::new("pkg-config").args(args).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        Some(text.split_whitespace().map(str::to_string).collect())
    }

    fn target_sibling(name: &str) -> Option<PathBuf> {
        let out = PathBuf::from(std::env::var("OUT_DIR").ok()?);
        Some(out.join("../../../").join(name))
    }
}
