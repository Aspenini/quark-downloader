#!/usr/bin/env bash
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=crystal-env.sh
source "$root/scripts/unix/crystal-env.sh"

build_dir="$root/build"
binary="$build_dir/quark-downloader"
gui_binary="$build_dir/quark-downloader-gui"

echo "quark-downloader (Unix build)"
echo ""

mkdir -p "$build_dir"

echo "  Compiling CLI..."
crystal build --release "$root/src/quark-downloader.cr" -o "$binary"

echo "  Compiling GUI..."
crystal build --release "$root/src/gui/quark-downloader-gui.cr" -o "$gui_binary"
cp "$root/src/gui/quark-downloader-gui.tcl" "$build_dir/"

if [[ "$(uname -s)" == "Darwin" ]]; then
  if command -v swiftc >/dev/null 2>&1; then
    echo "  Compiling macOS GUI helper (swiftc)..."
    swiftc -O -o "$build_dir/quark-downloader-gui-helper" "$root"/src/gui/macos/*.swift -framework AppKit
  else
    echo "  (swiftc not found; skipping native macOS UI - the GUI will fall back to Tk)"
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
if [[ -x "$build_dir/quark-downloader-gui-helper" ]]; then
  echo "  $build_dir/quark-downloader-gui-helper"
fi
