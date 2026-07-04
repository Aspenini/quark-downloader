. (Join-Path $PSScriptRoot "common.ps1")

$root = Get-ProjectRoot
$buildDir = Initialize-BuildDir $root
$rcFile = Join-Path $root "win32\gui.rc"
$res = Join-Path $buildDir "gui.res"
$coff = Join-Path $buildDir "gui_rc.o"
$lib = Join-Path $buildDir "gui.lib"

Remove-Item $res, $coff, $lib -Force -ErrorAction SilentlyContinue

if (Get-Command windres -ErrorAction SilentlyContinue) {
  & windres -O coff $rcFile -o $coff
  Write-Host "  GUI resources (windres) -> build/gui_rc.o"
  exit 0
}

$sdkRc = Get-WindowsSdkRc
$rcCmd = Get-Command rc -ErrorAction SilentlyContinue
if (-not $sdkRc -and $rcCmd) { $sdkRc = $rcCmd.Source }
if (-not $sdkRc) {
  throw "Need windres (Git for Windows / MSYS) or Windows SDK rc.exe for GUI resources."
}

$includes = Get-WindowsSdkIncludeDirs
& $sdkRc /nologo /I $includes.Um /I $includes.Shared /fo $res $rcFile

$libExe = Get-VsLibExe
& $libExe /NOLOGO /MACHINE:X64 "/OUT:$lib" $res
Write-Host "  GUI resources (rc.exe) -> build/gui.lib"
