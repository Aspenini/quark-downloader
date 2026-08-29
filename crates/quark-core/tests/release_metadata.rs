use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn release_versions_are_synchronized() {
    let version = env!("CARGO_PKG_VERSION");
    assert!(read("android/app/build.gradle.kts").contains(&format!("versionName = \"{version}\"")));
    assert!(read("packaging/PKGBUILD").contains(&format!("pkgver={version}")));
    assert!(
        read("packaging/quark-downloader.iss")
            .contains(&format!("#define MyAppVersion    \"{version}\""))
    );
}

#[test]
fn android_distribution_includes_license_sources() {
    assert!(root().join("android/LICENSE").is_file());
    assert!(root().join("android/APACHE-2.0").is_file());
    assert!(root().join("android/THIRD_PARTY_NOTICES.md").is_file());
}

#[test]
fn release_artifacts_share_one_dist_directory() {
    let windows = read("scripts/windows/package-release.ps1");
    let linux = read("scripts/unix/package-release.sh");
    let macos = read("scripts/macos/build-dmg.sh");
    let android_windows = read("scripts/windows/release-android.ps1");
    let android_unix = read("scripts/unix/release-android.sh");
    let installer = read("packaging/quark-downloader.iss");

    assert!(windows.contains("Initialize-DistDir"));
    assert!(linux.contains("$root/dist/"));
    assert!(macos.contains("dist=\"$root/dist\""));
    assert!(android_windows.contains("Join-Path $root \"dist\""));
    assert!(android_unix.contains("$root/dist/"));
    assert!(installer.contains("OutputDir=..\\dist"));

    let packaging = format!("{windows}\n{linux}\n{macos}\n{installer}");
    assert!(!packaging.contains("packaging\\output"));
    assert!(!packaging.contains("$root/build"));
    assert!(!packaging.contains("..\\build"));
}
