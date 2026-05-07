#!/usr/bin/env bash
# sign-release.sh — spec 005 US8 T114 / FR-044.
#
# Produce a detached Ed25519 signature for a release artifact using the
# offline release private key (held by the release engineer, never in CI).
#
# Usage:
#   ops/release/sign-release.sh <artifact> <release-private-key.pem>
#
# Writes: <artifact>.sig  (detached Ed25519 signature, 64 bytes, base64-encoded)
#
# The corresponding public key is pinned in ops/release/verify-release.sh
# as RELEASE_PUBLIC_KEY_HEX. When the key rotates, update both files
# atomically and cut a new release tag.
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <artifact> <release-private-key.pem>" >&2
    exit 2
fi

ARTIFACT="$1"
KEY="$2"

if [[ ! -f "$ARTIFACT" ]]; then
    echo "ERROR: artifact not found: $ARTIFACT" >&2
    exit 2
fi
if [[ ! -f "$KEY" ]]; then
    echo "ERROR: key not found: $KEY" >&2
    exit 2
fi

SIG_FILE="${ARTIFACT}.sig"

echo "Signing $ARTIFACT with $KEY"
# openssl supports Ed25519 via pkeyutl
openssl pkeyutl -sign -inkey "$KEY" -rawin -in "$ARTIFACT" -out "${SIG_FILE}.raw"
base64 < "${SIG_FILE}.raw" > "$SIG_FILE"
rm -f "${SIG_FILE}.raw"

echo "Signature written to $SIG_FILE"
echo "Size: $(wc -c < "$SIG_FILE") bytes (base64-encoded, typical ~88 for 64-byte Ed25519)"
