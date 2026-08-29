#!/usr/bin/env bash
# Builds Quark Downloader.app and a distributable DMG into dist/.
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: build-dmg.sh only runs on macOS" >&2
  exit 1
fi

bash "$root/scripts/unix/build.sh"

if [[ ! -x "$root/build/quark-downloader-gui" ]]; then
  echo "error: quark-downloader-gui missing" >&2
  exit 1
fi

version="$(awk -F'"' '/^version = / {print $2; exit}' "$root/Cargo.toml")"
dist="$root/dist"
app="$dist/Quark Downloader.app"
macos_dir="$app/Contents/MacOS"
resources_dir="$app/Contents/Resources"

echo ""
echo "Assembling app bundle (v$version)..."
rm -rf "$dist"
mkdir -p "$macos_dir" "$resources_dir"

cp "$root/build/quark-downloader" \
   "$root/build/quark-downloader-gui" \
   "$macos_dir/"

echo "  Generating icon.icns..."
iconset="$(mktemp -d)/icon.iconset"
mkdir -p "$iconset"
for size in 16 32 64 128 256 512; do
  sips -z "$size" "$size" "$root/icons/icon.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
  sips -z "$((size * 2))" "$((size * 2))" "$root/icons/icon.png" \
    --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$resources_dir/icon.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>quark-downloader-gui</string>
    <key>CFBundleIdentifier</key>
    <string>com.aspenini.quark-downloader</string>
    <key>CFBundleName</key>
    <string>Quark Downloader</string>
    <key>CFBundleDisplayName</key>
    <string>Quark Downloader</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>CFBundleIconFile</key>
    <string>icon</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

sign_identity="${QUARK_MACOS_SIGN_IDENTITY:--}"
notary_profile="${QUARK_MACOS_NOTARY_PROFILE:-}"
if [[ -n "$notary_profile" && "$sign_identity" == "-" ]]; then
  echo "error: notarization requires QUARK_MACOS_SIGN_IDENTITY" >&2
  exit 1
fi

if [[ "$sign_identity" == "-" ]]; then
  echo "  Signing (ad-hoc development build)..."
  codesign --force --deep -s - "$app"
else
  echo "  Signing with Developer ID..."
  codesign --force --deep --options runtime --timestamp -s "$sign_identity" "$app"
fi

echo "  Creating DMG..."
staging="$(mktemp -d)"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
dmg="$dist/QuarkDownloader-$version.dmg"
hdiutil create -volname "Quark Downloader" -srcfolder "$staging" -ov -format UDZO "$dmg" >/dev/null
rm -rf "$staging"

if [[ "$sign_identity" != "-" ]]; then
  codesign --force --timestamp -s "$sign_identity" "$dmg"
fi
if [[ -n "$notary_profile" ]]; then
  echo "  Notarizing DMG..."
  xcrun notarytool submit "$dmg" --keychain-profile "$notary_profile" --wait
  xcrun stapler staple "$dmg"
  xcrun stapler validate "$dmg"
fi

echo ""
echo "Done:"
echo "  $app"
echo "  $dmg"
echo ""
if [[ "$sign_identity" == "-" ]]; then
  echo "Note: this release is ad-hoc signed. Downloaded copies may trigger Gatekeeper;"
  echo "right-click > Open the first time, or: xattr -dr com.apple.quarantine \"<app>\""
else
  echo "Developer ID signature and notarization complete."
fi
