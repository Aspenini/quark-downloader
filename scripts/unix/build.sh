#!/usr/bin/env bash
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
build_dir="$root/build"
binary="$build_dir/quark-downloader"
gui_binary="$build_dir/quark-downloader-gui"

echo "quark-downloader (Unix build)"
echo ""

mkdir -p "$build_dir"

echo "  Compiling CLI + GUI dispatcher..."
(cd "$root" && cargo build --release -p quark-cli -p quark-gui)
cp "$root/target/release/quark-downloader" "$binary"
cp "$root/target/release/quark-downloader-gui" "$gui_binary"

if [[ "$(uname -s)" == "Linux" ]]; then
  echo "  Compiling GTK frontend..."
  (cd "$root" && cargo build --release -p quark-gui-gtk)
  cp "$root/target/release/quark-downloader-gui-gtk" "$build_dir/quark-downloader-gui-gtk"
fi

if [[ "$(uname -s)" == "Darwin" ]]; then
  if command -v swiftc >/dev/null 2>&1; then
    echo "  Compiling macOS GUI helper (swiftc)..."
    swiftc -O -o "$build_dir/quark-downloader-gui-appkit" "$root"/src/gui/macos/*.swift -framework AppKit
  else
    echo "  (swiftc not found; skipping native macOS UI)"
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
if [[ -x "$build_dir/quark-downloader-gui-gtk" ]]; then
  echo "  $build_dir/quark-downloader-gui-gtk"
fi
if [[ -x "$build_dir/quark-downloader-gui-appkit" ]]; then
  echo "  $build_dir/quark-downloader-gui-appkit"
fi
