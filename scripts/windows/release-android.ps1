. (Join-Path $PSScriptRoot "common.ps1")

$ErrorActionPreference = "Stop"
$root = Get-ProjectRoot
$props = Join-Path $root "android\keystore.properties"

function Read-PlainSecret([string]$Prompt) {
  $secret = Read-Host $Prompt -AsSecureString
  $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secret)
  try {
    return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
  } finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer)
  }
}

if (-not (Test-Path $props)) {
  if (-not $env:QUARK_ANDROID_STORE_FILE) {
    $candidate = Join-Path $env:USERPROFILE "quark-release.jks"
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
      $env:QUARK_ANDROID_STORE_FILE = $candidate
    }
  }
  if (-not $env:QUARK_ANDROID_STORE_FILE -or -not (Test-Path -LiteralPath $env:QUARK_ANDROID_STORE_FILE -PathType Leaf)) {
    throw "Android release keystore not found. Set QUARK_ANDROID_STORE_FILE or create $env:USERPROFILE\quark-release.jks."
  }
  if (-not $env:QUARK_ANDROID_STORE_PASSWORD) {
    $env:QUARK_ANDROID_STORE_PASSWORD = Read-PlainSecret "Android keystore password"
  }
  if (-not $env:QUARK_ANDROID_STORE_PASSWORD) {
    throw "Android keystore password cannot be empty."
  }
  if (-not $env:QUARK_ANDROID_KEY_ALIAS) {
    $env:QUARK_ANDROID_KEY_ALIAS = "quark"
  }
  if (-not $env:QUARK_ANDROID_KEY_PASSWORD) {
    $env:QUARK_ANDROID_KEY_PASSWORD = $env:QUARK_ANDROID_STORE_PASSWORD
  }
  Write-Host "  Using Android keystore: $env:QUARK_ANDROID_STORE_FILE"
}

$gradlew = Join-Path $root "android\gradlew.bat"
Write-Host "  Building signed release APK..."
Push-Location (Join-Path $root "android")
try {
  Invoke-Checked { & $gradlew :app:assembleRelease }
} finally {
  Pop-Location
}

$apk = Join-Path $root "android\app\build\outputs\apk\release\app-release.apk"
if (-not (Test-Path $apk)) {
  throw "Release APK missing: $apk"
}

$gradle = Get-Content (Join-Path $root "android\app\build.gradle.kts") -Raw
if ($gradle -match 'versionName\s*=\s*"([^"]+)"') {
  $version = $Matches[1]
} else {
  $version = "dev"
}

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$dest = Join-Path $dist "quark-downloader-$version-android.apk"
Copy-Item $apk $dest -Force

$sdk = @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $sdk) { throw "Android SDK not found for APK verification." }
$buildTools = Get-ChildItem (Join-Path $sdk "build-tools") -Directory |
  Sort-Object { [version]$_.Name } -Descending |
  Select-Object -First 1
if (-not $buildTools) { throw "Android build-tools not found for APK verification." }
$apksigner = Join-Path $buildTools.FullName "apksigner.bat"
$zipalign = Join-Path $buildTools.FullName "zipalign.exe"
Invoke-Checked { & $apksigner verify --verbose --print-certs $dest }
Invoke-Checked { & $zipalign -c -P 16 -v 4 $dest }
Write-Host "  $dest"
