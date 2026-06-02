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
embed-icon:
    if (-not (Test-Path {{build_dir}})) { New-Item -ItemType Directory -Force -Path {{build_dir}} | Out-Null }; $res = "{{build_dir}}/app.res"; $kitsRc = Get-ChildItem "$env:ProgramFiles(x86)\Windows Kits\10\bin\*\x64\rc.exe" -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1; if ($kitsRc) { & $kitsRc.FullName /nologo /fo $res win32/app.rc } elseif (Get-Command rc -ErrorAction SilentlyContinue) { rc /nologo /fo $res win32/app.rc } elseif (Get-Command windres -ErrorAction SilentlyContinue) { windres -O coff win32/app.rc $res } else { throw "Windows SDK rc.exe or windres required to embed icons/icon.ico" }

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
build: copy-bundled-tools embed-icon
    if (-not (Test-Path {{build_dir}})) { New-Item -ItemType Directory -Force -Path {{build_dir}} | Out-Null }; $iconRes = (Resolve-Path "{{build_dir}}/app.res").Path; crystal build --release src/quark-downloader.cr -o {{binary}} --link-flags="$iconRes"; upx --best --lzma {{binary}}

run:
    crystal run src/quark-downloader.cr

[unix]
clean:
    rm -rf {{build_dir}} {{installer_output}}

[windows]
clean:
    if (Test-Path {{build_dir}}) { Remove-Item -Recurse -Force {{build_dir}} }
    if (Test-Path {{installer_output}}) { Remove-Item -Recurse -Force {{installer_output}} }
