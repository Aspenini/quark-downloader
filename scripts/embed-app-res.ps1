$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$buildDir = Join-Path $root "build"
$rcFile = Join-Path $root "win32\app.rc"
$res = Join-Path $buildDir "app.res"
$iconIco = Join-Path $root "icons\icon-cli.ico"

if (-not (Test-Path $iconIco)) {
  throw "Missing icons/icon-cli.ico (CLI Windows icon)"
}

New-Item -ItemType Directory -Force -Path $buildDir | Out-Null

$sdkRc = & (Join-Path $PSScriptRoot "win-sdk-rc.ps1")
if ($sdkRc) {
  & $sdkRc /nologo /fo $res $rcFile
  Write-Host "  CLI icon (rc.exe) -> build/app.res"
  exit 0
}
if (Get-Command rc -ErrorAction SilentlyContinue) {
  & rc /nologo /fo $res $rcFile
  Write-Host "  CLI icon (rc) -> build/app.res"
  exit 0
}
if (Get-Command windres -ErrorAction SilentlyContinue) {
  & windres -O coff $rcFile $res
  Write-Host "  CLI icon (windres) -> build/app.res"
  exit 0
}
throw "Windows SDK rc.exe or windres required to embed icons/icon-cli.ico"
