#!/usr/bin/env bash
# Builds Quark Downloader.app and a distributable DMG into dist/.
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: build-dmg.sh only runs on macOS" >&2
  exit 1
fi

bash "$root/scripts/unix/build.sh"

if [[ ! -x "$root/build/quark-downloader-gui-helper" ]]; then
  echo "error: native macOS helper missing (swiftc required to build the app bundle)" >&2
  exit 1
fi

version="$(awk '/^version:/ {print $2}' "$root/shard.yml")"
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
   "$root/build/quark-downloader-gui-helper" \
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

echo "  Signing (ad-hoc)..."
codesign --force --deep -s - "$app"

echo "  Creating DMG..."
staging="$(mktemp -d)"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
dmg="$dist/QuarkDownloader-$version.dmg"
hdiutil create -volname "Quark Downloader" -srcfolder "$staging" -ov -format UDZO "$dmg" >/dev/null
rm -rf "$staging"

echo ""
echo "Done:"
echo "  $app"
echo "  $dmg"
echo ""
echo "Note: the app is ad-hoc signed. Downloaded copies hit Gatekeeper;"
echo "right-click > Open the first time, or: xattr -dr com.apple.quarantine \"<app>\""
