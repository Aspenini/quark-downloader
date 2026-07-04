. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$buildDir = Initialize-BuildDir $root
$binary = Join-Path $buildDir "quark-downloader.exe"
$guiBinary = Join-Path $buildDir "quark-downloader-gui.exe"

Write-Host "quark-downloader (Windows build)"
Write-Host ""

$iconRes = (Resolve-Path (Join-Path $buildDir "app.res")).Path
$guiFlags = Get-GuiLinkFlags $root

Write-Host "  Compiling CLI..."
Invoke-Checked { crystal build --release (Join-Path $root "src\quark-downloader.cr") -o $binary --link-flags="$iconRes" }

Write-Host "  Compiling GUI..."
Invoke-Checked { crystal build --release (Join-Path $root "src\gui\quark-downloader-gui.cr") -o $guiBinary --link-flags="$guiFlags" }

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
