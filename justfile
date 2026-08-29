set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]
set quiet := true

[default]
default:
    @just --list

[group('build')]
[unix]
build:
    @bash scripts/unix/build.sh

[group('build')]
[windows]
build:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/build.ps1

[group('build')]
[linux]
linux-release:
    @bash scripts/unix/package-release.sh

[group('build')]
[macos]
macos-release:
    @bash scripts/macos/build-dmg.sh

[private]
[macos]
dmg: macos-release

[group('build')]
[windows]
windows-release: build
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/package-release.ps1

[group('dev')]
run:
    @cargo run -p quark-cli --

[group('dev')]
[unix]
run-gui:
    @cargo build -p quark-cli -p quark-gui-dispatch
    @mkdir -p target/debug/qml && cp src/gui/qt/*.qml target/debug/qml/ || true
    @QUARK_DOWNLOADER_CLI=target/debug/quark-downloader cargo run -p quark-gui-dispatch --

[group('dev')]
[windows]
run-gui:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/run-gui.ps1

[group('android')]
[windows]
android-debug:
    @& .\android\gradlew.bat -p android :app:assembleDebug

[group('android')]
[unix]
android-debug:
    @android/gradlew -p android :app:assembleDebug

[group('android')]
[windows]
android-run:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/run-android.ps1

[group('android')]
[unix]
android-run:
    @bash scripts/unix/run-android.sh

[group('android')]
[windows]
android-release:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/release-android.ps1

[group('android')]
[unix]
android-release:
    @bash scripts/unix/release-android.sh

[group('test')]
test:
    @cargo test --workspace

[group('clean')]
[unix]
clean:
    @bash scripts/unix/clean.sh

[group('clean')]
[windows]
clean:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/clean.ps1
