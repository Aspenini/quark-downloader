. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$buildDir = Initialize-BuildDir $root
$binary = Join-Path $buildDir "quark-downloader.exe"
$guiBinary = Join-Path $buildDir "quark-downloader-gui.exe"

Write-Host "quark-downloader (Windows build)"
Write-Host ""

Write-Host "  Compiling CLI + GUI..."
Push-Location $root
try {
  Invoke-Checked { cargo build --release -p quark-cli -p quark-gui }
} finally {
  Pop-Location
}

Copy-Item (Join-Path $root "target\release\quark-downloader.exe") $binary -Force
Copy-Item (Join-Path $root "target\release\quark-downloader-gui.exe") $guiBinary -Force

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
