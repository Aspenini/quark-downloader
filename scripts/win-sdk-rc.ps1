# Returns full path to Windows SDK rc.exe, or $null.
$ErrorActionPreference = "Stop"
$kitsRoot = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
if (-not (Test-Path $kitsRoot)) { return $null }
Get-ChildItem "$kitsRoot\*\x64\rc.exe" -ErrorAction SilentlyContinue |
  Sort-Object FullName -Descending |
  Select-Object -First 1 -ExpandProperty FullName
