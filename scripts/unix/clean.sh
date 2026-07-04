#!/usr/bin/env bash
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
rm -rf "$root/build" "$root/packaging/output"
echo "Cleaned build/ and packaging/output/"
