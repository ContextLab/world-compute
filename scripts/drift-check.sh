#!/usr/bin/env bash
# drift-check.sh — spec 005 US2 T036 / FR-011a.
#
# Weekly CI refetch of pinned AMD ARK + Intel DCAP + Sigstore Rekor values
# from authoritative upstream endpoints, diffing against the in-tree pins.
# On mismatch, opens a repository issue tagged `drift-check` when running in CI
# (GITHUB_TOKEN available). Locally, just reports diffs and exits non-zero.
#
# Usage:
#   scripts/drift-check.sh              # check and report
#   scripts/drift-check.sh --open-issue # open an issue on mismatch (CI)
#
# Exit codes:
#   0  — all pins match upstream
#   1  — at least one pin mismatches; diff printed to stderr
#   2  — upstream fetch failure (network or temporary endpoint outage)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

OPEN_ISSUE="${1:-}"

# In-tree pin locations — we grep the Rust source for the canonical 32-byte
# fingerprint constants. Format is the Rust byte array notation `0xXX, 0xXX, ...`.
ATTESTATION_RS="src/verification/attestation.rs"
TRANSPARENCY_RS="src/ledger/transparency.rs"

# Helper: extract a 32-byte fingerprint constant's value from a Rust source file.
# Arg 1: name of the const (regex-matched against `pub const <name>: [u8; 32] = [`)
# Arg 2: path to source file.
# Emits the 64-char lowercase hex string to stdout.
extract_pin() {
    local name="$1" file="$2"
    python3 - "$name" "$file" <<'PY'
import re, sys
name, path = sys.argv[1], sys.argv[2]
text = open(path).read()
# Find `pub const <name>: [u8; 32] = [ ... ];` block and extract bytes
m = re.search(
    r'pub\s+const\s+' + re.escape(name) + r'\s*:\s*\[u8;\s*32\]\s*=\s*\[([^\]]+)\]',
    text, re.DOTALL
)
if not m:
    print(f"ERROR: could not find {name} in {path}", file=sys.stderr)
    sys.exit(3)
body = m.group(1)
bytes_hex = re.findall(r'0x([0-9a-fA-F]{2})', body)
if len(bytes_hex) != 32:
    print(f"ERROR: expected 32 bytes in {name}, found {len(bytes_hex)}", file=sys.stderr)
    sys.exit(3)
print("".join(b.lower() for b in bytes_hex))
PY
}

# Helper: fetch DER from URL and compute SHA-256.
fetch_sha256_der() {
    local url="$1"
    local tmp
    tmp=$(mktemp)
    if ! curl -fsSL "$url" -o "$tmp"; then
        rm -f "$tmp"
        echo "FETCH_FAIL"
        return 0
    fi
    openssl dgst -sha256 "$tmp" | awk '{print $NF}'
    rm -f "$tmp"
}

# Helper: for PEM cert chains, extract the root (self-signed) cert's DER and hash it.
fetch_sha256_pem_root() {
    local url="$1"
    local tmpchain tmpdir
    tmpchain=$(mktemp)
    tmpdir=$(mktemp -d)
    if ! curl -fsSL "$url" -o "$tmpchain"; then
        rm -rf "$tmpchain" "$tmpdir"
        echo "FETCH_FAIL"
        return 0
    fi
    # Split the PEM chain into individual cert files.
    awk -v d="$tmpdir" 'BEGIN{n=0} /-----BEGIN CERTIFICATE-----/{n++; f=d"/cert_"n".pem"} {if(n>0) print > f}' "$tmpchain"
    # Find the self-signed root: the cert whose Subject == Issuer.
    local root=""
    for f in "$tmpdir"/cert_*.pem; do
        subj=$(openssl x509 -in "$f" -noout -subject 2>/dev/null | sed 's/^subject=//')
        issuer=$(openssl x509 -in "$f" -noout -issuer 2>/dev/null | sed 's/^issuer=//')
        if [ "$subj" = "$issuer" ]; then
            root="$f"
            break
        fi
    done
    if [ -z "$root" ]; then
        rm -rf "$tmpchain" "$tmpdir"
        echo "NO_ROOT_FOUND"
        return 0
    fi
    openssl x509 -in "$root" -outform DER 2>/dev/null | openssl dgst -sha256 | awk '{print $NF}'
    rm -rf "$tmpchain" "$tmpdir"
}

