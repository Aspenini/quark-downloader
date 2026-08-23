#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
props="$root/android/keystore.properties"
if [[ ! -f "$props" ]]; then
  cat >&2 <<'EOF'
Missing android/keystore.properties — needed to sign a release APK.

Create a keystore (store it somewhere safe, not in git):

  keytool -genkeypair -v \
    -keystore "$HOME/quark-release.jks" \
    -alias quark \
    -keyalg RSA -keysize 2048 -validity 10000

Then write android/keystore.properties (gitignored):

  storeFile=/home/YOU/quark-release.jks
  storePassword=YOUR_STORE_PASSWORD
  keyAlias=quark
  keyPassword=YOUR_KEY_PASSWORD

Copy android/keystore.properties.example to start.
EOF
  exit 1
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
echo "  $dest"
