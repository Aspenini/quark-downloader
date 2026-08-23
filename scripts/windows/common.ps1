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
