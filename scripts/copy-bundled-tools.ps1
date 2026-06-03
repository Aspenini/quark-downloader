param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)
$ErrorActionPreference = "Stop"
$toolsDir = Join-Path $Root "build/tools"
$bundled = Join-Path $Root "bundled-tools"

New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null
foreach ($t in @("ffmpeg.exe", "ffprobe.exe")) {
  $src = Join-Path $bundled $t
  if (Test-Path $src) { Copy-Item $src $toolsDir -Force }
}
