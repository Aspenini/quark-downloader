#!/usr/bin/env bash
set -eu

root="$(cd "$(dirname "$0")/../.." && pwd)"
if [[ -x "$root/android/gradlew" && -d "$root/android/.gradle" ]]; then
  "$root/android/gradlew" -p "$root/android" --stop >/dev/null
fi
rm -rf \
  "$root/target" \
  "$root/android/.gradle" \
  "$root/android/.cxx" \
  "$root/android/build" \
  "$root/android/app/build" \
  "$root/android/app/src/main/jniLibs"
echo "Cleaned all build and package intermediates (dist/ preserved)"
