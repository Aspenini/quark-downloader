#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
version="$(awk -F'"' '/^version = / {print $2; exit}' "$root/Cargo.toml")"
system="$(uname -s)"
arch="$(uname -m)"
case "$arch" in
  amd64) arch="x86_64" ;;
  arm64) arch="aarch64" ;;
esac

case "$system" in
  Darwin) package_dir="$root/target/package/macos-binaries" ;;
  Linux) package_dir="$root/target/package/quark-downloader-$version-linux-$arch" ;;
  *) echo "error: unsupported Unix platform: $system" >&2; exit 1 ;;
esac
case "$package_dir" in
  "$root"/target/package/*) ;;
  *) echo "error: invalid package staging path: $package_dir" >&2; exit 1 ;;
esac

echo "quark-downloader ($system release build)"
echo ""

rm -rf "$package_dir"
mkdir -p "$package_dir"

echo "  Compiling CLI + GUI..."
(cd "$root" && cargo build --release -p quark-cli -p quark-gui-dispatch)
cp "$root/target/release/quark-downloader" \
   "$root/target/release/quark-downloader-gui" \
   "$root/LICENSE" \
   "$root/README.md" \
   "$package_dir/"

if [[ "$system" == "Linux" ]]; then
  have_pkg() { command -v pkg-config >/dev/null && pkg-config --exists "$1"; }
  mkdir -p "$package_dir/qml"
  cp "$root"/src/gui/qt/*.qml "$package_dir/qml/"
  if ! have_pkg Qt6Quick || ! have_pkg Qt6Qml; then
    echo "  (Qt UI not linked — no Qt6Quick.pc)"
    echo "    Arch:  sudo pacman -S --needed qt6-declarative pkgconf"
    echo "    Debian/Ubuntu: sudo apt install qt6-declarative-dev"
  elif [[ "${XDG_CURRENT_DESKTOP:-}" == *COSMIC* ]]; then
    echo "  Tip: install CuteCosmic for native COSMIC colors, fonts, icons, and dialogs."
  fi
fi

echo "  UPX (CLI only)..."
if [[ "$system" == "Darwin" ]]; then
  echo "  (upx skipped on macOS)"
elif command -v upx >/dev/null 2>&1; then
  upx --best --lzma "$package_dir/quark-downloader" || echo "  (upx failed, skipping)"
else
  echo "  (upx not found, skipping)"
fi

echo ""
echo "Staged package:"
echo "  $package_dir"
echo "Final release files are written only by a platform release command into dist/."
