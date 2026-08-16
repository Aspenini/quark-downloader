. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
foreach ($dir in @(
  (Join-Path $root "build"),
  (Join-Path $root "packaging\output"),
  (Join-Path $root "target")
)) {
  if (Test-Path $dir) {
    Remove-Item -Recurse -Force $dir
  }
}

Write-Host "Cleaned build/, packaging/output/, and target/"
