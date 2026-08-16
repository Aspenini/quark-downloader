. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$cli = Join-Path $root "target\debug\quark-downloader.exe"

Push-Location $root
try {
  if (-not (Test-Path $cli)) {
    Write-Host "  Building CLI (required by GUI)..."
    Invoke-Checked { cargo build -p quark-cli }
  }
  $env:QUARK_DOWNLOADER_CLI = (Resolve-Path $cli).Path
  cargo run -p quark-gui --
  exit $LASTEXITCODE
} finally {
  Pop-Location
}
