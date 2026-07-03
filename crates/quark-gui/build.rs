fn main() {
    // Qt backend: compile the cxx bridge and the Qt Widgets C++ shim.
    // Requires a Qt 6 installation discoverable via qmake on PATH (or QMAKE).
    #[cfg(feature = "native-kirigami")]
    {
        let qt = qt_build_utils::QtBuild::new(vec!["Core".into(), "Gui".into(), "Widgets".into()])
            .expect("locate Qt 6 (is qmake on PATH, or set QMAKE?)");

        let mut build = cxx_build::bridge("src/backends/kirigami.rs");
        for path in qt.include_paths() {
            build.include(path);
        }
        for path in qt.framework_paths() {
            build.flag(format!("-F{}", path.display()));
        }
        build.file("cpp/qt_backend.cpp").std("c++17");
        qt.cargo_link_libraries(&mut build);
        build.compile("quark-gui-qt");

        println!("cargo:rerun-if-changed=cpp/qt_backend.cpp");
        println!("cargo:rerun-if-changed=cpp/qt_backend.h");
        println!("cargo:rerun-if-changed=src/backends/kirigami.rs");
    }
}
