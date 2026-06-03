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
embed-icon:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/embed-app-res.ps1

[group('build')]
[private]
[windows]
embed-icon-gui:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/embed-gui-res.ps1

[group('build')]
[private]
[windows]
copy-bundled-tools:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/copy-bundled-tools.ps1

[group('build')]
[unix]
build:
    @bash scripts/build-unix.sh

[group('build')]
[windows]
build: copy-bundled-tools embed-icon embed-icon-gui
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1

[group('dev')]
run:
    @crystal run src/quark-downloader.cr

[group('dev')]
[unix]
run-gui:
    @crystal run src/gui/quark-downloader-gui.cr

[group('dev')]
[windows]
run-gui:
    @powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-gui-windows.ps1

[group('clean')]
[unix]
clean:
    @rm -rf {{build_dir}} {{installer_output}}
    @echo "Cleaned build/ and packaging/output/"

[group('clean')]
[windows]
clean:
    @powershell -NoProfile -Command "if (Test-Path '{{build_dir}}') { Remove-Item -Recurse -Force '{{build_dir}}' }; if (Test-Path '{{installer_output}}') { Remove-Item -Recurse -Force '{{installer_output}}' }; Write-Host 'Cleaned build/ and packaging/output/'"
