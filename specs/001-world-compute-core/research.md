# Phase 0 Research: Consolidated Decision Log

**Feature**: 001-world-compute-core
**Date**: 2026-04-15
**Source**: 7 research stages (~28,600 words) + 5 clarification decisions

This document consolidates the key decisions from the research phase. Each
entry links to the full research file for traceability. All NEEDS
CLARIFICATION items have been resolved — no outstanding unknowns block
Phase 1 design or Phase 2 task generation.

## Decisions

### D1. Scheduler architecture — Three-tier hybrid hierarchical

**Decision**: Sharded-Raft global coordinators (~100–1000 operator-hardened
nodes) + libp2p gossip regional brokers (ClassAd-style matchmaking) +
fully autonomous local agents (preemption authority exclusively local).

**Rationale**: Sub-second preemption (FR-040) is only achievable locally
by physical necessity — no remote coordinator can participate in that path.
Separating the tiers means no tier is on the critical path of another's
hard guarantees.

**Alternatives rejected**: Fully centralized (single point of failure,
violates decentralization), fully P2P gossip-only (too slow for global
consistency, can't enforce fairness accounting), Kubernetes-only (too
heavy for volunteer laptops, not peer-to-peer).

**Source**: [research/01-job-management.md](research/01-job-management.md)

### D2. Job model — Replicated work units with CID-addressed manifests

**Decision**: Task (atomic) → Workflow (DAG) → Job (instance). Default
R=3 replicas with canonical-hash 2-of-3 quorum. Workloads specified as
OCI containers or WASM modules, CID-addressed. Mandatory checkpointing
every 60s.

**Rationale**: R=3 quorum is BOINC's 20-year scar tissue — it works on
heterogeneous untrusted hardware. CID addressing gives trustless integrity
verification. 60s checkpointing collapses preemption, migration, and
recovery into one operation.

