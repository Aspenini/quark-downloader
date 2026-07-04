$ErrorActionPreference = "Stop"

function Get-ProjectRoot {
  param([string]$ScriptRoot = $PSScriptRoot)
  return (Resolve-Path (Join-Path $ScriptRoot "..\..")).Path
}

function Get-BuildDir {
  param([string]$Root)
  return (Join-Path $Root "build")
}

function Initialize-BuildDir {
  param([string]$Root)
  $buildDir = Get-BuildDir $Root
  New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
  return $buildDir
}

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)]
    [scriptblock]$Command
  )
  & $Command
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

function Get-WindowsSdkRc {
  $kitsRoot = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
  if (-not (Test-Path $kitsRoot)) { return $null }
  return Get-ChildItem "$kitsRoot\*\x64\rc.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName
}

function Get-WindowsSdkIncludeDirs {
  $kitsInc = "${env:ProgramFiles(x86)}\Windows Kits\10\Include"
  $ver = Get-ChildItem $kitsInc -Directory -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '^\d' } |
    Sort-Object Name -Descending |
    Select-Object -First 1
  if (-not $ver) { throw "Windows SDK include directory not found under $kitsInc" }

  return @{
    Um     = Join-Path $ver.FullName "um"
    Shared = Join-Path $ver.FullName "shared"
  }
}

function Get-VsLibExe {
  $libExe = Get-ChildItem "${env:ProgramFiles(x86)}\Microsoft Visual Studio\*\*\VC\Tools\MSVC\*\bin\Hostx64\x64\lib.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1
  if (-not $libExe) {
    throw "Visual Studio lib.exe not found (needed to link rc.exe .res with Crystal)."
  }
  return $libExe.FullName
}

function Get-GuiResourceLinkInput {
  param([string]$Root)

  $buildDir = Get-BuildDir $Root
  foreach ($name in @("gui_rc.o", "gui.lib", "gui.res")) {
    $candidate = Join-Path $buildDir $name
    if (Test-Path $candidate) {
      return (Resolve-Path $candidate).Path
    }
  }

  throw "Missing GUI resources. Run scripts/windows/compile-gui-resources.ps1 or just build."
}

function Get-GuiLinkFlags {
  param([string]$Root)
  $guiRes = Get-GuiResourceLinkInput $Root
  return "/SUBSYSTEM:WINDOWS $guiRes user32.lib shell32.lib comctl32.lib"
}
