. (Join-Path $PSScriptRoot "common.ps1")

$ErrorActionPreference = "Stop"
$root = Get-ProjectRoot
$version = Get-ProjectVersion $root
$packageDir = Get-WindowsPackageDir $root $version
$dist = Initialize-DistDir $root

if (-not (Test-Path -LiteralPath (Join-Path $packageDir "quark-downloader.exe") -PathType Leaf)) {
  throw "Portable package staging is missing. Run 'just build' first."
}

$portable = Join-Path $dist "quark-downloader-$version-windows-portable.zip"
if (Test-Path -LiteralPath $portable) {
  Remove-Item -LiteralPath $portable -Force
}
Compress-Archive -Path $packageDir -DestinationPath $portable -CompressionLevel Optimal

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

$setup = Join-Path $dist "quark-downloader-$version-setup.exe"
if (-not (Test-Path -LiteralPath $setup -PathType Leaf)) { throw "Missing release installer: $setup" }

Write-Warning "Windows binaries and installer are intentionally unsigned; SmartScreen may warn users."
Write-Host "Unsigned Windows release ready:"
Write-Host "  $portable"
Write-Host "  $setup"
