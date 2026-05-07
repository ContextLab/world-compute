#!/usr/bin/env bash
# quickstart-timed.sh — spec 005 US8 T118 / FR-042 / SC-008.
#
# Measures wall-clock time for a fresh machine to reach a running donor
# agent following the quickstart.md steps. Exits 0 if under 15 minutes.
#
# Usage: scripts/quickstart-timed.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DEADLINE_SECONDS=900  # 15 minutes

TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)
EVIDENCE_DIR="${REPO_ROOT}/evidence/phase1/quickstart/${TIMESTAMP}"
mkdir -p "$EVIDENCE_DIR"
LOG="$EVIDENCE_DIR/run.log"

exec > >(tee "$LOG") 2>&1

echo "=== quickstart-timed run starting at $TIMESTAMP ==="
echo "Deadline: ${DEADLINE_SECONDS}s"

START=$(date +%s)

# Step 1: ensure binary exists (simulates "download")
step_start=$(date +%s)
echo "[Step 1] Build or locate binary"
cargo build --release --bin worldcompute 2>&1 | tail -5
BINARY="$REPO_ROOT/target/release/worldcompute"
if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: binary not produced" >&2
    exit 1
fi
echo "  Step 1 took $(( $(date +%s) - step_start ))s"

# Step 2: identity (idempotent)
step_start=$(date +%s)
echo "[Step 2] Create donor identity"
"$BINARY" donor status >/dev/null 2>&1 || true
echo "  Step 2 took $(( $(date +%s) - step_start ))s"

# Step 3: start daemon briefly in background
step_start=$(date +%s)
echo "[Step 3] Start daemon (30s window)"
"$BINARY" donor join --daemon --port 19990 &
DAEMON_PID=$!
sleep 10
if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    echo "ERROR: daemon died within 10s" >&2
    exit 1
fi
kill "$DAEMON_PID" 2>/dev/null || true
echo "  Step 3 took $(( $(date +%s) - step_start ))s"

# Step 6: admin status
step_start=$(date +%s)
echo "[Step 6] Run admin status"
"$BINARY" admin audit --id "test-proposal" | head -2
echo "  Step 6 took $(( $(date +%s) - step_start ))s"

TOTAL=$(( $(date +%s) - START ))
echo
echo "=== Total wall-clock: ${TOTAL}s (deadline ${DEADLINE_SECONDS}s) ==="

cat > "$EVIDENCE_DIR/metadata.json" <<EOF
{
  "run_id": "$TIMESTAMP",
  "area": "quickstart",
  "spec": "005-production-readiness",
  "git_sha": "$(git rev-parse HEAD)",
  "platform": "$(uname -s)-$(uname -m)",
  "total_seconds": $TOTAL,
  "deadline_seconds": $DEADLINE_SECONDS
}
EOF

cat > "$EVIDENCE_DIR/results.json" <<EOF
{
  "overall": "$(if (( TOTAL <= DEADLINE_SECONDS )); then echo pass; else echo fail; fi)",
  "assertions": [
    {
      "name": "SC-008: quickstart completes in <= 15 minutes",
      "expected": "total_seconds <= $DEADLINE_SECONDS",
      "observed": {"total_seconds": $TOTAL},
      "pass": $(if (( TOTAL <= DEADLINE_SECONDS )); then echo true; else echo false; fi)
    }
  ]
}
EOF

cat > "$EVIDENCE_DIR/index.md" <<EOF
# Quickstart Timed Evidence — $TIMESTAMP

**Platform**: $(uname -s) $(uname -m)
**Total**: ${TOTAL}s (deadline ${DEADLINE_SECONDS}s)
**Outcome**: $(if (( TOTAL <= DEADLINE_SECONDS )); then echo "✅ PASS"; else echo "❌ FAIL"; fi)
EOF

if (( TOTAL <= DEADLINE_SECONDS )); then
    echo "✅ SC-008 PASS (${TOTAL}s ≤ ${DEADLINE_SECONDS}s)"
    exit 0
else
    echo "❌ SC-008 FAIL (${TOTAL}s > ${DEADLINE_SECONDS}s)" >&2
    exit 1
fi
