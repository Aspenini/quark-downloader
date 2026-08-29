use std::path::{Path, PathBuf};
use std::process::Command;

pub fn embed(rc_name: &str, tag: &str) {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..");
    let rc = root.join("win32").join(rc_name);
    let version_info = root.join("win32/version-info.rcinc");
    let helper = root.join("build-support/windows_resources.rs");
    for input in [&rc, &version_info, &helper] {
        println!("cargo:rerun-if-changed={}", input.display());
    }
    assert!(
        rc.is_file(),
        "missing Windows resource file: {}",
        rc.display()
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out = out_dir.join(format!("{tag}.res"));
    write_version_header(&out_dir);
    assert!(
        compile(&rc, &out, &out_dir),
        "could not compile {} with rc.exe or windres",
        rc.display()
    );
    println!("cargo:rustc-link-arg={}", out.display());
}

fn write_version_header(out_dir: &Path) {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut numbers: Vec<u32> = version
        .split(['.', '-', '+'])
        .take(4)
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    numbers.resize(4, 0);
    let header = format!(
        "#define QUARK_VERSION_COMMA {}\n#define QUARK_VERSION_STRING \"{}\"\n",
        numbers
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(","),
        version
    );
    std::fs::write(out_dir.join("quark-version.h"), header).unwrap();
}

fn compile(rc: &Path, out: &Path, include_dir: &Path) -> bool {
    if Command::new("rc")
        .args([
            "/nologo",
            "/i",
            &include_dir.to_string_lossy(),
            "/fo",
            &out.to_string_lossy(),
            &rc.to_string_lossy(),
        ])
        .status()
        .is_ok_and(|status| status.success())
    {
        return true;
    }
    Command::new("windres")
        .args([
            "-i",
            &rc.to_string_lossy(),
            "-I",
            &include_dir.to_string_lossy(),
            "-o",
            &out.to_string_lossy(),
            "-O",
            "coff",
        ])
        .status()
        .is_ok_and(|status| status.success())
}