**Alternatives rejected**: R=1 with TEE-only (TEEs not universal, side-
channel-prone), DAG-less flat task model (can't express ML pipelines),
custom manifest format (OCI/WASM have ecosystem tooling).

**Source**: [research/01-job-management.md](research/01-job-management.md)

### D3. Trust and verification — Layered stack, NO blockchain

**Decision**: (1) R=3 replicated execution + hash quorum (default),
(2) TEE-attested single-execution for T3+ nodes (SEV-SNP/TDX),
(3) 3% randomized audit re-execution on high-trust nodes, (4) Trust
Score-weighted replica placement on disjoint AS/trust buckets, (5) zkVMs
reserved for future niche. Credit ledger is a CRDT-replicated
Merkle-chained threshold-signed append-only log anchored to Sigstore
Rekor every 10 min — NOT a blockchain.

**Rationale**: Blockchain adds consensus overhead, gas, and energy
cost incompatible with Principle IV; the ledger achieves every
tamper-evidence property a chain would, with sub-second writes and no
collateral. Layered verification avoids silver-bullet thinking — each
layer compensates for the others' blind spots.

**Alternatives rejected**: PoS blockchain for credit ledger (consensus
overhead, gas, unnecessary), zkVM as primary verifier (10⁴–10⁶× proving
overhead in 2026), optimistic rollups (incompatible with 2h redemption
SLA, requires collateral), reputation-only (gameable).

**Source**: [research/02-trust-and-verification.md](research/02-trust-and-verification.md)

### D4. Trust Score formula

**Decision**: `T = clamp(0,1, 0.5·R_consistency + 0.3·R_attestation +
0.2·R_age) · (1 − P_recent_failures)`. Capped at 0.5 for 7 days,
ramps to 1.0 after 30 days. Browser/WASM capped at T0. Two strikes in
a 14-day window → quarantine and re-attestation.

**Source**: [research/02-trust-and-verification.md](research/02-trust-and-verification.md)

### D5. Sandboxing — Tiered VM-boundary-mandatory per platform

**Decision**: Firecracker microVM on Linux (Tier 1 GPU-eligible),
Apple Virtualization.framework on macOS (Tier 2 CPU-only), Hyper-V/WSL2
on Windows (Tier 2), WASM-only on mobile/browser (Tier 3). Process-only
isolation is NEVER sufficient for production.

**Rationale**: Kernel CVEs cross process boundaries by definition. Only
a VMM/hypervisor boundary provides the isolation class needed for
Principle I.

**Alternatives rejected**: gVisor alone (user-space kernel, incomplete
syscall coverage, no GPU), Kata containers alone (adds OCI overhead
without WASM support), Bubblewrap/seccomp alone (process-level,
insufficient).

**Source**: [research/03-sandboxing.md](research/03-sandboxing.md)

### D6. Storage — Custom data plane with RS(10,18) erasure coding

**Decision**: Build a custom data plane on libp2p primitives (go-cid for
CIDv1, libp2p transport) with RS(10,18) erasure coding (k=10 data, n=18
total, 8 parity, 1.80× overhead). Shard placement: 1/AS, ≤2/country,
≥3 continents. CRDT + Raft metadata plane. Content-addressed with
SHA-256 CIDv1 throughout.

**Rationale**: IPFS Bitswap lacks erasure coding and placement
enforcement. Filecoin adds blockchain consensus overhead. Tahoe-LAFS
validated the erasure-coded volunteer model but is Python/legacy.
Building custom on proven primitives gives the best cost/control.

**Alternatives rejected**: IPFS as-is (no erasure coding, no placement),
Filecoin (blockchain overhead), Storj (centralized satellite model),
3× replication (2.8× more overhead than RS for same durability).

**Source**: [research/04-storage.md](research/04-storage.md)

### D7. P2P discovery — libp2p (rust-libp2p) + mDNS + Kademlia

**Decision**: rust-libp2p for native nodes, js-libp2p for browser. QUIC
transport (primary), TCP (fallback), WebRTC DataChannel (browser). mDNS
for zero-config LAN discovery (<2s), Kademlia DHT for WAN, GossipSub
v1.1 for broker broadcast, DCUtR for hole punching, Circuit Relay v2 as
final fallback. DNS seeds for initial WAN bootstrap. ComputeAdapter
trait for Slurm/K8s/cloud/edge/mobile/browser integration.

**Source**: [research/05-discovery-and-bootstrap.md](research/05-discovery-and-bootstrap.md)

### D8. Fairness — NCU credits, priority hierarchy, <10ms preemption

**Decision**: NCU (Normalized Compute Unit) = 1 TFLOP/s FP32-second,
multi-dimensional DRF accounting. Scheduling hierarchy: LOCAL_USER >
DONOR_REDEMPTION > PAID_SPONSORED > PUBLIC_GOOD > SELF_IMPROVEMENT.
SIGSTOP within 10ms, 500ms checkpoint, full release <1s. Credits decay
with 45-day half-life. Same-caliber redemption guarantee enforced via
caliber classes 0–4.

**Source**: [research/06-fairness-and-credits.md](research/06-fairness-and-credits.md)

### D9. Governance — US 501(c)(3), TSC + Board, Tauri + Rust + gRPC

**Decision**: US 501(c)(3) Delaware public charity (ISRG model). Two-body
governance: TSC (5–7, merit-elected, no company >2 seats) + Board (5,
partially elected). CLI: Rust + clap. Desktop GUI: Tauri. Web: React SPA.
API: gRPC primary + REST gateway. Staged release: Phase 0–4 with
published kill gates.

**Source**: [research/07-governance-testing-ux.md](research/07-governance-testing-ux.md)

### D10–D14. Clarification decisions (speckit.clarify session)

| # | Decision | FR |
|-|-|-|
| D10 | Agent implemented in Rust everywhere (rust-libp2p) | FR-006 |
| D11 | Apache 2.0 license | FR-099 |
| D12 | US 501(c)(3) Delaware (ISRG model) | FR-100 |
| D13 | Per-donor shard-category allowlist for data residency | FR-074 |
| D14 | Full OpenTelemetry (logs+metrics+traces) + privacy redaction | FR-105–107 |

**Source**: [spec.md § Clarifications](spec.md)

## Open items (deferred to implementation planning)

1. Coordinator election/rotation protocol
2. GPU kernel preemption (CUDA MPS / driver time-slicing)
3. Empirical calibration (audit rate, Trust Score weights, credit decay)
4. Acceptable-use automated classifier design
5. Relay bandwidth capacity model at scale
6. Coordinator threshold-signing key management
7. Governance voting quorum threshold formula

These are flagged in the spec checklist and plan.md — none block task
generation.
