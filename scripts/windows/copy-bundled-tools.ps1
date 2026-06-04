. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$toolsDir = Join-Path (Get-BuildDir $root) "tools"
$bundled = Join-Path $root "bundled-tools"

New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null
foreach ($tool in @("ffmpeg.exe", "ffprobe.exe")) {
  $src = Join-Path $bundled $tool
  if (Test-Path $src) {
    Copy-Item $src $toolsDir -Force
  }
}
