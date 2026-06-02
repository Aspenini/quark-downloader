name := "quark-downloader"
build_dir := "build"
installer_output := "installer/output"
bundled_tools := "bundled-tools"
tools_dir := build_dir + "/tools"
exe_ext := if os() == "windows" { ".exe" } else { "" }
binary := build_dir + "/" + name + exe_ext

set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

default:
    @just --list

# Release build into build/, then UPX compress.
[windows]
copy-bundled-tools:
    if (-not (Test-Path {{tools_dir}})) { New-Item -ItemType Directory -Force -Path {{tools_dir}} | Out-Null }
    foreach ($tool in @('ffmpeg.exe','ffprobe.exe')) { if (Test-Path "{{bundled_tools}}/$tool") { Copy-Item "{{bundled_tools}}/$tool" "{{tools_dir}}/" -Force } }

[unix]
build:
    mkdir -p {{build_dir}}
    crystal build --release src/quark-downloader.cr -o {{binary}}
    command -v upx >/dev/null && upx --best --lzma {{binary}} || true

[windows]
build: copy-bundled-tools
    if (-not (Test-Path {{build_dir}})) { New-Item -ItemType Directory -Force -Path {{build_dir}} | Out-Null }
    crystal build --release src/quark-downloader.cr -o {{binary}}
    upx --best --lzma {{binary}}

run:
    crystal run src/quark-downloader.cr

[unix]
clean:
    rm -rf {{build_dir}} {{installer_output}}

[windows]
clean:
    if (Test-Path {{build_dir}}) { Remove-Item -Recurse -Force {{build_dir}} }
    if (Test-Path {{installer_output}}) { Remove-Item -Recurse -Force {{installer_output}} }
