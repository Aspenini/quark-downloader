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
