#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
if [[ "$(uname -s)" != "Linux" ]]; then
  echo "error: package-release.sh only runs on Linux" >&2
  exit 1
fi

bash "$root/scripts/unix/build.sh"

version="$(awk -F'"' '/^version = / {print $2; exit}' "$root/Cargo.toml")"
arch="$(uname -m)"
case "$arch" in
  amd64) arch="x86_64" ;;
  arm64) arch="aarch64" ;;
esac
package_name="quark-downloader-$version-linux-$arch"
package_parent="$root/target/package"
package_dir="$package_parent/$package_name"
archive="$root/dist/$package_name.tar.gz"

[[ -x "$package_dir/quark-downloader" ]] || {
  echo "error: Linux package staging is missing: $package_dir" >&2
  exit 1
}
mkdir -p "$root/dist"
tar -C "$package_parent" -czf "$archive" "$package_name"

echo ""
echo "Linux release ready:"
echo "  $archive"
