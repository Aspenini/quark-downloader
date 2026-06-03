#!/usr/bin/env bash
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=crystal-env.sh
source "$root/scripts/crystal-env.sh"
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

echo "  UPX (CLI only)..."
if command -v upx >/dev/null 2>&1; then
  upx --best --lzma "$binary"
else
  echo "  (upx not found, skipping)"
fi

echo ""
echo "Done:"
echo "  $binary"
echo "  $gui_binary"
