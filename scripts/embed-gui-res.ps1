# Compile win32/gui.rc for the GUI executable.
# Prefer MinGW windres (COFF .o) — links reliably with Crystal's MSVC toolchain.
# Fallback: SDK rc.exe -> .res -> import library for the linker.
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$buildDir = Join-Path $root "build"
$rcFile = Join-Path $root "win32\gui.rc"
$res = Join-Path $buildDir "gui.res"
$coff = Join-Path $buildDir "gui_rc.o"
$lib = Join-Path $buildDir "gui.lib"

New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
Remove-Item $res, $coff, $lib -Force -ErrorAction SilentlyContinue

if (Get-Command windres -ErrorAction SilentlyContinue) {
  & windres -O coff $rcFile -o $coff
  Write-Host "  GUI resources (windres) -> build/gui_rc.o"
  exit 0
}

$sdkRc = & (Join-Path $PSScriptRoot "win-sdk-rc.ps1")
if (-not $sdkRc) {
  $rcCmd = Get-Command rc -ErrorAction SilentlyContinue
  if ($rcCmd) { $sdkRc = $rcCmd.Source }
}
if (-not $sdkRc) {
  throw "Need windres (Git for Windows / MSYS) or Windows SDK rc.exe for GUI resources."
}

$kitsInc = "${env:ProgramFiles(x86)}\Windows Kits\10\Include"
$ver = Get-ChildItem $kitsInc -Directory -ErrorAction SilentlyContinue |
  Where-Object { $_.Name -match '^\d' } |
  Sort-Object Name -Descending |
  Select-Object -First 1
if (-not $ver) { throw "Windows SDK include directory not found under $kitsInc" }

$incUm = Join-Path $ver.FullName "um"
$incShared = Join-Path $ver.FullName "shared"
& $sdkRc /nologo /I $incUm /I $incShared /fo $res $rcFile

$libExe = Get-ChildItem "${env:ProgramFiles(x86)}\Microsoft Visual Studio\*\*\VC\Tools\MSVC\*\bin\Hostx64\x64\lib.exe" -ErrorAction SilentlyContinue |
  Sort-Object FullName -Descending |
  Select-Object -First 1
if (-not $libExe) {
  throw "Visual Studio lib.exe not found (needed to link rc.exe .res with Crystal)."
}
& $libExe.FullName /NOLOGO /MACHINE:X64 "/OUT:$lib" $res
Write-Host "  GUI resources (rc.exe) -> build/gui.lib"
exit 0
