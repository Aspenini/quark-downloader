param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)
$ErrorActionPreference = "Stop"
$buildDir = Join-Path $Root "build"
$binary = Join-Path $buildDir "quark-downloader.exe"
$guiBinary = Join-Path $buildDir "quark-downloader-gui.exe"

Write-Host "quark-downloader (Windows build)"
Write-Host ""

$iconRes = (Resolve-Path (Join-Path $buildDir "app.res")).Path
if (Test-Path (Join-Path $buildDir "gui_rc.o")) {
  $guiRes = (Resolve-Path (Join-Path $buildDir "gui_rc.o")).Path
} elseif (Test-Path (Join-Path $buildDir "gui.lib")) {
  $guiRes = (Resolve-Path (Join-Path $buildDir "gui.lib")).Path
} elseif (Test-Path (Join-Path $buildDir "gui.res")) {
  $guiRes = (Resolve-Path (Join-Path $buildDir "gui.res")).Path
} else {
  throw "Missing GUI resources. Run: just embed-icon-gui"
}

Write-Host "  Compiling CLI..."
crystal build --release (Join-Path $Root "src\quark-downloader.cr") -o $binary --link-flags="$iconRes"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "  Compiling GUI..."
crystal build --release (Join-Path $Root "src\gui\quark-downloader-gui.cr") -o $guiBinary `
  --link-flags="/SUBSYSTEM:WINDOWS $guiRes user32.lib shell32.lib comctl32.lib"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "  UPX (CLI only)..."
if (Get-Command upx -ErrorAction SilentlyContinue) {
  & upx --best --lzma $binary
} else {
  Write-Host "  (upx not found, skipping)"
}

Write-Host ""
Write-Host "Done:"
Write-Host "  $binary"
Write-Host "  $guiBinary"
