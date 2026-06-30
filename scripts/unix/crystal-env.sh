#!/usr/bin/env bash
# Crystal links OpenSSL through pkg-config. Homebrew kegs such as openssl@3 are
# often outside the default search path, which can break linking with EVP symbols.

# Crystal on macOS may default to the LLVM lld linker, but Homebrew's lld cannot
# link Mach-O for current macOS ("This version of lld does not support linking
# for platform macOS"), so every crystal build/run fails. Point Crystal at
# Apple's system linker instead. Recipes/scripts append $QUARK_CRYSTAL_LINK_FLAGS
# to their crystal invocations. Override by exporting it yourself before running.
if [[ "$(uname -s)" == "Darwin" && -z "${QUARK_CRYSTAL_LINK_FLAGS+x}" ]]; then
  export QUARK_CRYSTAL_LINK_FLAGS="--link-flags=-fuse-ld=/usr/bin/ld"
fi

if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists libssl libcrypto 2>/dev/null; then
  return 0 2>/dev/null || exit 0
fi

openssl_pc_dirs=()
if command -v brew >/dev/null 2>&1; then
  for formula in openssl@3 openssl; do
    prefix="$(brew --prefix "$formula" 2>/dev/null || true)"
    if [[ -n "$prefix" && -d "$prefix/lib/pkgconfig" ]]; then
      openssl_pc_dirs+=("$prefix/lib/pkgconfig")
    fi
  done
fi
openssl_pc_dirs+=(
  "/opt/homebrew/opt/openssl@3/lib/pkgconfig"
  "/opt/homebrew/opt/openssl/lib/pkgconfig"
  "/usr/local/opt/openssl@3/lib/pkgconfig"
  "/usr/local/opt/openssl/lib/pkgconfig"
)

for dir in "${openssl_pc_dirs[@]}"; do
  if [[ -f "$dir/libssl.pc" ]]; then
    export PKG_CONFIG_PATH="${dir}${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
    break
  fi
done
