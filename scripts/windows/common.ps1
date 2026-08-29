$ErrorActionPreference = "Stop"

function Get-ProjectRoot {
  param([string]$ScriptRoot = $PSScriptRoot)
  return (Resolve-Path (Join-Path $ScriptRoot "..\..")).Path
}

function Get-ProjectVersion {
  param([string]$Root)
  $cargo = Get-Content (Join-Path $Root "Cargo.toml") -Raw
  if ($cargo -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Could not read workspace version."
  }
  return $Matches[1]
}

function Get-WindowsPackageDir {
  param([string]$Root, [string]$Version = (Get-ProjectVersion $Root))
  return (Join-Path $Root "target\package\quark-downloader-$Version-windows-portable")
}

function Initialize-WindowsPackageDir {
  param([string]$Root)
  $packageDir = Get-WindowsPackageDir $Root
  $rootPrefix = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
  $resolved = [IO.Path]::GetFullPath($packageDir)
  if (-not $resolved.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to replace package staging outside the repository: $resolved"
  }
  if (Test-Path -LiteralPath $resolved) {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path $resolved | Out-Null
  return $resolved
}

function Initialize-DistDir {
  param([string]$Root)
  $dist = Join-Path $Root "dist"
  New-Item -ItemType Directory -Force -Path $dist | Out-Null
  return $dist
}

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)]
    [scriptblock]$Command
  )
  & $Command
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
