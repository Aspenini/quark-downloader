fn main() {
    println!("cargo:rerun-if-changed=../../src/gui/qt");
    println!("cargo:rustc-check-cfg=cfg(qt_ui)");
    #[cfg(target_os = "linux")]
    linux::try_compile();
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::PathBuf;
    use std::process::Command;

    pub fn try_compile() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let src = manifest.join("../../src/gui/qt");
        let cpp = src.join("main.cpp");
        if !cpp.is_file() {
            println!("cargo:warning=Qt main.cpp missing at {}", cpp.display());
            return;
        }

        let Some(cflags) = pkg_config(&["--cflags", "Qt6Core", "Qt6Gui", "Qt6Qml", "Qt6Quick"])
        else {
            println!(
                "cargo:warning=Qt 6 QML not found via pkg-config; Qt UI will not be linked (install qt6-declarative / qt6-declarative-dev)"
            );
            return;
        };
        let Some(libs) = pkg_config(&["--libs", "Qt6Core", "Qt6Gui", "Qt6Qml", "Qt6Quick"]) else {
            println!("cargo:warning=pkg-config --libs Qt6Quick failed; Qt UI will not be linked");
            return;
        };

        let qml = src.canonicalize().unwrap_or(src.clone());
        let qml_define = format!("\"{}\"", qml.display());
        // cc applies CFLAGS/CXXFLAGS after builder flags, so makepkg's
        // -flto=auto would override -fno-lto and emit GCC LTO objects that
        // rust-lld cannot extract qt_ui_run from.
        strip_env_lto();
        let mut build = cc::Build::new();
        build
            .cpp(true)
            .file(&cpp)
            .flag_if_supported("-std=c++17")
            .flag_if_supported("-fPIC")
            .flag_if_supported("-fno-lto")
            .define("QUARK_QT_AS_LIBRARY", None)
            .define("QUARK_QT_QML", qml_define.as_str())
            .warnings(false)
            .cargo_metadata(true);

        let mut cflags = cflags.iter();
        while let Some(flag) = cflags.next() {
            if flag == "-I" || flag == "-isystem" {
                if let Some(path) = cflags.next() {
                    build.include(path);
                }
                continue;
            }
            if let Some(inc) = flag.strip_prefix("-I") {
                build.include(inc);
            } else if let Some(inc) = flag.strip_prefix("-isystem") {
                let inc = inc.trim_start_matches('=');
                if !inc.is_empty() {
                    build.include(inc);
                }
            } else {
                build.flag_if_supported(flag);
            }
        }

        match build.try_compile("quark_qt_ui") {
            Ok(()) => {
                println!("cargo:rustc-cfg=qt_ui");
                println!("cargo:rustc-link-lib=dylib=stdc++");
                apply_link_flags(&libs);
            }
            Err(e) => {
                println!("cargo:warning=failed to compile Qt UI with system Qt: {e}");
            }
        }
    }

    fn strip_env_lto() {
        for key in ["CFLAGS", "CXXFLAGS"] {
            let Ok(val) = std::env::var(key) else {
                continue;
            };
            let mut flags: Vec<&str> = val
                .split_whitespace()
                .filter(|flag| *flag != "-flto" && !flag.starts_with("-flto="))
                .collect();
            if !flags.contains(&"-fno-lto") {
                flags.push("-fno-lto");
            }
            // Safety: build script is single-threaded.
            unsafe {
                std::env::set_var(key, flags.join(" "));
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
