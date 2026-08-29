. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$packageDir = Initialize-WindowsPackageDir $root
$binary = Join-Path $packageDir "quark-downloader.exe"
$guiBinary = Join-Path $packageDir "quark-downloader-gui.exe"

Write-Host "quark-downloader (Windows build)"
Write-Host ""

Write-Host "  Compiling CLI + GUI..."
Push-Location $root
try {
  Invoke-Checked { cargo build --release -p quark-cli -p quark-gui-dispatch }
} finally {
  Pop-Location
}

Copy-Item (Join-Path $root "target\release\quark-downloader.exe") $binary -Force
Copy-Item (Join-Path $root "target\release\quark-downloader-gui.exe") $guiBinary -Force
Copy-Item (Join-Path $root "LICENSE") $packageDir -Force
Copy-Item (Join-Path $root "README.md") $packageDir -Force

$toolsDir = Join-Path $packageDir "tools"
$bundled = Join-Path $root "bundled-tools"
foreach ($tool in @("ffmpeg.exe", "ffprobe.exe", "yt-dlp.exe")) {
  $source = Join-Path $bundled $tool
  if (Test-Path -LiteralPath $source -PathType Leaf) {
    New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null
    Copy-Item $source $toolsDir -Force
  }
}

Write-Host "  UPX (CLI only)..."
if (Get-Command upx -ErrorAction SilentlyContinue) {
  & upx --best --lzma $binary
} else {
  Write-Host "  (upx not found, skipping)"
}

Write-Host ""
Write-Host "Done:"
Write-Host "  Staged portable package: $packageDir"
Write-Host "  Run 'just windows-release' for final files in dist/."
