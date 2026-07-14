#!/bin/bash
#
# Builds the netplane_client Rust static library for the PacketTunnel extension.
#
# Invoked by the PacketTunnel target's "Run Script" build phase (before the
# linker). Produces a (possibly universal) libnetplane_client.a that the target
# links via LIBRARY_SEARCH_PATHS + `-lnetplane_client`.
#
# Xcode-provided env used: SRCROOT (app/macos), ARCHS, CONFIGURATION.
set -euo pipefail

# app/macos -> app -> repo root
REPO_ROOT="$(cd "${SRCROOT}/../.." && pwd)"
OUT_DIR="${SRCROOT}/rust_artifacts"
mkdir -p "${OUT_DIR}"

# Ensure cargo is on PATH under Xcode's minimal environment.
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

if [ "${CONFIGURATION:-Debug}" = "Release" ]; then
  PROFILE_FLAG="--release"
  PROFILE_DIR="release"
else
  PROFILE_FLAG=""
  PROFILE_DIR="debug"
fi

LIBS=()
for arch in ${ARCHS:-arm64}; do
  case "$arch" in
    arm64) triple="aarch64-apple-darwin" ;;
    x86_64) triple="x86_64-apple-darwin" ;;
    *) echo "warning: unsupported arch $arch, skipping"; continue ;;
  esac

  rustup target add "$triple" >/dev/null 2>&1 || true

  echo "cargo build -p netplane_client --target $triple ${PROFILE_FLAG}"
  cargo build \
    --manifest-path "${REPO_ROOT}/Cargo.toml" \
    -p netplane_client \
    --target "$triple" \
    ${PROFILE_FLAG}

  LIBS+=("${REPO_ROOT}/target/${triple}/${PROFILE_DIR}/libnetplane_client.a")
done

if [ "${#LIBS[@]}" -eq 0 ]; then
  echo "error: no architectures built" >&2
  exit 1
fi

# Combine into a single artifact for the linker.
lipo -create "${LIBS[@]}" -output "${OUT_DIR}/libnetplane_client.a"
echo "wrote ${OUT_DIR}/libnetplane_client.a"
