. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$buildDir = Initialize-BuildDir $root
$src = Join-Path $root "src\gui\quark-downloader-gui.cr"
$cli = Join-Path $buildDir "quark-downloader.exe"

if (-not (Test-Path $cli)) {
  Write-Host "  Building CLI (required by GUI)..."
  Invoke-Checked { crystal build (Join-Path $root "src\quark-downloader.cr") -o $cli }
}
$env:QUARK_DOWNLOADER_CLI = (Resolve-Path $cli).Path

if (-not (Test-Path (Join-Path $buildDir "gui_rc.o")) -and
    -not (Test-Path (Join-Path $buildDir "gui.lib")) -and
    -not (Test-Path (Join-Path $buildDir "gui.res"))) {
  & (Join-Path $PSScriptRoot "compile-gui-resources.ps1")
}

$guiFlags = Get-GuiLinkFlags $root
crystal run $src --link-flags="$guiFlags"
exit $LASTEXITCODE
