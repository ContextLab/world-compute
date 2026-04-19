#!/usr/bin/env bash
# build-reproducible.sh — spec 005 US8 T114 / FR-043.
#
# Produces a deterministic release binary. Two independent invocations on
# identical source + toolchain MUST produce bit-identical output, enforced
# by .github/workflows/reproducible-build.yml diffing with `diffoscope`.
#
# Usage: ops/release/build-reproducible.sh [--features production]
#
# Output: target/release/worldcompute (bit-identical across runners).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

FEATURES=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --features) FEATURES="--features $2"; shift 2 ;;
        *) echo "usage: $0 [--features <list>]" >&2; exit 2 ;;
    esac
done

# Pin SOURCE_DATE_EPOCH to the commit timestamp (deterministic).
SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
export SOURCE_DATE_EPOCH

# Disable timestamp-dependent build steps.
export CARGO_NET_OFFLINE=false
export RUSTFLAGS="${RUSTFLAGS:-} -C strip=symbols --remap-path-prefix=${REPO_ROOT}=/build/worldcompute"

echo "=== Reproducible build ==="
echo "Repo root: $REPO_ROOT"
echo "Commit: $(git rev-parse HEAD)"
echo "SOURCE_DATE_EPOCH: $SOURCE_DATE_EPOCH"
echo "RUSTFLAGS: $RUSTFLAGS"
echo "Features: ${FEATURES:-(none)}"
echo

# Clean stale artifacts to guarantee a fresh build.
cargo clean

# Build the release binary.
# shellcheck disable=SC2086
cargo build --release --bin worldcompute $FEATURES

BINARY="$REPO_ROOT/target/release/worldcompute"
if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: binary not produced at $BINARY" >&2
    exit 1
fi

sha=$(sha256sum "$BINARY" | awk '{print $1}')
echo
echo "=== Build complete ==="
echo "Binary: $BINARY"
echo "Size: $(stat -c%s "$BINARY" 2>/dev/null || stat -f%z "$BINARY")"
echo "SHA-256: $sha"
