fn main() {
    println!("cargo:rerun-if-changed=../../src/gui/macos");
    println!("cargo:rustc-check-cfg=cfg(appkit_ui)");
    #[cfg(target_os = "macos")]
    macos::compile();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;

    pub fn compile() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let src = manifest.join("../../src/gui/macos");
        let files = ["main.m", "Alerts.m", "SessionWindow.m", "ProgressWindow.m"];
        for file in files {
            let path = src.join(file);
            if !path.is_file() {
                panic!("missing AppKit UI source {}", path.display());
            }
        }

        let mut build = cc::Build::new();
        build
            .compiler("clang")
            .include(&src)
            .flag("-fobjc-arc")
            .flag("-mmacosx-version-min=11.0")
            .warnings(false)
            .cargo_metadata(true);
        for file in files {
            build.file(src.join(file));
        }
        build.compile("quark_appkit_ui");

        println!("cargo:rustc-cfg=appkit_ui");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-arg=-ObjC");
    }
}