# Helper: fetch SPKI DER from a PEM public key URL and hash it.
fetch_sha256_spki() {
    local url="$1"
    local tmp
    tmp=$(mktemp)
    if ! curl -fsSL "$url" -o "$tmp"; then
        rm -f "$tmp"
        echo "FETCH_FAIL"
        return 0
    fi
    openssl pkey -pubin -in "$tmp" -pubout -outform DER 2>/dev/null | openssl dgst -sha256 | awk '{print $NF}'
    rm -f "$tmp"
}

# Upstream endpoints (verified 2026-04-19)
AMD_MILAN_URL="https://kdsintf.amd.com/vcek/v1/Milan/cert_chain"
AMD_GENOA_URL="https://kdsintf.amd.com/vcek/v1/Genoa/cert_chain"
INTEL_URL="https://certificates.trustedservices.intel.com/Intel_SGX_Provisioning_Certification_RootCA.cer"
REKOR_URL="https://rekor.sigstore.dev/api/v1/log/publicKey"

errors=()
mismatches=()

check_one() {
    local label="$1" in_tree="$2" upstream="$3"
    if [ "$upstream" = "FETCH_FAIL" ] || [ "$upstream" = "NO_ROOT_FOUND" ]; then
        errors+=("$label: upstream fetch/parse failed ($upstream); check network connectivity")
        return
    fi
    if [ "$in_tree" = "$upstream" ]; then
        echo "OK   $label: $in_tree"
    else
        mismatches+=("$label: in-tree=$in_tree upstream=$upstream")
        echo "MISS $label: in-tree=$in_tree upstream=$upstream" >&2
    fi
}

echo "=== spec 005 drift-check — $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="

# 1. AMD ARK-Milan
milan_in_tree=$(extract_pin "AMD_ARK_SHA256_FINGERPRINT" "$ATTESTATION_RS")
milan_up=$(fetch_sha256_pem_root "$AMD_MILAN_URL")
check_one "AMD ARK-Milan" "$milan_in_tree" "$milan_up"

# 2. AMD ARK-Genoa
genoa_in_tree=$(extract_pin "AMD_ARK_GENOA_SHA256_FINGERPRINT" "$ATTESTATION_RS")
genoa_up=$(fetch_sha256_pem_root "$AMD_GENOA_URL")
check_one "AMD ARK-Genoa" "$genoa_in_tree" "$genoa_up"

# 3. Intel DCAP Root
intel_in_tree=$(extract_pin "INTEL_ROOT_CA_SHA256_FINGERPRINT" "$ATTESTATION_RS")
intel_up=$(fetch_sha256_der "$INTEL_URL")
check_one "Intel DCAP Root" "$intel_in_tree" "$intel_up"

# 4. Rekor SPKI
rekor_in_tree=$(extract_pin "REKOR_PUBLIC_KEY" "$TRANSPARENCY_RS")
rekor_up=$(fetch_sha256_spki "$REKOR_URL")
check_one "Rekor SPKI" "$rekor_in_tree" "$rekor_up"

echo
if [ ${#errors[@]} -gt 0 ]; then
    echo "UPSTREAM FETCH FAILURES:" >&2
    for e in "${errors[@]}"; do
        echo "  $e" >&2
    done
    exit 2
fi

if [ ${#mismatches[@]} -eq 0 ]; then
    echo "ALL PINS MATCH UPSTREAM."
    exit 0
fi

echo "MISMATCHES DETECTED:" >&2
for m in "${mismatches[@]}"; do
    echo "  $m" >&2
done

if [ "$OPEN_ISSUE" = "--open-issue" ]; then
    if [ -z "${GITHUB_TOKEN:-}" ]; then
        echo "ERROR: --open-issue requires GITHUB_TOKEN in env." >&2
        exit 1
    fi
    if ! command -v gh >/dev/null; then
        echo "ERROR: --open-issue requires gh CLI." >&2
        exit 1
    fi
    body=$(printf 'Automated drift-check detected mismatch between pinned root-of-trust constants and upstream values.\n\n'; for m in "${mismatches[@]}"; do printf -- '- %s\n' "$m"; done; printf '\nRefer to docs/releases.md section 1 (Pre-release drift check).\n')
    gh issue create \
        --title "drift-check: pinned root-of-trust value mismatch detected" \
        --label "drift-check" \
        --body "$body" >&2
fi

exit 1
