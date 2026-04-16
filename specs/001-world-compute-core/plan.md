# Implementation Plan: World Compute — Core Cluster v1

**Branch**: `001-world-compute-core` | **Date**: 2026-04-15 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-world-compute-core/spec.md`

## Summary

Build the World Compute v1 agent, scheduler, data plane, verification
layer, ledger, CLI/GUI, and adapters as a single Rust codebase. The system
is a SETI@home-style volunteer compute federation that auto-forms clusters
on any network, sandboxes all workloads in VM-level isolation, verifies
results via R=3 quorum + 3% audit, records credits in a tamper-evident
non-blockchain ledger, and yields all donor resources within 1 second of
local user activity. The technical approach is synthesized from 7
completed research stages (~28,600 words) and 5 clarification decisions.

## Technical Context

**Language/Version**: Rust (latest stable, currently 1.82+), per FR-006
**Primary Dependencies**:
- `rust-libp2p` — P2P networking (transport, DHT, mDNS, gossip, relay)
- `firecracker` / `cloud-hypervisor` — VM-level sandboxing (Linux)
- `apple-virtualization-rs` / system `Virtualization.framework` (macOS)
- `windows-rs` + Hyper-V API (Windows)
- `wasmtime` — WASM workload runtime (Tier 3 / browser donors)
- `tonic` — gRPC server/client
- `clap` — CLI framework
- `tauri` — desktop GUI (OS-native WebView)
- `opentelemetry-rust` — structured logs + metrics + traces (FR-105–107)
- `reed-solomon-erasure` / `liberasurecode` — RS(10,18) erasure coding
- `ed25519-dalek` — peer identity, signing
- `threshold-crypto` — coordinator threshold signatures
- `cid` — CIDv1 content addressing
- `raft-rs` / `openraft` — coordinator consensus

**Storage**: CIDv1-addressed custom data plane over libp2p (hot tier on
executor nodes, cold tier RS(10,18) erasure-coded across donors).
Coordinator state in embedded Raft DB. Ledger as CRDT-replicated
Merkle-chained append-only log anchored to Sigstore Rekor.

**Testing**: Rust test harness (`cargo test`), integration tests on real
hardware (Phase 0 single-machine, Phase 1 3–5 LAN), direct-test evidence
artifacts per Principle V (quickstart.md). No mocks for sandbox, network,
or storage — real VMs, real libp2p, real erasure recovery.

**Target Platform**: Linux (primary, x86_64 + aarch64), macOS (Intel +
Apple Silicon), Windows (x86_64). Browser (WASM, Phase 3). Mobile
(deferred to Phase 3).

**Project Type**: Distributed system — daemon + CLI + GUI + adapters

**Performance Goals**: sub-second preemption yield (FR-040, <10ms SIGSTOP +
500ms full release), p95 <30min donor redemption queue (SC-007), >80% job
completion at 30% churn (SC-004), >90% completion over 30 days at GA
(SC-005).

**Constraints**: Principle I: VM-level sandbox non-negotiable, no
process-only isolation. Principle III: donor preemption is
unconditional, local-only, no network call. Principle V: every
component direct-tested on real hardware before deployment.

**Scale/Scope**: Phase 1: 3–5 machines. Phase 2: 20–50. Phase 3:
500–5000. Phase 4 (GA): unbounded. System designed for millions of nodes
but v1 must work correctly at 3.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status |
|-|-|-|
| I. Safety First | All workloads in VM-level sandbox; no host access; code-signed agent; attestation before dispatch; P0 incident halts cluster | **PASS** — FR-010–014, Firecracker/AppleVF/Hyper-V per platform |
| II. Robustness | R=3 replication; checkpoint every 60s; resume from any node; heartbeat failure detection; sharded-Raft control plane; RS(10,18) storage | **PASS** — FR-023, FR-030–034, FR-071 |
| III. Fairness | LOCAL_USER > all; SIGSTOP <10ms; guaranteed same-caliber redemption (FR-042); open-access multi-factor priority (FR-032); NCU credits; transparent auditable ledger; no pay-for-priority | **PASS** — FR-032, FR-040–042, FR-050–059, FR-103 |
| IV. Efficiency | 5–10% self-improvement slice; energy/carbon reporting; locality-aware scheduling; performance regressions block release | **PASS** — FR-033, SC-009 |
| V. Direct Testing | No component ships without real-hardware direct-test evidence; adversarial tests mandatory; Phases 0–4 with kill gates | **PASS** — FR-110–112, quickstart.md |

No violations. No complexity justification required.

## Project Structure

### Documentation (this feature)

```text
specs/001-world-compute-core/
├── plan.md              # This file
├── research.md          # Phase 0 consolidated decisions
├── data-model.md        # Phase 1 entity catalog (22 entities)
├── quickstart.md        # Phase 1 direct-test walkthrough
├── contracts/
│   ├── README.md        # Transport, auth, versioning
│   ├── donor.proto.md   # DonorService (6 RPCs)
│   ├── submitter.proto.md # SubmitterService (6 RPCs)
│   ├── cluster.proto.md # ClusterService (4 RPCs)
│   ├── governance.proto.md # GovernanceService (4 RPCs)
│   ├── admin.proto.md   # AdminService (4 RPCs)
│   ├── rest-gateway.md  # REST mapping + curl examples
│   └── errors.md        # 20 canonical error codes
├── whitepaper.md        # Public whitepaper
├── checklists/
│   └── requirements.md  # Spec quality checklist
├── design/
│   └── architecture-overview.md
├── research/
│   ├── 01-job-management.md
│   ├── 02-trust-and-verification.md
│   ├── 03-sandboxing.md
│   ├── 04-storage.md
│   ├── 05-discovery-and-bootstrap.md
│   ├── 06-fairness-and-credits.md
│   └── 07-governance-testing-ux.md
└── spec.md
```

### Source Code (repository root)

```text
src/
├── agent/               # Per-host daemon (entry point, lifecycle)
├── sandbox/
│   ├── firecracker.rs   # Linux KVM/Firecracker driver
│   ├── apple_vf.rs      # macOS Virtualization.framework driver
│   ├── hyperv.rs        # Windows Hyper-V driver
│   ├── wasm.rs          # Wasmtime runtime (Tier 3)
│   └── mod.rs           # Sandbox trait + factory
├── preemption/          # SIGSTOP supervisor, sovereignty triggers
├── scheduler/
│   ├── local.rs         # Local lease manager + ClassAd matching
│   ├── broker.rs        # Regional gossip broker
│   └── coordinator.rs   # Sharded-Raft global system of record
├── network/
│   ├── discovery.rs     # mDNS, Kademlia DHT, DNS bootstrap
│   ├── transport.rs     # QUIC, TCP, WebRTC, Circuit Relay
│   └── gossip.rs        # GossipSub for broker broadcast
├── data_plane/
│   ├── cid_store.rs     # CIDv1 content-addressed object store
│   ├── erasure.rs       # RS(10,18) encode/decode/repair
│   ├── placement.rs     # Geographic + AS-diverse shard placement
│   └── staging.rs       # Job input/output staging pipeline
├── verification/
│   ├── quorum.rs        # R=3 canonical-hash quorum
│   ├── audit.rs         # 3% spot-check re-execution
│   ├── trust_score.rs   # Trust Score computation (FR-052)
│   └── attestation.rs   # TPM/SEV/TDX/soft attestation
├── ledger/
│   ├── entry.rs         # LedgerEntry, Merkle chain
│   ├── crdt.rs          # OR-Map CRDT balance view
│   ├── threshold_sig.rs # Coordinator threshold signing
│   └── transparency.rs  # Sigstore Rekor anchoring
├── credits/
│   ├── ncu.rs           # NCU computation, DRF accounting
│   ├── decay.rs         # 45-day half-life credit decay
│   └── caliber.rs       # CaliberClass matching for redemption
├── acceptable_use/      # Policy enforcement filter
├── governance/          # Proposal, vote, report subsystem
├── telemetry/           # OTel structured logs + metrics + traces
├── cli/                 # `worldcompute` CLI (clap)
└── lib.rs               # Shared types, config, error model

