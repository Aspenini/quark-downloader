#!/usr/bin/env bash
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
build_dir="$root/build"
binary="$build_dir/quark-downloader"
gui_binary="$build_dir/quark-downloader-gui"

echo "quark-downloader (Unix build)"
echo ""

mkdir -p "$build_dir"

echo "  Compiling CLI + GUI..."
(cd "$root" && cargo build --release -p quark-cli -p quark-gui-dispatch)
cp "$root/target/release/quark-downloader" "$binary"
cp "$root/target/release/quark-downloader-gui" "$gui_binary"

if [[ "$(uname -s)" == "Linux" ]]; then
  have_pkg() { command -v pkg-config >/dev/null && pkg-config --exists "$1"; }
  mkdir -p "$build_dir/qml"
  cp "$root"/src/gui/qt/*.qml "$build_dir/qml/"
  echo "  qml/"
  if ! have_pkg Qt6Quick || ! have_pkg Qt6Qml; then
    echo "  (Qt UI not linked — no Qt6Quick.pc)"
    echo "    Arch:  sudo pacman -S --needed qt6-declarative pkgconf"
    echo "    Debian/Ubuntu: sudo apt install qt6-declarative-dev"
  elif [[ "${XDG_CURRENT_DESKTOP:-}" == *COSMIC* ]]; then
    echo "  Tip: install CuteCosmic for native COSMIC colors, fonts, icons, and dialogs."
  fi
fi

echo "  UPX (CLI only)..."
if [[ "$(uname -s)" == "Darwin" ]]; then
  echo "  (upx skipped on macOS)"
elif command -v upx >/dev/null 2>&1; then
  if upx --best --lzma "$binary"; then
    :
  else
    echo "  (upx failed, skipping)"
  fi
else
  echo "  (upx not found, skipping)"
fi

echo ""
echo "Done:"
echo "  $binary"
echo "  $gui_binary"
if [[ -d "$build_dir/qml" ]]; then
  echo "  $build_dir/qml"
fi
