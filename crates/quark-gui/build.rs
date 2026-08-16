fn main() {
    #[cfg(windows)]
    windows::embed("gui.rc", "gui");
}

#[cfg(windows)]
mod windows {
    use std::path::PathBuf;
    use std::process::Command;

    pub fn embed(rc_name: &str, tag: &str) {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let root = manifest.join("../..");
        let rc = root.join("win32").join(rc_name);
        if !rc.exists() {
            return;
        }
        println!("cargo:rerun-if-changed={}", rc.display());
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join(format!("{tag}.res"));
        if compile(&rc, &out) {
            println!("cargo:rustc-link-arg={}", out.display());
        }
    }

    fn compile(rc: &std::path::Path, out: &std::path::Path) -> bool {
        if let Ok(status) = Command::new("rc")
            .args([
                "/nologo",
                "/fo",
                &out.to_string_lossy(),
                &rc.to_string_lossy(),
            ])
            .status()
        {
            if status.success() {
                return true;
            }
        }
        Command::new("windres")
            .args([
                "-i",
                &rc.to_string_lossy(),
                "-o",
                &out.to_string_lossy(),
                "-O",
                "coff",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}
