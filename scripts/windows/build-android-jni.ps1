. (Join-Path $PSScriptRoot "common.ps1")

$ErrorActionPreference = "Stop"
$root = Get-ProjectRoot
$minApi = 26

function Get-AndroidSdk {
  foreach ($key in @("ANDROID_HOME", "ANDROID_SDK_ROOT")) {
    $value = [Environment]::GetEnvironmentVariable($key)
    if ($value -and (Test-Path $value)) { return $value }
  }
  $props = Join-Path $root "android\local.properties"
  if (Test-Path $props) {
    foreach ($line in Get-Content $props) {
      if ($line -match '^\s*sdk\.dir=(.+)$') {
        $dir = $Matches[1].Trim().Replace("\\", "\").Trim('"')
        if (Test-Path $dir) { return $dir }
      }
    }
  }
  throw "Android SDK not found"
}

$sdk = Get-AndroidSdk
$ndk = $env:ANDROID_NDK_HOME
if (-not $ndk -or -not (Test-Path $ndk)) {
  $ndkRoot = Join-Path $sdk "ndk"
  $ndk = Get-ChildItem $ndkRoot -Directory | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $ndk) { throw "Android NDK not found" }
$env:ANDROID_NDK_HOME = $ndk

$prebuilt = Join-Path $ndk "toolchains\llvm\prebuilt\windows-x86_64"
$bin = Join-Path $prebuilt "bin"
if (-not (Test-Path $bin)) { throw "NDK llvm prebuilt missing: $bin" }

$jni = Join-Path $root "android\app\src\main\jniLibs"
$targets = @(
  @{ rust = "aarch64-linux-android"; abi = "arm64-v8a"; triple = "aarch64-linux-android" },
  @{ rust = "x86_64-linux-android"; abi = "x86_64"; triple = "x86_64-linux-android" }
)

Write-Host "  NDK $ndk"
foreach ($t in $targets) {
  $clang = Join-Path $bin "$($t.triple)$minApi-clang.cmd"
  $ar = Join-Path $bin "llvm-ar.exe"
  if (-not (Test-Path $clang)) { throw "Missing $clang" }
  $rustUpper = $t.rust.ToUpper().Replace("-", "_")
  Set-Item -Path "env:CARGO_TARGET_${rustUpper}_LINKER" -Value $clang
  Set-Item -Path "env:CC_$($t.rust.Replace('-','_'))" -Value $clang
  Set-Item -Path "env:AR_$($t.rust.Replace('-','_'))" -Value $ar
  Set-Item -Path "env:CARGO_TARGET_${rustUpper}_RUSTFLAGS" -Value "-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384"
  Write-Host "  cargo build -p quark-android --target $($t.rust) --release"
  Push-Location $root
  try {
    cargo build -p quark-android --target $t.rust --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  } finally {
    Pop-Location
  }
  $src = Join-Path $root "target\$($t.rust)\release\libquark.so"
  if (-not (Test-Path $src)) { throw "missing $src" }
  $destDir = Join-Path $jni $t.abi
  New-Item -ItemType Directory -Force -Path $destDir | Out-Null
  Copy-Item $src (Join-Path $destDir "libquark.so") -Force
  Write-Host "  -> $destDir\libquark.so"
}

$align = Join-Path $root "scripts\align_elf_16k.py"
if (Test-Path $align) {
  Write-Host "  Aligning JNI libs to 16 KiB pages..."
  $py = Get-Command py -ErrorAction SilentlyContinue
  if ($py) {
    & py -3 $align $jni
  } else {
    python $align $jni
  }
}
