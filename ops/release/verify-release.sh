#!/usr/bin/env bash
# verify-release.sh — spec 005 US8 T114 / FR-044.
#
# Verify a release artifact against its detached Ed25519 signature using
# the pinned release public key. Any operator downloading a World Compute
# release binary should run this before trusting it.
#
# Usage: ops/release/verify-release.sh <artifact> <artifact.sig>
#
# Exit codes:
#   0 — signature verifies
#   1 — signature does NOT verify
#   2 — missing input or openssl not available
set -euo pipefail

# RELEASE_PUBLIC_KEY_HEX: Ed25519 public key in hex (32 bytes = 64 hex chars).
# This is pinned at release-cut time and shipped with every binary; operators
# verify the artifact against this hard-coded value.
#
# Awaiting the first signed release:
#   cargo run --bin worldcompute-release-keygen (TBD) will produce the keypair
#   and print the public key hex. Until then, this is the zero sentinel and
#   verify-release.sh will report all signatures as invalid.
RELEASE_PUBLIC_KEY_HEX="0000000000000000000000000000000000000000000000000000000000000000"

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <artifact> <artifact.sig>" >&2
    exit 2
fi

ARTIFACT="$1"
SIG_B64="$2"

if [[ ! -f "$ARTIFACT" ]]; then
    echo "ERROR: artifact not found: $ARTIFACT" >&2
    exit 2
fi
if [[ ! -f "$SIG_B64" ]]; then
    echo "ERROR: signature not found: $SIG_B64" >&2
    exit 2
fi

if ! command -v openssl >/dev/null; then
    echo "ERROR: openssl not available" >&2
    exit 2
fi

if [[ "$RELEASE_PUBLIC_KEY_HEX" == "0000000000000000000000000000000000000000000000000000000000000000" ]]; then
    echo "ERROR: RELEASE_PUBLIC_KEY_HEX is the zero sentinel — no release has been signed yet." >&2
    echo "       The first signed release will update this value atomically with sign-release.sh." >&2
    exit 1
fi

# Decode signature
SIG_RAW=$(mktemp)
trap 'rm -f "$SIG_RAW" "${SIG_RAW}.pub"' EXIT

base64 -d < "$SIG_B64" > "$SIG_RAW"

# Reconstruct the public key in PEM form for openssl pkeyutl.
# Ed25519 raw 32-byte public keys embedded in SPKI are:
#   30 2a 30 05 06 03 2b 65 70 03 21 00 || <32 bytes>
PUB_DER="${SIG_RAW}.pub.der"
{
    printf '\x30\x2a\x30\x05\x06\x03\x2b\x65\x70\x03\x21\x00'
    printf '%s' "$RELEASE_PUBLIC_KEY_HEX" | xxd -r -p
} > "$PUB_DER"

PUB_PEM="${SIG_RAW}.pub"
openssl pkey -pubin -in "$PUB_DER" -inform DER -out "$PUB_PEM" 2>/dev/null

# Verify
if openssl pkeyutl -verify -pubin -inkey "$PUB_PEM" -rawin -in "$ARTIFACT" -sigfile "$SIG_RAW" >/dev/null 2>&1; then
    echo "✅ Signature verified for $ARTIFACT"
    exit 0
else
    echo "❌ Signature FAILED to verify for $ARTIFACT" >&2
    exit 1
fi
