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
    @powershell -NoProfile -Command "if (-not (Test-Path '{{tools_dir}}')) { New-Item -ItemType Directory -Force -Path '{{tools_dir}}' | Out-Null }; foreach ($t in @('ffmpeg.exe','ffprobe.exe')) { $s = Join-Path '{{bundled_tools}}' $t; if (Test-Path $s) { Copy-Item $s '{{tools_dir}}' -Force } }"

[group('build')]
[unix]
build:
    @mkdir -p {{build_dir}}
    @crystal build --release src/quark-downloader.cr -o {{binary}}
    @crystal build --release src/gui/quark-downloader-gui.cr -o {{gui_binary}}
    @command -v upx >/dev/null && upx --best --lzma {{binary}} || true

[group('build')]
[windows]
build:
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

[group('clean')]
[windows]
clean:
    @powershell -NoProfile -Command "if (Test-Path '{{build_dir}}') { Remove-Item -Recurse -Force '{{build_dir}}' }; if (Test-Path '{{installer_output}}') { Remove-Item -Recurse -Force '{{installer_output}}' }; Write-Host 'Cleaned build/ and packaging/output/'"
