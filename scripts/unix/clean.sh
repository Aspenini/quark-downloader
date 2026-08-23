#!/usr/bin/env bash
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
rm -rf "$root/build" "$root/packaging/output" "$root/target"
echo "Cleaned build/, packaging/output/, and target/"
