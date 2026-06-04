. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$buildDir = Initialize-BuildDir $root
$rcFile = Join-Path $root "win32\app.rc"
$res = Join-Path $buildDir "app.res"
$iconIco = Join-Path $root "icons\icon-cli.ico"

if (-not (Test-Path $iconIco)) {
  throw "Missing icons/icon-cli.ico (CLI Windows icon)"
}

$sdkRc = Get-WindowsSdkRc
if ($sdkRc) {
  & $sdkRc /nologo /fo $res $rcFile
  Write-Host "  CLI resources (rc.exe) -> build/app.res"
  exit 0
}

$rcCmd = Get-Command rc -ErrorAction SilentlyContinue
if ($rcCmd) {
  & $rcCmd.Source /nologo /fo $res $rcFile
  Write-Host "  CLI resources (rc) -> build/app.res"
  exit 0
}

if (Get-Command windres -ErrorAction SilentlyContinue) {
  & windres -O coff $rcFile $res
  Write-Host "  CLI resources (windres) -> build/app.res"
  exit 0
}

throw "Windows SDK rc.exe or windres required to embed icons/icon-cli.ico"
