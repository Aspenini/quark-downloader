#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
package="com.Aspenini.QuarkDownloader"
activity=".MainActivity"
preferred_avd="${ANDROID_AVD:-Quark}"
quark_image="system-images;android-35;google_apis;x86_64"

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [[ -z "$sdk" && -f "$root/android/local.properties" ]]; then
  sdk="$(sed -n 's/^sdk.dir=//p' "$root/android/local.properties" | tr -d '\r' | sed 's/\\\\/\//g')"
fi
if [[ -z "$sdk" || ! -d "$sdk" ]]; then
  echo "Android SDK not found. Set ANDROID_HOME or android/local.properties sdk.dir." >&2
  exit 1
fi

adb="$sdk/platform-tools/adb"
emulator="$sdk/emulator/emulator"
avdmanager="$sdk/cmdline-tools/latest/bin/avdmanager"
gradlew="$root/android/gradlew"
apk="$root/android/app/build/outputs/apk/debug/app-debug.apk"

echo "  SDK $sdk"
echo "  Building debug APK (arm64-v8a + x86_64)..."
(cd "$root/android" && ./gradlew :app:assembleDebug)
[[ -f "$apk" ]] || { echo "APK missing: $apk" >&2; exit 1; }

serial="$("$adb" devices | awk '/\tdevice$/{print $1; exit}')"
if [[ -z "$serial" ]]; then
  export SKIP_JDK_VERSION_CHECK=1
  avds="$("$emulator" -list-avds 2>/dev/null || true)"
  if ! grep -qx "$preferred_avd" <<<"$avds"; then
    if [[ "$preferred_avd" == "Quark" && -d "$sdk/system-images/android-35/google_apis/x86_64" ]]; then
      echo "  Creating AVD '$preferred_avd' (android-35 google_apis x86_64)..."
      echo no | "$avdmanager" create avd --name "$preferred_avd" --package "$quark_image" --device pixel_7 --force
    elif [[ -z "$avds" ]]; then
      echo "No Android Virtual Devices. Create one or set ANDROID_AVD." >&2
      exit 1
    else
      preferred_avd="$(printf '%s\n' "$avds" | head -n1)"
      echo "  Using AVD '$preferred_avd'"
    fi
  fi
  echo "  Starting emulator $preferred_avd..."
  "$emulator" -avd "$preferred_avd" -netdelay none -netspeed full -gpu auto >/dev/null 2>&1 &
  echo "  Waiting for emulator boot..."
  deadline=$((SECONDS + 300))
  serial=""
  while (( SECONDS < deadline )); do
    serial="$("$adb" devices | awk '/\tdevice$/{print $1; exit}')"
    if [[ -n "$serial" ]]; then
      boot="$("$adb" -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')"
      if [[ "$boot" == "1" ]]; then
        sleep 2
        break
      fi
    fi
    sleep 3
  done
  [[ -n "$serial" ]] || { echo "Emulator did not boot." >&2; exit 1; }
else
  echo "  Using existing device $serial"
fi

echo "  Installing $apk"
"$adb" -s "$serial" install -r -t "$apk"
echo "  Launching $package"
"$adb" -s "$serial" shell am start -n "$package/$activity"
echo "  Quark Downloader is running on $serial"
