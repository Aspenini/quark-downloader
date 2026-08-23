. (Join-Path $PSScriptRoot "common.ps1")

$ErrorActionPreference = "Stop"
$root = Get-ProjectRoot
$package = "com.Aspenini.QuarkDownloader"
$activity = ".MainActivity"
$preferredAvd = if ($env:ANDROID_AVD) { $env:ANDROID_AVD } else { "Quark" }
$quarkImage = "system-images;android-35;google_apis;x86_64"

function Get-AndroidSdk {
  foreach ($key in @("ANDROID_HOME", "ANDROID_SDK_ROOT")) {
    $value = [Environment]::GetEnvironmentVariable($key)
    if ($value -and (Test-Path $value)) {
      return $value
    }
  }
  $props = Join-Path $root "android\local.properties"
  if (Test-Path $props) {
    foreach ($line in Get-Content $props) {
      if ($line -match '^\s*sdk\.dir=(.+)$') {
        $dir = $Matches[1].Trim().Replace("\\", "\").Trim('"')
        if (Test-Path $dir) {
          return $dir
        }
      }
    }
  }
  throw "Android SDK not found. Set ANDROID_HOME or android/local.properties sdk.dir."
}

function Get-SdkTool {
  param([string]$Sdk, [string]$Rel)
  $path = Join-Path $Sdk $Rel
  if (-not (Test-Path $path)) {
    throw "Missing SDK tool: $path"
  }
  return $path
}

function Get-AdbDevices([string]$Adb) {
  & $Adb devices |
    Select-Object -Skip 1 |
    Where-Object { $_ -match '\tdevice$' } |
    ForEach-Object { ($_ -split '\s+')[0] }
}

function Get-AvdNames([string]$Emulator) {
  @(& $Emulator -list-avds 2>$null | ForEach-Object { $_.Trim() } | Where-Object { $_ })
}

function Wait-Boot {
  param([string]$Adb, [int]$TimeoutSec = 300)
  Write-Host "  Waiting for emulator boot (up to ${TimeoutSec}s)..."
  $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while ((Get-Date) -lt $deadline) {
    $serial = @(Get-AdbDevices $Adb) | Select-Object -First 1
    if ($serial) {
      $boot = (& $Adb -s $serial shell getprop sys.boot_completed 2>$null | Out-String).Trim()
      if ($boot -eq "1") {
        Start-Sleep -Seconds 2
        return $serial
      }
    }
    Start-Sleep -Seconds 3
  }
  throw "Emulator did not finish booting within ${TimeoutSec}s"
}

function New-QuarkAvdIfNeeded {
  param([string]$Sdk, [string]$Emulator, [string]$AvdManager)
  $avds = Get-AvdNames $Emulator
  if ($avds -contains $preferredAvd) {
    return $preferredAvd
  }
  $imageDir = Join-Path $Sdk "system-images\android-35\google_apis\x86_64"
  if ($preferredAvd -eq "Quark" -and (Test-Path $imageDir)) {
    Write-Host "  Creating AVD '$preferredAvd' (android-35 google_apis x86_64, 4 KB pages)..."
    $env:SKIP_JDK_VERSION_CHECK = "1"
    "no" | & $AvdManager create avd --name $preferredAvd --package $quarkImage --device pixel_7 --force
    if ($LASTEXITCODE -ne 0) {
      throw "avdmanager failed to create $preferredAvd"
    }
    $avds = Get-AvdNames $Emulator
    if ($avds -notcontains $preferredAvd) {
      throw "AVD '$preferredAvd' was not created. Set SKIP_JDK_VERSION_CHECK=1 and run avdmanager by hand."
    }
    return $preferredAvd
  }
  if ($avds.Count -eq 0) {
    throw "No Android Virtual Devices. Install a system image and create an AVD, or set ANDROID_AVD."
  }
  $fallback = $avds[0]
  if ($fallback -ne $preferredAvd) {
    Write-Host "  AVD '$preferredAvd' missing; using '$fallback'."
    Write-Host "  Pixel 16 KB images often cannot load youtubedl-android's Python. Prefer AVD Quark."
  }
  return $fallback
}

$sdk = Get-AndroidSdk
$env:SKIP_JDK_VERSION_CHECK = "1"
$adb = Get-SdkTool $sdk "platform-tools\adb.exe"
$emulator = Get-SdkTool $sdk "emulator\emulator.exe"
$avdManager = Get-SdkTool $sdk "cmdline-tools\latest\bin\avdmanager.bat"
$gradlew = Join-Path $root "android\gradlew.bat"
$apk = Join-Path $root "android\app\build\outputs\apk\debug\app-debug.apk"

Write-Host "  SDK $sdk"
Write-Host "  Building debug APK (arm64-v8a + x86_64)..."
Push-Location (Join-Path $root "android")
try {
  Invoke-Checked { & $gradlew :app:assembleDebug }
} finally {
  Pop-Location
}
if (-not (Test-Path $apk)) {
  throw "APK missing: $apk"
}

$serial = @(Get-AdbDevices $adb) | Select-Object -First 1
if (-not $serial) {
  $avd = New-QuarkAvdIfNeeded -Sdk $sdk -Emulator $emulator -AvdManager $avdManager
  Write-Host "  Starting emulator $avd..."
  $emuArgs = @("-avd", $avd, "-netdelay", "none", "-netspeed", "full", "-gpu", "auto")
  Start-Process -FilePath $emulator -ArgumentList $emuArgs | Out-Null
  $serial = Wait-Boot -Adb $adb
  if (-not $serial) {
    throw "Emulator started but adb has no device"
  }
} else {
  Write-Host "  Using existing device $serial"
}

Write-Host "  Installing $apk"
Invoke-Checked { & $adb -s $serial install -r -t $apk }
Write-Host "  Launching $package"
Invoke-Checked { & $adb -s $serial shell am start -n "$package/$activity" }
Write-Host "  Quark Downloader is running on $serial"
