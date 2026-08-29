#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
props="$root/android/keystore.properties"
if [[ ! -f "$props" ]]; then
  store_file="${QUARK_ANDROID_STORE_FILE:-$HOME/quark-release.jks}"
  [[ -f "$store_file" ]] || {
    echo "Android release keystore not found. Set QUARK_ANDROID_STORE_FILE or create $HOME/quark-release.jks." >&2
    exit 1
  }
  export QUARK_ANDROID_STORE_FILE="$store_file"
  if [[ -z "${QUARK_ANDROID_STORE_PASSWORD:-}" ]]; then
    read -r -s -p "Android keystore password: " QUARK_ANDROID_STORE_PASSWORD
    echo ""
    export QUARK_ANDROID_STORE_PASSWORD
  fi
  [[ -n "$QUARK_ANDROID_STORE_PASSWORD" ]] || { echo "Android keystore password cannot be empty." >&2; exit 1; }
  export QUARK_ANDROID_KEY_ALIAS="${QUARK_ANDROID_KEY_ALIAS:-quark}"
  export QUARK_ANDROID_KEY_PASSWORD="${QUARK_ANDROID_KEY_PASSWORD:-$QUARK_ANDROID_STORE_PASSWORD}"
  echo "  Using Android keystore: $QUARK_ANDROID_STORE_FILE"
fi

echo "  Building signed release APK..."
(cd "$root/android" && ./gradlew :app:assembleRelease)
apk="$root/android/app/build/outputs/apk/release/app-release.apk"
[[ -f "$apk" ]] || { echo "Release APK missing: $apk" >&2; exit 1; }

version="$(sed -n 's/.*versionName = "\([^"]*\)".*/\1/p' "$root/android/app/build.gradle.kts" | head -n1)"
version="${version:-dev}"
mkdir -p "$root/dist"
dest="$root/dist/quark-downloader-${version}-android.apk"
cp "$apk" "$dest"

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
[[ -n "$sdk" && -d "$sdk" ]] || { echo "Android SDK not found for APK verification." >&2; exit 1; }
build_tools="$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n1)"
[[ -n "$build_tools" ]] || { echo "Android build-tools not found for APK verification." >&2; exit 1; }
"$build_tools/apksigner" verify --verbose --print-certs "$dest"
"$build_tools/zipalign" -c -P 16 -v 4 "$dest"
echo "  $dest"
