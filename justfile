name := "quark-downloader"
gui_name := "quark-downloader-gui"
build_dir := "build"
exe_ext := if os() == "windows" { ".exe" } else { "" }
binary := build_dir + "/" + name + exe_ext
gui_binary := build_dir + "/" + gui_name + exe_ext

set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]
set quiet := true

[default]
default:
    @just --list

[group('build')]
[private]
[windows]
compile-cli-resources:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/compile-cli-resources.ps1

[group('build')]
[private]
[windows]
compile-gui-resources:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/compile-gui-resources.ps1

[group('build')]
[private]
[windows]
copy-bundled-tools:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/copy-bundled-tools.ps1

[group('build')]
[unix]
build:
    @bash scripts/unix/build.sh

[group('build')]
[windows]
build: copy-bundled-tools compile-cli-resources compile-gui-resources
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/build.ps1

[group('build')]
[unix]
dmg:
    @bash scripts/macos/build-dmg.sh

[group('dev')]
run:
    @cargo run -p quark-cli --

[group('dev')]
[unix]
run-gui:
    @cargo build -p quark-cli -p quark-gui -p quark-gui-gtk
    @QUARK_DOWNLOADER_CLI=target/debug/quark-downloader cargo run -p quark-gui --

[group('dev')]
[windows]
run-gui:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/run-gui.ps1

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
