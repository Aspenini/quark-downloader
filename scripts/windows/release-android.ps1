. (Join-Path $PSScriptRoot "common.ps1")

$ErrorActionPreference = "Stop"
$root = Get-ProjectRoot
$props = Join-Path $root "android\keystore.properties"
if (-not (Test-Path $props)) {
  Write-Host "Missing android/keystore.properties (needed to sign a release APK)."
  Write-Host "Create a keystore (keep it off git) with:"
  Write-Host '  keytool -genkeypair -v -keystore $env:USERPROFILE\quark-release.jks -alias quark -keyalg RSA -keysize 2048 -validity 10000'
  Write-Host "Then copy android/keystore.properties.example to android/keystore.properties and fill in storeFile, passwords, and keyAlias=quark."
  Write-Host "Full steps: android/README.md"
  exit 1
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
Write-Host "  $dest"
