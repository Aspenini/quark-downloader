. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$rootPrefix = [IO.Path]::GetFullPath($root).TrimEnd('\') + '\'
$gradlew = Join-Path $root "android\gradlew.bat"
if ((Test-Path -LiteralPath $gradlew) -and (Test-Path -LiteralPath (Join-Path $root "android\.gradle"))) {
  & $gradlew -p (Join-Path $root "android") --stop | Out-Null
}
foreach ($dir in @(
  (Join-Path $root "build"),
  (Join-Path $root "packaging\output"),
  (Join-Path $root "target"),
  (Join-Path $root "android\.gradle"),
  (Join-Path $root "android\.cxx"),
  (Join-Path $root "android\build"),
  (Join-Path $root "android\app\build"),
  (Join-Path $root "android\app\src\main\jniLibs")
)) {
  $resolved = [IO.Path]::GetFullPath($dir)
  if (-not $resolved.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean path outside the repository: $resolved"
  }
  if (Test-Path -LiteralPath $resolved) {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}

Write-Host "Cleaned Rust, desktop packaging, and Android build intermediates (dist/ preserved)"