gui/                     # Tauri desktop app (OS-native WebView)
├── src-tauri/           # Rust backend (calls into src/)
└── src/                 # React/TypeScript web frontend

adapters/
├── slurm/               # Slurm pilot-job gateway adapter
├── kubernetes/          # K8s operator + CRD
└── cloud/               # Cloud instance-metadata attester

tests/
├── contract/            # gRPC contract tests
├── integration/         # Multi-node real-hardware tests
├── adversarial/         # Red-team tests (sandbox escape, Byzantine)
└── unit/                # Unit tests (Rust #[test])

proto/                   # Protobuf definitions (source of truth for gRPC)
├── donor.proto
├── submitter.proto
├── cluster.proto
├── governance.proto
└── admin.proto
```

**Structure Decision**: Single Rust workspace (`cargo workspace`) with
crates for agent, cli, gui, adapters, and a shared `worldcompute-core`
library crate. The agent daemon and CLI are separate binaries sharing the
same crate graph. The Tauri GUI wraps the agent via local IPC. Adapters
are separate binaries per target (slurm-adapter, k8s-operator, etc.).

## Complexity Tracking

No Constitution Check violations — no entries needed.

## Phase 0 Research: Consolidated Decisions

See `research.md` for the full decision log. All NEEDS CLARIFICATION items
from the Technical Context section have been resolved by the 7 research
stages and 5 clarification questions. No outstanding unknowns block Phase 1
design or Phase 2 task generation.

## Phase 1 Design Artifacts

| Artifact | Status | Path |
|-|-|-|
| Data model (22 entities) | Complete | `data-model.md` |
| Contracts (5 services, 24 RPCs) | Complete | `contracts/` (8 files) |
| Quickstart (direct-test walkthrough) | Complete | `quickstart.md` |
| Architecture design | Complete (prior) | `design/architecture-overview.md` |

### Cross-artifact issues flagged by data-model agent

1. **I-02**: Replica placement diversity (not all replicas in top trust
   tier) is implicit in research but not a numbered FR — promote to FR
   before implementation.
2. **I-03**: WorkUnitReceipt NCU balance invariant (sum of per-node
   awards = submitter charge) — if a system-fee mechanism is added, a
   new LedgerEntryType is needed.
3. **I-06**: Consent double-check TOCTOU between broker grant and agent
   accept — protocol handshake must be documented before implementation.
4. **Governance quorum threshold** — voting rules not yet normative;
   GovernanceProposal outcome validation is deferred until governance
   doc is finalized.
5. **Contracts gap**: no standalone `GetCreditHistory` RPC with Merkle
   proof — add to DonorService before implementation if needed.
6. **Architecture doc**: references `go-libp2p` as v1 primary in one
   place; must be updated to `rust-libp2p` per FR-006 clarification.

### Post-design Constitution re-check

| Principle | Status |
|-|-|
| I. Safety First | **PASS** — sandbox drivers in data model, attestation in contracts, adversarial tests in quickstart |
| II. Robustness | **PASS** — 9-state Task lifecycle with checkpoint-before-commit, RS(10,18) in storage tier mapping |
| III. Fairness | **PASS** — NCU DRF accounting in credits module, CaliberClass enum in data model, priority hierarchy in scheduler |
| IV. Efficiency | **PASS** — self-improvement reserved slice as broker policy, energy reporting deferred to Phase 3 |
| V. Direct Testing | **PASS** — quickstart.md maps all P1 user stories to real-hardware test procedures with evidence artifacts |

## Next Step

Run `/speckit.tasks` to generate Phase 2: the task list for implementation,
organized by user story.
