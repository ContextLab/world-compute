#!/usr/bin/env bash
# verify-no-placeholders.sh — spec 005 FR-038 / SC-006 hard-block CI check.
#
# Scans production Rust sources for placeholder tokens. Any match not listed
# in .placeholder-allowlist (at repo root) causes a hard failure.
#
# Usage:
#   scripts/verify-no-placeholders.sh              # scan + exit 0/64
#   scripts/verify-no-placeholders.sh --list       # list every match with allowlist-membership
#   scripts/verify-no-placeholders.sh --check-empty # additionally require empty allowlist (spec-005-completion gate)
#
# Exit codes:
#   0  — clean (or every match covered by allowlist)
#   64 — at least one match is not covered by allowlist
#   65 — --check-empty requested and allowlist has non-comment entries
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

ALLOWLIST_FILE=".placeholder-allowlist"
MODE="${1:-scan}"

# Scope per contracts/ci-verify-no-placeholders.md
SCAN_GLOBS=(
    "src"
    "adapters/slurm/src"
    "adapters/kubernetes/src"
    "adapters/cloud/src"
    "gui/src-tauri/src"
    "proto"
)

# Token regex (case-insensitive word boundary)
PATTERN='\b(placeholder|stub|TODO|todo!|unimplemented!)\b'

# Collect matches as "path:line:content"
matches=""
for scope in "${SCAN_GLOBS[@]}"; do
    if [[ -d "$scope" ]]; then
        # Use grep -r with -n; -E for extended regex; -i for case-insensitive
        if grep_output=$(grep -rniE "$PATTERN" "$scope" --include='*.rs' --include='*.proto' 2>/dev/null); then
            matches="${matches}${grep_output}"$'\n'
        fi
    fi
done

# Load allowlist into a sorted set of "path:line" keys
allowlist_keys=""
if [[ -f "$ALLOWLIST_FILE" ]]; then
    # Non-comment, non-empty lines; extract path:line before " — "
    allowlist_keys=$(grep -vE '^\s*(#|$)' "$ALLOWLIST_FILE" 2>/dev/null \
        | awk -F' — ' '{print $1}' \
        | sort -u || true)
fi

# Partition matches
unallowed=""
allowed_count=0
unallowed_count=0

while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    # Extract path:line from grep output (format: path:lineno:content)
    path_line=$(echo "$line" | awk -F: '{print $1":"$2}')
    if echo "$allowlist_keys" | grep -Fxq "$path_line"; then
        allowed_count=$((allowed_count + 1))
    else
        unallowed="${unallowed}${line}"$'\n'
        unallowed_count=$((unallowed_count + 1))
    fi
done <<< "$matches"

# --list mode: print everything and exit 0
if [[ "$MODE" == "--list" ]]; then
    echo "=== Matches covered by $ALLOWLIST_FILE (allowed): $allowed_count ==="
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        path_line=$(echo "$line" | awk -F: '{print $1":"$2}')
        if echo "$allowlist_keys" | grep -Fxq "$path_line"; then
            echo "[ALLOWED] $line"
        fi
    done <<< "$matches"
    echo
    echo "=== Matches NOT in allowlist (would fail scan): $unallowed_count ==="
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        path_line=$(echo "$line" | awk -F: '{print $1":"$2}')
        if ! echo "$allowlist_keys" | grep -Fxq "$path_line"; then
            echo "[DENIED]  $line"
        fi
    done <<< "$matches"
    exit 0
fi

# Default / --check-empty mode: enforce
if [[ -n "$unallowed" ]]; then
    echo "ERROR: $unallowed_count placeholder occurrence(s) not in $ALLOWLIST_FILE:" >&2
    echo >&2
    echo "$unallowed" >&2
    echo >&2
    echo "To fix: remove the placeholder in source, OR (if the mention is legitimate" >&2
    echo "historic-context documentation) add it to $ALLOWLIST_FILE with a rationale." >&2
    echo "NOTE: during spec-005 implementation, allowlist entries are NOT permitted." >&2
    exit 64
fi

if [[ "$MODE" == "--check-empty" ]]; then
    # grep returns non-zero when no non-comment lines exist, which under
    # set -o pipefail would kill the script. Use `|| true` to absorb the
    # zero-match case cleanly.
    nonempty_lines=$(grep -vE '^\s*(#|$)' "$ALLOWLIST_FILE" 2>/dev/null || true)
    if [[ -n "$nonempty_lines" ]]; then
        count=$(echo "$nonempty_lines" | wc -l | tr -d ' ')
        echo "ERROR: spec-005-completion gate requires empty $ALLOWLIST_FILE but $count entry/entries present:" >&2
        echo "$nonempty_lines" >&2
        exit 65
    fi
fi

echo "OK: zero placeholder occurrences in production sources ($allowed_count allowed, 0 denied)."
exit 0
