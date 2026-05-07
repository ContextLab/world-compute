#!/usr/bin/env bash
# validate-evidence.sh — per contracts/evidence-artifact-format.md.
#
# Verifies that an evidence bundle directory contains the required files,
# that metadata.json.git_sha points at a real commit, that results.json is
# well-formed, and that total size is under the 10 MB soft limit.
#
# Usage: scripts/validate-evidence.sh path/to/evidence/phase1/<area>/<ts>/
set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: $0 <evidence-bundle-dir>" >&2
    exit 2
fi

DIR="$1"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ ! -d "$DIR" ]]; then
    echo "ERROR: not a directory: $DIR" >&2
    exit 2
fi

errs=0
for required in run.log metadata.json results.json index.md; do
    if [[ ! -f "$DIR/$required" ]]; then
        echo "MISSING: $DIR/$required" >&2
        errs=$((errs + 1))
    fi
done

if [[ -f "$DIR/metadata.json" ]]; then
    if ! python3 -c "import json; d = json.load(open('$DIR/metadata.json')); assert 'git_sha' in d, 'no git_sha'" 2>/dev/null; then
        echo "ERROR: $DIR/metadata.json malformed or missing git_sha" >&2
        errs=$((errs + 1))
    else
        sha=$(python3 -c "import json; print(json.load(open('$DIR/metadata.json'))['git_sha'])")
        if ! git -C "$REPO_ROOT" cat-file -e "$sha" 2>/dev/null; then
            echo "WARNING: metadata.json git_sha $sha is not a known commit (may be unreachable)" >&2
        fi
    fi
fi

if [[ -f "$DIR/results.json" ]]; then
    if ! python3 -c "import json; d = json.load(open('$DIR/results.json')); assert d.get('overall') in ('pass','fail','partial'), 'bad overall'" 2>/dev/null; then
        echo "ERROR: $DIR/results.json malformed or overall not in {pass,fail,partial}" >&2
        errs=$((errs + 1))
    fi
fi

size_bytes=$(du -sb "$DIR" 2>/dev/null | awk '{print $1}')
if [[ "${size_bytes:-0}" -gt 10485760 ]]; then
    echo "ERROR: bundle size ${size_bytes} bytes exceeds 10 MB hard limit" >&2
    errs=$((errs + 1))
elif [[ "${size_bytes:-0}" -gt 5242880 ]]; then
    echo "NOTE: bundle size ${size_bytes} bytes exceeds 5 MB soft-warn threshold" >&2
fi

if [[ $errs -eq 0 ]]; then
    echo "OK: $DIR is a valid evidence bundle."
    exit 0
fi

echo "FAIL: $errs issue(s) in $DIR" >&2
exit 1
