# Quickstart: Full Functional Implementation Validation

**Date**: 2026-04-17 | **Branch**: `004-full-implementation`

## Prerequisites

- Rust stable 1.95.0+
- SSH access to `tensor01.dartmouth.edu` (credentials in `.credentials`)
- Linux with KVM support (for Firecracker tests)
- macOS 13+ (for Apple VF tests — developer workstation)
- Docker (for deployment tests)

## Quick Validation (local machine)

```sh
# Build and test
cargo build --lib
cargo test
cargo clippy --lib -- -D warnings
cargo fmt --check

# Verify test count >= 700
cargo test 2>&1 | grep "^test result:" | awk '{sum+=$4} END {print "Total:", sum}'

# Verify zero TODOs remain
grep -rn "// TODO" src/ | wc -l  # Must be 0

# Verify zero ignored tests
grep -rn '#\[ignore\]' tests/ | wc -l  # Must be 0
```

## Phase A Validation: Core Infrastructure

```sh
# Attestation: run cert chain tests
cargo test verification::attestation -- --nocapture

# Rekor: inclusion proof verification
cargo test ledger::transparency -- --nocapture

# Agent lifecycle
cargo test agent::lifecycle -- --nocapture

# Policy engine: artifact + egress
cargo test policy -- --nocapture

# Preemption latency (requires real sandbox)
cargo test preemption::supervisor -- --nocapture
```

## Phase B Validation: Security

```sh
# All adversarial tests (requires KVM on Linux)
cargo test adversarial -- --nocapture

# Confidential compute round-trip
cargo test data_plane::confidential -- --nocapture

# mTLS certificate lifecycle
cargo test network::tls -- --nocapture
```

## Phase C Validation: Real Hardware (tensor01.dartmouth.edu)

```sh
# SSH to test host
ssh f002d6b@tensor01.dartmouth.edu

# Clone and build on real hardware
git clone https://github.com/ContextLab/world-compute.git
cd world-compute && git checkout 004-full-implementation
cargo build --release

# Run full test suite on real hardware
cargo test

# Phase 1 LAN testnet (3 agent instances)
./target/release/worldcompute donor join --consent=general_compute &
./target/release/worldcompute donor join --consent=general_compute &
./target/release/worldcompute donor join --consent=general_compute &

# Verify cluster formation
./target/release/worldcompute cluster status

# Submit test job
./target/release/worldcompute job submit test-sha256.json

# Verify preemption latency
./target/release/worldcompute donor status --preemption-stats
```

## Phase F Validation: Deployment

```sh
# Docker build
docker build -t worldcompute:latest .

# Docker Compose 3-node cluster
docker compose up -d
docker compose exec agent1 worldcompute cluster status

# Verify cluster formed
docker compose exec agent1 worldcompute cluster peers
```

## Phase G Validation: Mesh LLM (requires GPU)

```sh
# Register expert nodes (on GPU machines)
./target/release/worldcompute mesh register --model llama-3-8b-q4

# Check router status
./target/release/worldcompute mesh status

# Generate tokens
./target/release/worldcompute mesh generate "Analyze scheduler efficiency"

# Test kill switch
./target/release/worldcompute mesh halt --reason "validation test"
```

## Success Criteria Checklist

- [ ] SC-001: `grep -rn "// TODO" src/ | wc -l` returns 0
- [ ] SC-002: `grep -rn '#\[ignore\]' tests/ | wc -l` returns 0
- [ ] SC-003: All 12 previously untested modules have tests in tests/
- [ ] SC-004: `cargo test` reports 700+ passing tests
- [ ] SC-005: Preemption latency < 10ms (measured on tensor01)
- [ ] SC-006: Agent withdrawal leaves zero residue (`find /tmp/worldcompute -type f | wc -l` returns 0)
- [ ] SC-007: Churn simulator reports >= 80% completion at 30% churn
- [ ] SC-008: 3-node cluster forms via mDNS in < 5 seconds
- [ ] SC-009: All CI checks pass (Linux, macOS, Windows)
- [ ] SC-010: Mesh LLM generates 3.2+ tokens/second
- [ ] SC-011: Kill switch halts inference within 1 second
- [ ] SC-012: Every FR has a corresponding passing test
