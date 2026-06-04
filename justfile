name := "quark-downloader"
gui_name := "quark-downloader-gui"
build_dir := "build"
installer_output := "packaging/output"
bundled_tools := "bundled-tools"
tools_dir := build_dir + "/tools"
exe_ext := if os() == "windows" { ".exe" } else { "" }
binary := build_dir + "/" + name + exe_ext
gui_binary := build_dir + "/" + gui_name + exe_ext

set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]
set quiet := true

[default]
default:
    @just --list

# Release build into build/, then UPX compress (CLI only on Windows; GUI must not be UPXed).
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

[group('dev')]
[unix]
run:
    @bash -c 'source scripts/unix/crystal-env.sh && crystal run src/quark-downloader.cr'

[group('dev')]
[windows]
run:
    @crystal run src/quark-downloader.cr

[group('dev')]
[unix]
run-gui:
    @bash -c 'source scripts/unix/crystal-env.sh && crystal run src/gui/quark-downloader-gui.cr'

[group('dev')]
[windows]
run-gui:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/run-gui.ps1

[group('clean')]
[unix]
clean:
    @bash scripts/unix/clean.sh

[group('clean')]
[windows]
clean:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/clean.ps1
