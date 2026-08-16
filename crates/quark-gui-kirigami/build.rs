fn main() {
    println!("cargo:rerun-if-changed=../../src/gui/kirigami");
    println!("cargo:rustc-check-cfg=cfg(kirigami_ui)");
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
        let cpp = src.join("main.cpp");
        if !cpp.is_file() {
            println!("cargo:warning=Kirigami main.cpp missing at {}", cpp.display());
            return;
        }

        let Some(cflags) = pkg_config(&["--cflags", "Qt6Core", "Qt6Gui", "Qt6Qml", "Qt6Quick"])
        else {
            println!(
                "cargo:warning=Qt 6 QML not found via pkg-config; Kirigami UI will not be linked (install qt6-declarative / qt6-declarative-dev and kirigami)"
            );
            return;
        };
        let Some(libs) = pkg_config(&["--libs", "Qt6Core", "Qt6Gui", "Qt6Qml", "Qt6Quick"]) else {
            println!("cargo:warning=pkg-config --libs Qt6Quick failed; Kirigami UI will not be linked");
            return;
        };

        let qml = src.canonicalize().unwrap_or(src.clone());
        let qml_define = format!("\"{}\"", qml.display());
        let mut build = cc::Build::new();
        build
            .cpp(true)
            .file(&cpp)
            .flag_if_supported("-std=c++17")
            .flag_if_supported("-fPIC")
            .define("KIRIGAMI_AS_LIBRARY", None)
            .define("QUARK_KIRIGAMI_QML", qml_define.as_str())
            .warnings(false)
            .cargo_metadata(true);

        for flag in &cflags {
            if let Some(inc) = flag.strip_prefix("-I") {
                build.include(inc);
            } else if let Some(inc) = flag.strip_prefix("-isystem") {
                if inc.is_empty() {
                    continue;
                }
                build.flag(flag);
            } else {
                build.flag_if_supported(flag);
            }
        }

        match build.try_compile("quark_kirigami_ui") {
            Ok(()) => {
                println!("cargo:rustc-cfg=kirigami_ui");
                println!("cargo:rustc-link-lib=dylib=stdc++");
                apply_link_flags(&libs);
            }
            Err(e) => {
                println!("cargo:warning=failed to compile Kirigami UI with system Qt: {e}");
            }
        }
    }

    fn apply_link_flags(flags: &[String]) {
        let mut pending_isystem = false;
        for flag in flags {
            if pending_isystem {
                pending_isystem = false;
                continue;
            }
            if flag == "-isystem" {
                pending_isystem = true;
                continue;
            }
            if let Some(path) = flag.strip_prefix("-L") {
                println!("cargo:rustc-link-search=native={path}");
            } else if let Some(lib) = flag.strip_prefix("-l") {
                println!("cargo:rustc-link-lib=dylib={lib}");
            } else if flag.starts_with("-Wl,") {
                println!("cargo:rustc-link-arg={flag}");
            }
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
}
