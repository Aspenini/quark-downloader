. (Join-Path $PSScriptRoot "common.ps1")

$ErrorActionPreference = "Stop"
$root = Get-ProjectRoot

$iscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if (-not $iscc) {
  foreach ($candidate in @(
    "C:\Program Files (x86)\Inno Setup 7\ISCC.exe",
    "C:\Program Files\Inno Setup 7\ISCC.exe"
  )) {
    if (Test-Path -LiteralPath $candidate) { $iscc = $candidate; break }
  }
}
if (-not $iscc) { throw "Inno Setup 7 compiler (ISCC.exe) was not found." }

& $iscc (Join-Path $root "packaging\quark-downloader.iss")
if ($LASTEXITCODE -ne 0) { throw "Inno Setup compilation failed." }

$cargo = Get-Content (Join-Path $root "Cargo.toml") -Raw
if ($cargo -notmatch '(?m)^version\s*=\s*"([^"]+)"') { throw "Could not read workspace version." }
$version = $Matches[1]
$setup = Join-Path $root "packaging\output\quark-downloader-$version-setup.exe"
if (-not (Test-Path -LiteralPath $setup -PathType Leaf)) { throw "Missing release installer: $setup" }

Write-Warning "Windows binaries and installer are intentionally unsigned; SmartScreen may warn users."
Write-Host "Unsigned Windows release ready: $setup"
