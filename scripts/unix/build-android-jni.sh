#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
min_api=26

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [[ -z "$sdk" && -f "$root/android/local.properties" ]]; then
  sdk="$(sed -n 's/^sdk.dir=//p' "$root/android/local.properties" | tr -d '\r' | sed 's/\\\\/\//g')"
fi
if [[ -z "$sdk" || ! -d "$sdk" ]]; then
  echo "Android SDK not found. Set ANDROID_HOME or android/local.properties sdk.dir." >&2
  exit 1
fi

ndk="${ANDROID_NDK_HOME:-}"
if [[ -z "$ndk" || ! -d "$ndk" ]]; then
  ndk="$(ls -d "$sdk"/ndk/* 2>/dev/null | sort -V | tail -n1 || true)"
fi
if [[ -z "$ndk" || ! -d "$ndk" ]]; then
  echo "Android NDK not found." >&2
  exit 1
fi
export ANDROID_NDK_HOME="$ndk"

prebuilt=""
for host in linux-x86_64 darwin-arm64 darwin-x86_64; do
  if [[ -d "$ndk/toolchains/llvm/prebuilt/$host/bin" ]]; then
    prebuilt="$ndk/toolchains/llvm/prebuilt/$host/bin"
    break
  fi
done
if [[ -z "$prebuilt" ]]; then
  echo "NDK llvm prebuilt missing." >&2
  exit 1
fi

jni="$root/android/app/src/main/jniLibs"
echo "  NDK $ndk"

build_one() {
  local rust="$1" abi="$2" triple="$3"
  local clang="$prebuilt/${triple}${min_api}-clang"
  local ar="$prebuilt/llvm-ar"
  [[ -x "$clang" ]] || { echo "Missing $clang" >&2; exit 1; }
  local rust_upper
  rust_upper="$(echo "$rust" | tr 'a-z' 'A-Z' | tr '-' '_')"
  export "CARGO_TARGET_${rust_upper}_LINKER=$clang"
  export "CC_${rust//-/_}=$clang"
  export "AR_${rust//-/_}=$ar"
  export "CARGO_TARGET_${rust_upper}_RUSTFLAGS=-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384"
  echo "  cargo build -p quark-android --target $rust --release"
  (cd "$root" && cargo build -p quark-android --target "$rust" --release)
  local src="$root/target/$rust/release/libquark.so"
  [[ -f "$src" ]] || { echo "missing $src" >&2; exit 1; }
  mkdir -p "$jni/$abi"
  cp "$src" "$jni/$abi/libquark.so"
  echo "  -> $jni/$abi/libquark.so"
}

build_one aarch64-linux-android arm64-v8a aarch64-linux-android
build_one x86_64-linux-android x86_64 x86_64-linux-android

if command -v python3 >/dev/null 2>&1; then
  echo "  Aligning JNI libs to 16 KiB pages..."
  python3 "$root/scripts/align_elf_16k.py" "$jni"
fi
