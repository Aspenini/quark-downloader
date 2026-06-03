$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$buildDir = Join-Path $root "build"
$src = Join-Path $root "src\gui\quark-downloader-gui.cr"
$cli = Join-Path $buildDir "quark-downloader.exe"

New-Item -ItemType Directory -Force -Path $buildDir | Out-Null

if (-not (Test-Path $cli)) {
  Write-Host "  Building CLI (required by GUI)..."
  crystal build (Join-Path $root "src\quark-downloader.cr") -o $cli
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
$env:QUARK_DOWNLOADER_CLI = (Resolve-Path $cli).Path

$hasGuiRes = (Test-Path (Join-Path $buildDir "gui_rc.o")) -or
  (Test-Path (Join-Path $buildDir "gui.lib")) -or
  (Test-Path (Join-Path $buildDir "gui.res"))
if (-not $hasGuiRes) {
  & (Join-Path $PSScriptRoot "embed-gui-res.ps1")
}

if (Test-Path (Join-Path $buildDir "gui_rc.o")) {
  $guiRes = (Resolve-Path (Join-Path $buildDir "gui_rc.o")).Path
} elseif (Test-Path (Join-Path $buildDir "gui.lib")) {
  $guiRes = (Resolve-Path (Join-Path $buildDir "gui.lib")).Path
} elseif (Test-Path (Join-Path $buildDir "gui.res")) {
  $guiRes = (Resolve-Path (Join-Path $buildDir "gui.res")).Path
} else {
  throw "Missing GUI resources. Run: just embed-icon-gui"
}

crystal run $src --link-flags="/SUBSYSTEM:WINDOWS $guiRes user32.lib shell32.lib comctl32.lib"
exit $LASTEXITCODE
