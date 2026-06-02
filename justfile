name := "quark-downloader"
build_dir := "build"
exe_ext := if os() == "windows" { ".exe" } else { "" }
binary := build_dir + "/" + name + exe_ext

set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

default:
    @just --list

# Release build into build/, then UPX compress.
[unix]
build:
    mkdir -p {{build_dir}}
    crystal build --release quark-downloader.cr -o {{binary}}
    upx --best --lzma {{binary}}

[windows]
build:
    if (-not (Test-Path {{build_dir}})) { New-Item -ItemType Directory -Force -Path {{build_dir}} | Out-Null }
    crystal build --release quark-downloader.cr -o {{binary}}
    upx --best --lzma {{binary}}

run:
    crystal run quark-downloader.cr

[unix]
clean:
    rm -rf {{build_dir}}

[windows]
clean:
    if (Test-Path {{build_dir}}) { Remove-Item -Recurse -Force {{build_dir}} }
