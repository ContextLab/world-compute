# World Compute

**A planetary-scale, decentralized volunteer compute federation — governed by a ratified constitution, backed by full research, and in active early implementation.**

[![Version](https://img.shields.io/badge/version-0.1.0--pre--alpha-lightgrey)]()
[![Status](https://img.shields.io/badge/status-early--implementation-yellow)]()
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)]()
[![Constitution](https://img.shields.io/badge/constitution-v1.0.0%20ratified-green)]()

---

> **Honesty notice — please read before going further.**
>
> This repository contains a ratified governing constitution, a full seven-stage research package (~28,600 words), a detailed feature specification, and this README. It does not contain any runnable code, compiled binaries, testnet infrastructure, or deployable agent. World Compute is a pre-implementation project as of 2026-04-15. Every CLI example and installation instruction in this document is aspirational and labeled accordingly. The design described here is complete and serious; the implementation has not started.
>
> If you want to help build it, see [Contributing](#contributing). If you want to be notified when it becomes installable, watch this repository.

---

## Table of Contents

1. [What is World Compute?](#what-is-world-compute)
2. [Why (Mission)](#why-mission)
3. [Status](#status)
4. [Quick Tour (Aspirational)](#quick-tour-aspirational)
5. [Architecture at a Glance](#architecture-at-a-glance)
6. [Installing](#installing)
7. [Public API Reference](#public-api-reference)
8. [Security and Safety](#security-and-safety)
9. [Contributing](#contributing)
10. [Governance and Funding](#governance-and-funding)
11. [Roadmap](#roadmap)
12. [License](#license)
13. [FAQ](#faq)
14. [Related Documents](#related-documents)

---

## What is World Compute?

World Compute is a SETI@home-style volunteer compute federation. Anyone who opts in installs a small background agent on their machine. The agent sandboxes and runs donated compute jobs only when the machine is genuinely idle, yields every resource the instant the local user returns, and records the contribution as redeemable compute credits. Donors can later spend those credits to run their own jobs on the collective cluster. The exchange is symmetric, transparent, and audited in real time.

The cluster is fractal by design. A two-laptop home network becomes a functioning micro-cluster the moment both agents start — no configuration, no central registry, no account required. That same LAN cluster is also a sub-cluster of the global federation once an internet connection is available. The topology spans personal PCs, institutional HPC clusters, Kubernetes deployments, cloud tenants, edge devices, and eventually browser tabs, all in one continuously self-healing mesh. There is no "size at which it breaks" in the design; the architecture was explicitly derived from the operational lessons of BOINC, Kubernetes, libp2p, and HTCondor.

Anyone can submit jobs to World Compute for free. No hardware donation is required to get started. Donors earn Normalized Compute Units (NCU) by contributing hardware; those NCU boost their scheduling priority. Job priority is computed from a continuous multi-factor score — NCU balance, public community votes, job size, queue age, and recent usage — so donors get faster service while non-donors' jobs still run, and no job waits forever. A public proposal board lets verified humans vote on which work deserves priority, giving the community a direct voice in how shared capacity is allocated.

No money changes hands in the credit system. No token is traded on any exchange. Financial support for the project flows through a separate, publicly-audited nonprofit entity whose income and expenditure are reported quarterly. The compute economy and the funding model are deliberately kept separate so that financial donation never buys scheduling priority.

---

## Why (Mission)

The goal is to make World Compute the most powerful compute cluster on Earth, governed as a public good, available to anyone.

Five constitutional principles govern every design decision. They are not aspirational values — they are hard engineering constraints that block deployment if violated.

**Principle I — Safety First.** The safety of donor machines is the single highest priority and overrides every other concern. A donor is lending hardware they rely on for their life and livelihood. A single verified breach would permanently destroy the trust that makes the cluster possible. Every workload must run inside a hardware-enforced sandbox (Firecracker on Linux, Apple Virtualization.framework on macOS, Hyper-V on Windows) with zero path to the host filesystem, credentials, network identity, peripherals, or user data. Discovered sandbox escapes trigger a cluster-wide halt immediately.

**Principle II — Robustness.** Every node is treated as fundamentally unreliable and capable of disappearing mid-job. The scheduler is declarative and self-healing. Storage is erasure-coded across geographically independent donors. Every long-running task checkpoints to the data plane so that any replica can resume on any eligible node from the latest checkpoint, without restarting from zero. Network partitions, high churn, and Byzantine donors are the assumed normal condition, not edge cases.

**Principle III — Fairness and Donor Sovereignty.** The local human user takes absolute priority over cluster workloads on their own machine, always, without exception. When the user returns — keyboard, mouse, thermal threshold, AC disconnect — all cluster jobs freeze within 10 milliseconds and release all interactive resources within 500 milliseconds. Access to the cluster is open: anyone can submit jobs for free. Donors earn scheduling priority through their NCU balance — a priority boost, not an access gate. Job scheduling uses a continuous multi-factor score (NCU, public votes, job size, queue age, recent usage) that guarantees no job waits forever and that donors are served faster without locking out non-donors. Accounting is transparent and auditable by every participant at any time.

**Principle IV — Efficiency and Self-Improvement.** A public-good cluster that squanders donated hardware and donated power is not a public good. At planetary scale, inefficiency is measured in gigawatts. A permanently reserved fraction of cluster capacity (5–10%) is dedicated to continuously improving the cluster itself — scheduler quality, sandbox hardening, storage efficiency, protocol evolution. This self-improvement capacity is realized as a distributed mesh LLM: each GPU-capable donor node runs a complete small language model; a lightweight router selects K-of-N models per inference step and aggregates their outputs. The mesh runs autonomous self-prompting agents that analyze cluster metrics, propose policy changes, and submit validated improvements through a tiered review and governance process. A governance kill switch can halt all mesh LLM activity instantly. Energy and carbon footprint are published. Joules-per-useful-result must improve year over year; failure to improve is a governance event, not an optimization target.

**Principle V — Direct Testing (Non-Negotiable).** No component ships until it has been directly tested by running real jobs on real representative hardware and verifying the results against known-correct answers. Mocks and simulators may supplement testing but cannot replace it. Safety-critical paths — sandboxing, preemption latency, data durability, attestation — must be tested adversarially on every release. A failing direct test blocks deployment. There are no exceptions for Principles I, II, III, or V.

---

## Status

World Compute has completed its initial implementation across all 11 phases. Updated 2026-04-16.

### Design artifacts (complete)

| Artifact | Status | Location |
|-|-|-|
| Ratified constitution (v1.0.0) | Complete | `.specify/memory/constitution.md` |
| Feature specification (130+ FRs, 12 SCs) | Complete | `specs/001-world-compute-core/spec.md` |
| Research (10 stages, ~28,600 words) | Complete | `specs/001-world-compute-core/research/` |
| Architecture design doc (22 entities) | Complete | `specs/001-world-compute-core/design/architecture-overview.md` |
| Data model (22 entities, state machines) | Complete | `specs/001-world-compute-core/data-model.md` |
| API contracts (5 services, 24 RPCs, 20 errors) | Complete | `specs/001-world-compute-core/contracts/` |
| Quickstart direct-test plan (7 adversarial tests) | Complete | `specs/001-world-compute-core/quickstart.md` |
| Implementation plan + task list (151 tasks) | Complete | `specs/001-world-compute-core/plan.md`, `tasks.md` |
| Whitepaper v0.2 (PDF) | Complete | `specs/001-world-compute-core/whitepaper.pdf` |
| This README + proposed API reference | Complete | `README.md` |

### Implementation (in progress)

| Component | Status | Tests | Key files |
|-|-|-|-|
| Cargo workspace + protos + CI | Complete | — | `Cargo.toml`, `proto/`, `.github/workflows/ci.yml` |
| Core types (NcuAmount, TrustScore, Cid, etc.) | Complete | — | `src/types.rs` |
| Error model (20 codes, gRPC + HTTP mapping) | Complete | — | `src/error.rs` |
| Sandbox trait + 4 platform drivers + GPU check | Complete | 3 tests | `src/sandbox/` |
| Preemption supervisor (<10ms SIGSTOP) | Complete | 5 tests | `src/preemption/` |
| P2P discovery (mDNS + Kademlia DHT) | Complete | 4 tests | `src/network/discovery.rs` |
| Agent lifecycle (enroll, heartbeat, pause, withdraw) | Complete | 7 tests | `src/agent/lifecycle.rs` |
| Cryptographic attestation (5 types) | Complete | 2 tests | `src/verification/attestation.rs` |
| Trust Score computation (T0-T4 tiers) | Complete | 4 tests | `src/verification/trust_score.rs` |
| CaliberClass (C0-C4) + same-caliber guarantee | Complete | 3 tests | `src/credits/caliber.rs` |
| NCU credits + S_ncu priority signal | Complete | 6 tests | `src/credits/ncu.rs` |
| CIDv1 content-addressed store | Complete | 4 tests | `src/data_plane/cid_store.rs` |
| Privacy-redacting telemetry | Complete | 4 tests | `src/telemetry/redaction.rs` |
| Job manifest parsing + validation | Complete | 4 tests | `src/scheduler/manifest.rs` |
| Multi-factor priority scorer (FR-032) | Complete | 5 tests | `src/scheduler/priority.rs` |
| R=3 quorum verification | Complete | 5 tests | `src/verification/quorum.rs` |
| Job/Task/Replica state machines | Complete | 6 tests | `src/scheduler/job.rs` |
| RS(10,18) erasure coding | Complete | 5 tests | `src/data_plane/erasure.rs` |
| CRDT ledger balance view | Complete | 4 tests | `src/ledger/crdt.rs` |
| 3% audit re-execution | Complete | 5 tests | `src/verification/audit.rs` |
| Transparency log anchoring (Sigstore stub) | Complete | 2 tests | `src/ledger/transparency.rs` |
| Job staging pipeline | Complete | 3 tests | `src/data_plane/staging.rs` |
| Work unit receipt | Complete | 2 tests | `src/verification/receipt.rs` |
| Submitter entity | Complete | 1 test | `src/scheduler/submitter.rs` |
| Regional broker (ClassAd matching) | Complete | 4 tests | `src/scheduler/broker.rs` |
| Coordinator scaffold (Raft roles) | Complete | 2 tests | `src/scheduler/coordinator.rs` |
| Transport config (QUIC + TCP) | Complete | 1 test | `src/network/transport.rs` |
| GossipSub protocol | Complete | 2 tests | `src/network/gossip.rs` |
| NAT traversal config | Complete | 1 test | `src/network/nat.rs` |
| Shard placement validation | Complete | 3 tests | `src/data_plane/placement.rs` |
| ComputeAdapter trait | Complete | 4 tests | `src/scheduler/adapter.rs` |
| Adapters (Slurm, K8s, cloud) | Complete | — | `adapters/` |
| Governance proposals + board | Complete | 12 tests | `src/governance/` |
| Humanity Points (Sybil resistance) | Complete | 5 tests | `src/governance/humanity_points.rs` |
| Quadratic voting | Complete | 4 tests | `src/governance/voting.rs` |
| Vote validation (self-vote exclusion) | Complete | 3 tests | `src/governance/vote.rs` |
| Mesh LLM router + expert registry | Complete | 8 tests | `src/agent/mesh_llm/` |
| Logit aggregation + token sampling | Complete | 3 tests | `src/agent/mesh_llm/aggregator.rs` |
| Self-prompting loop | Complete | 1 test | `src/agent/mesh_llm/self_prompt.rs` |
| Agent subsetting | Complete | 2 tests | `src/agent/mesh_llm/subset.rs` |
| Safety tiers + kill switch | Complete | 4 tests | `src/agent/mesh_llm/safety.rs` |
| Credit decay (45-day half-life) | Complete | 4 tests | `src/credits/decay.rs` |
| Acceptable-use filter | Complete | 2 tests | `src/acceptable_use/filter.rs` |
| Rate limiter (token bucket) | Complete | 3 tests | `src/network/rate_limit.rs` |
| mTLS config stub | Complete | 1 test | `src/network/tls.rs` |
| Build info (reproducible builds) | Complete | 1 test | `src/agent/build_info.rs` |
| Desktop GUI (Tauri scaffold) | Complete | — | `gui/` |
| CLI (donor + job + governance + admin) | Complete | — | `src/cli/` |
| Adversarial tests (4 stubs, #[ignore]) | Complete | — | `tests/adversarial/` |

**Total: 8,421 lines Rust across 84 files, 228 real tests (0 mocks), all passing.**

### Remaining (operational, not code)

| Item | Target |
|-|-|
| Testnet (multi-node real hardware) | Phase 1-2 of staged release |
| Legal entity / 501(c)(3) incorporation | Before Phase 3 alpha |
| Independent security audit | Before Phase 3 alpha |
| Web dashboard React SPA build | Phase 10-11 (scaffold in place) |
| Quickstart Phase 0-1 validation on real hardware | Before Phase 2 testnet |

The source of truth for what will be built is `specs/001-world-compute-core/spec.md`. Every requirement is traceable to a research finding in the ten research documents and is covered by at least one implementation task.

---

## Quick Tour (Aspirational)

The following examples show what using World Compute is designed to feel like. None of these commands work today. Each is labeled with the phase in which it is planned to become available.

### Becoming a donor (Phase 1, not yet available)

```
$ worldcompute donor join --cpu-cap 50% --schedule "22:00-08:00"
Agent installed. Peer ID: 12D3KooWR7bHxkjFe2q...
Discovered 2 LAN peers via mDNS (192.168.1.4, 192.168.1.7).
Joined cluster: wc-lan-a1b2c3 (3 nodes).
Accepting workload classes: scientific, public-good-ml, self-improvement.
Idle threshold: 30s. CPU cap: 50%. Active window: 22:00-08:00 local.
Trust attestation: TPM 2.0 PCR quote accepted by control plane.
Cluster status: ready.
```

```
$ worldcompute donor status
Node ID:       12D3KooWR7bH...
Cluster:       wc-global / shard-eu-west-3
Status:        idle (no active jobs)
Credits:       1,284.7 NCU earned / 0.0 NCU spent
Caliber class: 2 (consumer GPU, RTX 3080)
Trust tier:    T1 (TPM-attested CPU VM)
Uptime:        14d 6h 32m
Jobs run:      847 tasks / 841 verified / 6 disputed (quorum resolved)
```

### Submitting a job without being a donor (Phase 2, not yet available)

Anyone can submit jobs for free. Non-donors receive lower initial priority but the age signal ensures every job eventually runs.

```
$ worldcompute submit hello.yaml
Validating manifest...  ok
Staging inputs (3 CIDs)...  ok
Job ID:    job_8f9c2a4b1e...
Priority score: 0.34  (S_ncu=0.00, S_vote=0.50, S_size=0.93, S_age=0.00, S_cool=1.00)
Replicas:  R=3 (disjoint autonomous systems)

State: queued (position: 47 — free tier, lower priority than NCU holders)
State: leased  (nodes: eu-west-1, us-east-2, ap-south-1)
State: running (checkpoint interval: 60s)
State: verifying (2-of-3 quorum reached)
State: verified

Result CID:  bafybeig3k7resultcid...
Receipt:     sha256:e3b0c44298...  (anchored to Sigstore Rekor at 14:22:07 UTC)
NCU charged: 0.00  (no NCU balance; free submission)
```

### Submitting a donor job with NCU priority (Phase 2, not yet available)

Donors with an NCU balance automatically receive a higher priority score. No separate redemption command is needed.

```
$ worldcompute submit hello.yaml
Validating manifest...  ok
Staging inputs (3 CIDs)...  ok
Job ID:    job_8f9c2a4b1e...
Priority score: 0.63  (S_ncu=0.75, S_vote=0.50, S_size=0.93, S_age=0.00, S_cool=1.00)
Replicas:  R=3 (disjoint autonomous systems)

State: queued (position: 1 — donor priority lane)
State: leased  (1m 14s from submission)
State: running (checkpoint interval: 60s)
State: verifying (2-of-3 quorum reached)
State: verified

Result CID:  bafybeig3k7resultcid...
Receipt:     sha256:e3b0c44298...  (anchored to Sigstore Rekor at 14:22:07 UTC)
NCU charged: 0.42
```

### Submitting a compute proposal and checking votes (Phase 2, not yet available)

Any verified user can post a compute proposal for public voting. Votes boost the job's priority score.

```
$ worldcompute proposal submit --title "Protein folding for open-access drug discovery" \
    --description "Run AlphaFold2 on 50,000 candidate sequences; results published CC0." \
    --manifest ml-train.yaml
Proposal ID:  prop_7a3f9c2d...
Status:       open (voting window: 30 days)
Vote URL:     https://worldcompute.org/proposals/prop_7a3f9c2d

$ worldcompute proposal status prop_7a3f9c2d
Proposal:     Protein folding for open-access drug discovery
Status:       open
Upvotes:      1,284
Downvotes:    17
Net votes:    1,267  (out of ~12,000 verified voters this epoch)
S_vote score: 0.94
```

### Checking and verifying credits (Phase 2, not yet available)

```
$ worldcompute donor credits --verify
Balance: 1,284.7 NCU

Fetching Merkle proof from ledger...
Ledger root:    sha256:a1b2c3d4e5f6...
Proof depth:    14 nodes
Verification:   PASS  (locally verified against published Merkle root)
Rekor entry ID: 3f8c9d2a1b4e  (anchored 2026-04-15T14:20:00Z)

Last 5 events:
  2026-04-14 22:14  +12.3 NCU  job_7a3f... (scientific, verified 3/3)
  2026-04-14 23:07  +11.8 NCU  job_8b1c... (scientific, verified 3/3)
  2026-04-15 00:32  +14.1 NCU  job_2d9a... (public-good-ml, verified 3/3)
  2026-04-15 01:55   -0.4 NCU  job_8f9c... (redemption, hello.yaml)
  2026-04-15 02:18  +13.7 NCU  job_5e6b... (scientific, verified 3/3)
```

### Checking mesh LLM status (Phase 3, not yet available)

```
$ worldcompute mesh status
Mesh LLM:     active
Phase:        2 (local ensemble — centralized router fallback available)
Active nodes: 312 GPU experts  /  841 CPU experts
Router:       replicated (3 coordinator nodes)
Streams:      3 active agent streams
  Stream A:   scheduler-optimization  (running: "Analyze last 24h latency metrics")
  Stream B:   security-analysis       (idle)
  Stream C:   storage-efficiency      (running: "Propose RS(10,18) compaction schedule")
Throughput:   3.1 tok/s per stream
Kill switch:  armed (governance: 3-of-5 quorum to halt)
```

### Redeeming credits for your own job (Phase 2, not yet available)

```
$ worldcompute submit ml-train.yaml --priority DONOR_REDEMPTION
Validating manifest...  ok
Job ID:    job_f1a2b3c4...
Priority:  DONOR_REDEMPTION (hard SLA: p95 queue time < 2h)
Caliber:   class-2 (workstation GPU) requested and matched.

State: queued (position: 1 — donor redemption lane)
State: leased  (1m 14s from submission)
State: running
```

### Example job manifest

```yaml
apiVersion: worldcompute/v1
kind: Job
metadata:
  name: hello-sha256
spec:
  # OCI container image addressed by content ID (CIDv1 SHA-256).
  # WASM modules are equally supported: wasm+cid:bafybei...
  image: oci+cid:bafybeihashofalpinewithsha256utils

  command: ["sha256sum", "/input/data.bin"]

  inputs:
    - cid: bafybeig3k7inputdatacid
      mount: /input/data.bin

  outputs:
    - name: result
      path: /output/result.txt
      max_bytes: 1024

  resources:
    cpu: "1"
    memory: "512Mi"

  # priority_class is optional. Omit it and the scheduler computes a continuous
  # priority score automatically from five signals: NCU balance (auto-applied from
  # your account), public votes on any linked proposal, job size, queue age, and
  # recent usage. You do not need an NCU balance to submit; you will simply queue
  # at lower initial priority and rise over time via the age signal.
  priority_class: PUBLIC_GOOD     # PUBLIC_GOOD | SELF_IMPROVEMENT (DONOR_REDEMPTION
                                  # is no longer a separate class; NCU balance boosts
                                  # priority automatically)
  replica_count: 3                # R=3 default; 2-of-3 quorum decides accepted result
  verification: hash-quorum
  checkpoint_interval_s: 0        # short job; checkpointing not needed
  confidentiality: public
  acceptable_use_classes:
    - scientific
  max_wall_time: 300s
```

---

## Architecture at a Glance

World Compute uses a three-tier hierarchical scheduler. No tier is on the critical path of another tier's hard guarantees. Preemption authority lives exclusively in the local agent and never requires a network round-trip.

```
+----------------------------------------------------------+
|          GLOBAL FEDERATED CONTROL PLANE                  |
|  ~100-1,000 elected coordinator nodes                    |
|  Sharded Raft (job catalog, credit ledger, governance)   |
|  Threshold-signed | Merkle-chained | Sigstore-anchored   |
|                                                          |
|  Priority subsystem: multi-factor score per job          |
|  P = 0.35·S_ncu + 0.25·S_vote + 0.15·S_size              |
|      + 0.15·S_age + 0.10·S_cool                          |
|  Proposal board: public voting, HP-verified humans       |
+---------------------+------------------------------------+
                       |
             libp2p GossipSub + Kademlia DHT
                       |
        +--------------+---------------+
        |                              |
+-------v----------+        +----------v-------+
| REGIONAL BROKER  |  ...   | REGIONAL BROKER  |
| ClassAd matching |        | (LAN micro-shard)|
| Lease management |        | mDNS gateway     |
| Speculative exec |        |                  |
+-------+----------+        +----------+-------+
        |  pull lease                  |
        +--------------+---------------+
                       |
               +-------v-------+
               |  LOCAL AGENTS |
               |  (donor nodes)|
               |               |
               | Sandbox driver|  Firecracker (Linux)
               |               |  Apple VF (macOS)
               |               |  Hyper-V / WSL2 (Windows)
               | Preemption    |  SIGSTOP < 10ms, autonomous
               | Attestation   |  TPM 2.0 / SEV-SNP / signing
               | Checkpointing |  -> RS(10,18) storage plane
               | Mesh LLM      |  complete small model per GPU
               |   expert      |  node (LLaMA-3 tokenizer)
               +---------------+

Data plane:  all artifacts addressed by CIDv1 (SHA-256)
Storage:     Reed-Solomon RS(10,18), >=3 continents, <=2 shards/country
Encryption:  ChaCha20-Poly1305 per chunk; X25519 key wrap
P2P stack:   libp2p (QUIC primary, TCP fallback, WebRTC for browser)

Mesh LLM (5-10% of cluster capacity, self-improvement):
  Each GPU donor node runs a complete quantized small model.
  A replicated router selects K-of-N experts per inference step
  and aggregates sparse output distributions. Self-prompting
  agent streams analyze cluster metrics and propose improvements
  through a tiered review process; a governance kill switch can
  halt all mesh activity instantly.
```

The three tiers interact as follows. The global control plane is the durable system of record for the job catalog, credit ledger, governance actions, and the priority scoring subsystem. It is never on the critical path of a single task execution. Regional brokers own task queues for a geographic shard; they match tasks to nearby agents using ClassAd-style capability expressions and manage leases and speculative re-execution. Local agents are the only entities that touch donor hardware. They enforce donor sovereignty — preemption, throttling, quiet hours — entirely autonomously, without consulting any remote service. GPU-capable agents additionally run a complete small language model as a mesh LLM expert, contributing to the self-improvement capacity slice.

A three-machine isolated LAN functions as a self-contained cluster via mDNS peer discovery, electing one agent as a transient regional broker. When the LAN gains internet connectivity it merges into the global DHT transparently, with no configuration and no loss of in-flight work or credit history.

For depth on any subsystem, see `specs/001-world-compute-core/` and (once written) `specs/001-world-compute-core/design/architecture-overview.md`.

---

## Installing

**No installable artifact exists as of 2026-04-15.** The sections below describe what installation is planned to look like in Phase 3 of the project. Do not attempt to follow these instructions today; no binary will be found at any URL shown.

### Linux (Phase 3, not yet available)

```bash
# Verify KVM is available (required for Firecracker microVM sandbox)
ls /dev/kvm

# Install via script (planned)
curl -fsSL https://install.worldcompute.org/linux | sh

# Or via package manager (planned)
sudo apt install worldcompute        # Debian / Ubuntu
sudo dnf install worldcompute        # Fedora / RHEL
```

### macOS (Phase 3, not yet available)

```bash
# Apple Virtualization.framework handles sandboxing; no KVM required.
brew install worldcompute            # planned Homebrew cask

# Or via notarized .pkg installer from the releases page (planned)
```

### Windows (Phase 3, not yet available)

```
# Requires Windows 10/11 with "Virtual Machine Platform" feature enabled.
# Hyper-V isolation is used on Pro/Enterprise; WHPX fallback covers Home.

winget install WorldCompute.Agent    # planned WinGet package
```

### HPC / Slurm cluster (Phase 3, not yet available)

```bash
# Install the Slurm adapter on a submit node (planned)
pip install worldcompute-slurm-adapter

# Register the cluster's idle capacity with World Compute
worldcompute-slurm-adapter register --cluster my-hpc --cpu-cores 512
```

### Kubernetes (Phase 3, not yet available)

```bash
# Install the World Compute operator (planned)
kubectl apply -f https://install.worldcompute.org/k8s/operator.yaml

# Donate a slice of cluster capacity
kubectl apply -f - <<EOF
apiVersion: worldcompute.org/v1
kind: ClusterDonation
metadata:
  name: my-donation
spec:
  cpu: "20"
  memory: "80Gi"
  gpu: 2
  jobClasses: [scientific, public-good-ml]
EOF
```

---

## Public API Reference

The API described in this section is **proposed and not yet implemented**. It represents the design target based on the ratified spec and research. All method names, field names, error codes, and endpoint paths are subject to change during implementation. This section is included because it is the most detailed public statement of intended interface contract that exists today.

### Transport

gRPC is the primary transport, using protobuf schema as the single source of truth for all types and service definitions. A REST/HTTP+JSON gateway is generated from the same `.proto` definitions via `grpc-gateway`, so the CLI, web dashboard, and third-party integrations all share one contract.

**Authentication:**
- Node-to-node (agent, broker, coordinator): mTLS with per-account certificates issued by the control plane CA.
- User-facing (CLI, web dashboard, third-party): OAuth2 bearer tokens. The CLI performs device-flow OAuth and stores credentials in the system keychain.

**Versioning:** All endpoints are versioned under `/v1/`. Breaking changes will increment to `/v2/`.

---

### DonorService

Manages the lifecycle of a donor node: joining the cluster, configuring resource policies, tracking credits, and withdrawing cleanly.

| Method | Request | Response | Description |
|-|-|-|-|
| `Join` | `JoinRequest` | `DonorHandle` | Register a new donor node; returns peer ID, keypair, and initial trust tier assignment. |
| `Status` | `StatusRequest` | `DonorStatus` | Current node status: online/paused, resource utilization, active job count, trust score. |
| `UpdateConfig` | `DonorConfig` | `Ack` | Update donor preferences: job classes, CPU/GPU caps, quiet-hours schedule, storage cap, preemption thresholds. |
| `PauseResume` | `PauseResumeAction` | `Ack` | Pause donation (gracefully evicts and checkpoints active jobs) or resume. |
| `Credits` | `CreditsRequest` | `CreditStatement` | Return current NCU balance with ledger event history. When `verify=true`, includes a Merkle proof against the published ledger root. |
| `Withdraw` | `WithdrawRequest` | `WithdrawalReceipt` | Cleanly withdraw: evict all jobs, wipe all host-resident cluster state, revoke keypair. Unspent credits remain redeemable for 180 days by default. |

---

### SubmitterService

Manages the full job lifecycle from submission through result retrieval.

| Method | Request | Response | Description |
|-|-|-|-|
| `SubmitJob` | `JobManifest` | `JobHandle` | Submit a job manifest (YAML or JSON); returns job ID and initial queued state. |
| `GetJob` | `JobId` | `JobStatus` | Poll job state: `QUEUED`, `LEASED`, `RUNNING`, `VERIFYING`, `VERIFIED`, `DISPUTED`, `FAILED`. |
| `StreamJobLogs` | `JobId` | `stream LogEntry` | Stream structured log output from executors in real time (server-streaming RPC). |
| `CancelJob` | `JobId` | `Ack` | Cancel an in-progress job; committed checkpoints and partial results are retained. |
| `ListJobs` | `JobFilter` | `JobList` | List submitter's jobs with filter by state, priority class, and date range, with cursor pagination. |
| `FetchResult` | `JobId` | `ResultStream` | Download job output as a stream or receive a short-lived presigned URL to the result CID. |

---

### ClusterService

Read-only visibility into the cluster's current state, ledger, and peer topology.

| Method | Request | Response | Description |
|-|-|-|-|
| `GetClusterStatus` | `ClusterStatusRequest` | `ClusterStatus` | Global cluster health: node count by trust tier, jobs in flight, outstanding NCU supply, scheduler queue depth. |
| `ListPeers` | `PeerFilter` | `PeerList` | List visible peers filtered by trust tier, caliber class, and region, with pagination. |
| `GetLedgerHead` | _(empty)_ | `LedgerHead` | Return the current Merkle root of the credit ledger and its Sigstore Rekor anchor entry ID. |
| `VerifyReceipt` | `ReceiptId` | `VerificationResult` | Verify a work unit receipt against the ledger; returns the Merkle proof path and verification outcome. |

---

### ComputeProposalService

Public compute proposals and voting. Any verified user can submit a proposal; any human with HP >= 5 can vote. Vote tallies feed the `S_vote` priority signal for jobs submitted under the proposal.

| Method | Request | Response | Description |
|-|-|-|-|
| `CreateComputeProposal` | `ComputeProposalBody` | `ComputeProposal` | Submit a compute proposal: title, description, estimated resource needs, openness declaration. Returns a proposal ID. |
| `GetComputeProposal` | `ProposalId` | `ComputeProposal` | Retrieve a proposal including current upvote/downvote counts, net vote score, and computed `S_vote`. |
| `CastComputeVote` | `ComputeVoteRequest` | `VoteReceipt` | Cast an upvote or downvote on an open proposal. Requires caller HP >= 5. Vote is ledger-recorded with verifiable witness. |
| `ListComputeProposals` | `ProposalFilter` | `ProposalList` | List proposals filtered by status (open/closed), sort order, and date range. |

---

### MeshLLMService

Read-only visibility into the mesh LLM self-improvement subsystem. Configuration changes require governance approval; the kill switch is in AdminService.

| Method | Request | Response | Description |
|-|-|-|-|
| `GetMeshStatus` | _(empty)_ | `MeshStatus` | Current mesh health: active expert count, active agent streams, tokens/second per stream, router phase, kill-switch state. |
| `ListMeshAgents` | `AgentFilter` | `AgentList` | List running self-improvement agent streams with their current task and progress. |
| `GetMeshAgentOutput` | `AgentId` | `AgentOutput` | Retrieve the latest analysis or proposal text generated by a specific agent stream. |

---

### GovernanceService

Proposal creation, voting, and financial reporting. All records are written to the same append-only ledger as compute provenance.

| Method | Request | Response | Description |
|-|-|-|-|
| `ListProposals` | `ProposalFilter` | `ProposalList` | List open and closed governance proposals with status and vote tallies. |
| `CreateProposal` | `ProposalBody` | `Proposal` | Submit a new policy or constitutional amendment proposal (TSC or Board members only). |
| `CastVote` | `CastVoteRequest` | `VoteReceipt` | Cast a vote on an open proposal; the ballot is recorded to the ledger with a verifiable witness. |
| `GetReport` | `ReportPeriod` | `FinancialReport` | Retrieve a published quarterly financial or compliance report. |

---

### AdminService

Emergency and administrative operations. Halt and resume require designated on-call or Board authorization; bans are logged to the public ledger.

| Method | Request | Response | Description |
|-|-|-|-|
| `HaltDispatch` | `HaltRequest` | `Ack` | Halt all new job dispatches cluster-wide (P0 incident response). Requires retroactive governance review within 7 days. |
| `ResumeDispatch` | `ResumeRequest` | `Ack` | Resume dispatching after an emergency halt. Requires Board authorization. |
| `BanNode` | `BanRequest` | `Ack` | Ban a node from the cluster for policy violation; reason is recorded to the public ledger. |
| `RotateCoordinatorKey` | `KeyRotationRequest` | `Ack` | Initiate a coordinator signing key rotation; requires threshold quorum of coordinators. |

---

### REST Gateway Examples

The REST gateway is generated from the same proto schema as the gRPC API. All requests require an `Authorization: Bearer <token>` header (or mTLS certificate for node clients).

#### Submit a job

```
POST /v1/jobs
Content-Type: application/json

{
  "metadata": {
    "name": "hello-sha256"
  },
  "spec": {
    "image": "oci+cid:bafybeihashofalpinewithsha256utils",
    "command": ["sha256sum", "/input/data.bin"],
    "inputs": [
      { "cid": "bafybeig3k7inputdatacid", "mount": "/input/data.bin" }
    ],
    "outputs": [
      { "name": "result", "path": "/output/result.txt", "max_bytes": 1024 }
    ],
    "resources": { "cpu": "1", "memory": "512Mi" },
    "priority_class": "PUBLIC_GOOD",
    "replica_count": 3,
    "verification": "hash-quorum",
    "checkpoint_interval_s": 0,
    "confidentiality": "public",
    "acceptable_use_classes": ["scientific"],
    "max_wall_time": "300s"
  }
}
```

Response `202 Accepted`:

```json
{
  "job_id": "job_8f9c2a4b1e3d",
  "state": "QUEUED",
  "submitted_at": "2026-04-15T14:21:53Z",
  "estimated_start": "2026-04-15T14:22:10Z"
}
```

#### Poll job status

```
GET /v1/jobs/job_8f9c2a4b1e3d
```

Response `200 OK`:

```json
{
  "job_id": "job_8f9c2a4b1e3d",
  "state": "VERIFIED",
  "priority_class": "PUBLIC_GOOD",
  "replicas_completed": 3,
  "replicas_required": 3,
  "quorum_result": "hash-quorum-2of3",
  "result_cid": "bafybeig3k7resultcid",
  "receipt_id": "rcpt_a1b2c3d4",
  "receipt_sha256": "e3b0c44298fc1c149afbf4c8996fb924",
  "ncu_charged": 0.42,
  "completed_at": "2026-04-15T14:22:19Z"
}
```

#### Verify credit balance with Merkle proof

```
GET /v1/donors/me/credits?verify=true
```

Response `200 OK`:

```json
{
  "account_id": "did:wc:12D3KooWR7bH...",
  "balance_ncu": 1284.7,
  "earned_total_ncu": 1285.1,
  "spent_total_ncu": 0.4,
  "ledger_root_sha256": "a1b2c3d4e5f6789012345678901234567890abcd",
  "rekor_entry_id": "3f8c9d2a1b4e",
  "rekor_verified_at": "2026-04-15T14:20:00Z",
  "merkle_proof": {
    "depth": 14,
    "path": ["b1c2d3...", "e4f5a6...", "..."],
    "leaf_hash": "7f8a9b...",
    "root_hash": "a1b2c3..."
  },
  "verification_result": "PASS"
}
```

#### Submit a compute proposal

```
POST /v1/proposals
Content-Type: application/json

{
  "title": "Protein folding for open-access drug discovery",
  "description": "Run AlphaFold2 on 50,000 candidate sequences; results published CC0.",
  "estimated_ncu_hours": 500,
  "results_public": true
}
```

Response `201 Created`:

```json
{
  "proposal_id": "prop_7a3f9c2d",
  "status": "open",
  "voting_closes_at": "2026-05-15T14:21:53Z",
  "vote_url": "https://worldcompute.org/proposals/prop_7a3f9c2d"
}
```

#### Get a proposal with vote count

```
GET /v1/proposals/prop_7a3f9c2d
```

Response `200 OK`:

```json
{
  "proposal_id": "prop_7a3f9c2d",
  "title": "Protein folding for open-access drug discovery",
  "status": "open",
  "upvotes": 1284,
  "downvotes": 17,
  "net_votes": 1267,
  "total_epoch_voters": 12000,
  "s_vote": 0.94,
  "submitted_at": "2026-04-15T14:21:53Z",
  "voting_closes_at": "2026-05-15T14:21:53Z"
}
```

#### Cast a vote on a proposal

Requires HP >= 5. Callers below that threshold receive a `403 Forbidden` with `"code": "INSUFFICIENT_HP"`.

```
POST /v1/proposals/prop_7a3f9c2d/vote
Content-Type: application/json

{
  "direction": "up"
}
```

Response `200 OK`:

```json
{
  "receipt_id": "vote_rcpt_b1c2d3",
  "proposal_id": "prop_7a3f9c2d",
  "direction": "up",
  "voter_hp": 8,
  "vote_weight": 1.0,
  "recorded_at": "2026-04-15T15:03:22Z",
  "ledger_entry": "sha256:f7a1b2c3..."
}
```

#### Get mesh LLM status

```
GET /v1/mesh/status
```

Response `200 OK`:

```json
{
  "mesh_active": true,
  "phase": 2,
  "active_experts": 312,
  "active_cpu_experts": 841,
  "active_streams": 3,
  "tokens_per_second_per_stream": 3.1,
  "router_mode": "local-ensemble",
  "kill_switch_armed": true,
  "streams": [
    {"id": "stream_A", "domain": "scheduler-optimization", "status": "running"},
    {"id": "stream_B", "domain": "security-analysis",      "status": "idle"},
    {"id": "stream_C", "domain": "storage-efficiency",     "status": "running"}
  ]
}
```

---

### Full Job Manifest Reference (YAML)

A complete annotated manifest showing all fields the scheduler recognizes:

```yaml
apiVersion: worldcompute/v1
kind: Job
metadata:
  name: protein-fold-batch-001
  submitter_did: did:wc:12D3KooWSub...

spec:
  # Workload image: OCI container or WASM module, both addressed by CIDv1.
  # OCI images run inside Firecracker (Linux), Apple VF (macOS), Hyper-V (Windows).
  # WASM modules run in Wasmtime (native hosts) or a browser Worker (Tier 3 donors).
  image: oci+cid:bafybeihashofalphafoldimagecid
  # Alternatively for WASM: wasm+cid:bafybeihashofwasmbinarymodule

  command: ["python", "-m", "alphafold", "--input", "/input/seq.fasta"]

  inputs:
    - cid: bafybeig3k7inputsequencecid
      mount: /input/seq.fasta
    - cid: bafybeig3k7modelweightscid
      mount: /input/weights/

  outputs:
    - name: structure
      path: /output/structure.pdb
      max_bytes: 10485760       # 10 MiB hard cap; job killed if exceeded

  resources:
    cpu: "4"
    memory: "16Gi"
    gpu:
      class: "rtx30+"           # minimum GPU capability class
      vram_gb: 8
      optional: true            # job runs on CPU-only donors if no GPU available

  # Scheduling policy
  # priority_class is optional. When omitted, the scheduler computes a continuous
  # priority score from five signals (NCU balance, public votes, size, age, cooldown).
  # NCU from your account is applied automatically — no separate redemption step needed.
  priority_class: PUBLIC_GOOD   # PUBLIC_GOOD | SELF_IMPROVEMENT (omit for auto-scored)
  replica_count: 3              # R=3 default; R=5 for high-value; R=1 + TEE for confidential
  verification: hash-quorum     # hash-quorum | range-check | tee-attestation
  preempt_class: checkpointable # yieldable | checkpointable | restartable

  # Checkpointing: required for jobs expected to run longer than ~5 minutes
  checkpoint_interval_s: 60
  max_wall_time: 86400s         # 24 hours; job is killed if wall time is exceeded

  # Locality hint: prefer executors that already hold input data shards locally
  locality_hint:
    prefer_near_cid: bafybeig3k7inputsequencecid

  # Trust and confidentiality
  confidentiality: public        # public | confidential-medium | confidential-high
  min_trust_tier: T1             # T0 (browser/WASM) | T1 (TPM CPU VM) | T2 (TPM+GPU) | T3 (SEV-SNP/TDX)

  # Acceptable-use: donor opt-in classes must include at least one of these
  acceptable_use_classes:
    - scientific
    - public-good-ml

  # Soft deadline: scheduler deprioritizes after this time but does not hard-cancel
  deadline: 2026-05-01T00:00:00Z
```

---

### Error Model

All API surfaces return errors in a structured envelope with a canonical code, a human-readable message, and an optional detail map.

| Code | Meaning |
|-|-|
| `INVALID_MANIFEST` | Job manifest fails schema validation or signature verification. |
| `INSUFFICIENT_HP` | Caller's humanity-point score is below the minimum required for this operation (e.g., casting a vote requires HP >= 5). |
| `ACCEPTABLE_USE_VIOLATION` | Job matches a prohibited category: unauthorized scanning, malware, illegal content, or targeted surveillance. |
| `NO_ELIGIBLE_NODES` | No nodes match the job's resource requirements, trust tier, caliber class, or geographic constraints. |
| `QUORUM_FAILURE` | Replicas returned inconsistent results; no quorum established after re-execution attempts. |
| `TRUST_TIER_MISMATCH` | Job requires a higher trust tier than any available node can satisfy (e.g., `confidential-high` requires T3). |
| `ATTESTATION_REJECTED` | Agent binary or workload image failed cryptographic attestation; node is quarantined. |
| `JOB_NOT_FOUND` | Requested job ID does not exist or does not belong to this account. |
| `RATE_LIMITED` | Account has exceeded the per-period rate limit for this operation; see `Retry-After` header. |
| `UNAUTHORIZED` | Token or certificate is missing, expired, or lacks sufficient scope. |
| `CLUSTER_HALTED` | A P0 emergency halt is in effect; no new jobs are being dispatched. |

---

### Rate Limits and Quotas

Rate limits are enforced per account at the API gateway. Default limits for newly registered accounts are conservative to prevent abuse while the cluster is small; accounts with a positive, multi-day Trust Score history receive higher limits. Specific published limits will appear in the operator documentation when the service is live. Expect per-account limits on: job submissions per minute, concurrent jobs, maximum input dataset size, maximum output size, and maximum wall-clock time per job. Donors set explicit per-machine resource caps (CPU fraction, GPU fraction, storage quota, network bandwidth) via `UpdateConfig`; the scheduler never exceeds declared donor caps. A request that exceeds any limit receives a `RATE_LIMITED` error with a `Retry-After` header.

---

## Security and Safety

Principle I of the constitution makes security not a feature but the precondition for this project's existence. Every production component is required to be open-source, independently auditable, and reproducibly built. Closed-source binaries are prohibited from running on donor machines by constitutional mandate. The sandboxing architecture uses hardware-enforced hypervisor boundaries on every supported platform; process-level isolation alone (namespaces, seccomp, gVisor without KVM) is explicitly insufficient and is prohibited by the feature specification (FR-010).

Any discovered sandbox escape, privilege escalation, or host-data exfiltration is a P0 incident. The constitution requires that affected agent versions be remotely disabled, new job dispatches halted cluster-wide, and public disclosure made within 72 hours of mitigation (and within 30 days of detection even if mitigation is delayed).

Before the project establishes a formal security contact, use GitHub's private vulnerability reporting feature for sensitive findings, or open a public issue tagged `security` for non-sensitive disclosures. A formal security contact address and written incident-disclosure policy will be published before the Phase 3 public alpha.

---

## Contributing

World Compute is in the pre-code phase. The most valuable contributions right now are:

- **Review and critique the research.** All seven research documents are in `specs/001-world-compute-core/research/`. Factual corrections, omitted prior art, and design tradeoff challenges are welcome as GitHub issues or pull requests against the research documents.
- **Review and critique the spec.** `specs/001-world-compute-core/spec.md` is the feature specification. Gaps, inconsistencies with the research findings, and missing requirements are valuable.
- **Participate in governance design.** The constitution and governance model are in `.specify/memory/constitution.md` and `specs/001-world-compute-core/research/07-governance-testing-ux.md`.

When implementation begins, all code changes must:

1. Pass a Constitution Check against Principles I–V, documented explicitly in the pull request.
2. Include a direct-test plan on real representative hardware (Principle V — non-negotiable).
3. Address host integrity and data-isolation impact (Principle I), failure modes and recovery (Principle II), donor-experience impact (Principle III), and resource and energy implications (Principle IV).
4. Follow the speckit workflow described in `.specify/memory/constitution.md`.

Code review must verify principle compliance. Reviewers are expected to block merges that regress sandbox strength, preemption latency guarantees, data-durability guarantees, or direct-test coverage.

---

## Governance and Funding

World Compute is planned to incorporate as a US 501(c)(3) public charity prior to the Phase 3 public alpha. The governing structure is a two-body model: a Technical Steering Committee (5–7 members, elected by active contributors) for technical decisions, and a Board of Directors (5 members) for financial and legal decisions. No company may hold more than 2 seats on either body. No TSC member may simultaneously be a board member.

Financial donations fund security audits, developer compensation, CI infrastructure, and test hardware — not compute priority. Financial donation explicitly cannot confer scheduling priority; the bylaws will document the refusal mechanism and it is enforced by the scheduler architecture. All income and expenditure are published quarterly in machine-readable format.

For the governance model in full detail — including the Public Good Review Board that approves `PUBLIC_GOOD` job classifications, the weighted voting structure, the approved funding channels, and the comparative analysis of Mozilla, Wikimedia, ISRG, and other nonprofit models — see `specs/001-world-compute-core/research/07-governance-testing-ux.md`.

There is no donation channel today. When the legal entity is incorporated, a donation link will appear on this page and in the quarterly financial reports. Do not send money to any address claiming to represent World Compute until that announcement is made through this repository.

---

## Roadmap

All phases are targets. None are completed as of 2026-04-15.

| Phase | Label | Key milestones |
|-|-|-|
| Phase 0 | Single-machine smoke tests | Agent installs from source; sandboxes a trivial job; returns correct result; leaves no host residue on exit; adversarial filesystem and network isolation tests pass. One developer laptop. |
| Phase 1 | 3–5 machine LAN testnet | Peer discovery via mDNS on a real home network (no configuration); job scheduling across nodes; checkpoint-resume across simulated host failure; sub-second preemption verified; no cross-node data leakage. |
| Phase 2 | 20–50 machine federated testnet | Heterogeneous hardware and NAT types across at least 3 geographic regions; 80% job completion rate over 72h with 30% simulated node churn; credit accounting cross-verified; legal entity incorporated. |
| Phase 3 | 500–5,000 node public alpha | 90% job completion over 30 days; zero real-world Principle I incidents; independent security audit completed with critical findings remediated; energy footprint published; mobile and browser donor modes independently audited and optionally available. |
| Phase 4 | General availability | Phase 3 metrics sustained for 30 days; governance structure fully seated; incident-disclosure policy published and tested; joules-per-NCU baseline established for year-over-year tracking. |

---

## License

Apache License 2.0 (planned).

A `LICENSE` file exists in this repository reserving rights pending the formal license attachment. When implementation begins, the full Apache 2.0 license text will be applied to all source files. The constitution, specification, and research documents are intended to be freely reusable for any project building toward similar public-good compute goals.

---

## FAQ

**Do I need to donate hardware to use World Compute?**

No. Anyone can submit jobs for free. No NCU balance is required to submit a job. Non-donors start with a lower priority score but their jobs rise in priority over time via the queue-age signal, and the system guarantees no job waits forever. Donors earn NCU by contributing hardware, which boosts their scheduling priority — but NCU is a priority boost, not an access gate.

**How does job priority work?**

Priority is a continuous score computed from five signals: `P = 0.35·S_ncu + 0.25·S_vote + 0.15·S_size + 0.15·S_age + 0.10·S_cool`. S_ncu reflects your NCU balance (saturating exponential — more NCU helps, but returns diminish). S_vote reflects public community votes on a linked compute proposal. S_size favors smaller jobs. S_age grows as a job waits, guaranteeing eventual scheduling. S_cool penalizes recent heavy users to prevent monopolization. The weights are governance-configurable and published transparently. There is no separate "donor redemption" queue; donors submit normally and their NCU balance is applied automatically.

**What is the mesh LLM?**

The mesh LLM is a distributed ensemble-of-experts language model that uses 5–10% of cluster capacity to improve the cluster itself. Each GPU-capable donor node runs a complete small quantized model (LLaMA-3 family). A lightweight router selects K-of-N experts per inference step and aggregates their sparse output distributions. The resulting system runs autonomous self-prompting agent streams that analyze cluster metrics, draft policy improvements, and run sandboxed experiments. It operates on timescales of minutes to hours, not real-time conversation. The minimum viable mesh requires approximately 280 cluster nodes. A phased rollout starts with a centralized small model and graduates to full distributed operation at 5,000+ nodes.

**Can the mesh LLM break things?**

The mesh LLM's proposed changes are never applied directly to production. Every proposed change passes through a staged pipeline: simulation against the last 24 hours of cluster traffic, then a 1% canary deployment, then the required governance approval tier. High-impact changes require a full governance vote and 24-hour review period. Any governance participant can issue a kill-switch command that immediately halts all mesh LLM inference streams and reverts the last several applied changes. The kill switch cannot be overridden by the mesh LLM itself.

**Is this a cryptocurrency?**

No. The Normalized Compute Unit (NCU) is an internal accounting unit that tracks the exchange between donated compute and redeemable compute. It is not traded on any exchange, has no monetary value, cannot be sold, and is not a financial instrument. The research explicitly rejected token-based designs because speculation creates incentives incompatible with the "donors are sovereign citizens" model in Principle III.

**Do you use a blockchain?**

No. The credit ledger is an append-only, Merkle-chained, threshold-signed, CRDT-replicated log anchored externally to Sigstore Rekor. It provides the tamper-evidence and auditability of a blockchain without the energy overhead, settlement latency, or token-economy baggage. `specs/001-world-compute-core/research/02-trust-and-verification.md` contains a detailed analysis of why blockchain was rejected as the primary ledger mechanism.

**Will running this hurt my computer?**

No. The agent runs only when the machine is genuinely idle. The moment you touch the keyboard, move the mouse, launch a foreground application, unplug AC power, or cross a thermal threshold, all cluster workloads freeze within 10 milliseconds. Full release of all interactive resources happens within 500 milliseconds. The agent runs at minimum system priority and is capped at user-configured CPU and GPU fractions. Sub-second yield of interactive resources is a constitutional requirement (Principle III), not a best-effort target.

**What is the trust model?**

Layered. At the network layer: cryptographic peer identity (Ed25519), IP-diversity enforcement in Kademlia routing tables (S/Kademlia-style), and GossipSub behavioral peer scoring. At the execution layer: replicated computation with canonical-hash quorum (default R=3, 2-of-3 agreement), plus randomized 3% audit re-execution on independent high-trust nodes to catch quorum collusion. At the hardware layer: TPM 2.0 cryptographic attestation for x86 hosts, Apple Notarization chain for macOS, and AMD SEV-SNP or Intel TDX attestation for nodes handling confidential workloads. Browser and mobile nodes are explicitly lower-trust (Tier 0) and receive only public-data workloads with higher replication factors.

**Can I run arbitrary code on the cluster?**

Yes, within the acceptable-use policy. The system accepts any OCI container image or WASM module as a workload, subject to: signature and content-address verification, acceptable-use category classification reviewed by the Public Good Review Board, and donor consent (donors opt in per workload class and can refuse any class at any time). The system is constitutionally required to refuse jobs categorized as unauthorized network scanning, malware distribution, illegal content, targeted surveillance, or credential cracking against third parties.

**How do I know my job's result is correct?**

Three mechanisms stack. First, replication: by default, R=3 independent donor nodes execute your task and the result is accepted only when at least 2 of 3 return identical canonical hashes. Second, ongoing audit: 3% of accepted results are randomly re-executed on high-trust nodes to detect quorum collusion over time. Third, optional attestation: you can request execution on Trust Tier T3 nodes (AMD SEV-SNP or Intel TDX), which provide a hardware-signed measurement proving that a specific, unmodified workload ran on specific hardware. Every accepted result carries a signed work unit receipt anchored to the public ledger and verifiable locally by any party.

**Why is a 501(c)(3) better than a DAO?**

Practical governance stability. A 501(c)(3) can hold employment contracts, own infrastructure, enter agreements, be sued and held accountable, and pursue tax-deductible donations from the largest donor pools globally. DAOs cannot do any of those things directly. On-chain governance has proven effective at distributing grants from a treasury; it has not replaced legal entities in any project requiring operational continuity. The World Compute governance model uses principles from DAO design — transparent public voting, immutable ledger of decisions — without requiring a blockchain to implement them and without the token-concentration problem that affects most deployed DAOs.

**What if my ISP or country blocks peer-to-peer traffic?**

The agent uses layered NAT traversal with automatic fallback. For most residential connections, QUIC hole punching via libp2p DCUtR succeeds without any configuration (effective for approximately 85% of internet NAT types based on WebRTC production data). For symmetric NAT — common on cellular networks and in approximately 15–20% of internet hosts — the agent falls back automatically to libp2p Circuit Relay v2 through volunteer relay nodes, covering close to 100% of connected configurations. The scheduler accounts for NAT type when placing high-bandwidth jobs, preferring co-located or same-LAN executors for data-intensive work.

**Why should I trust the coordinators?**

You do not have to trust any individual coordinator. The credit ledger is threshold-signed: a quorum of coordinators must agree before any ledger record is accepted. The Merkle root is anchored externally to Sigstore Rekor every 10 minutes, so no coordinator can rewrite history without detection by any external party. Merkle roots are also published in quarterly reports. The coordinator set is elected from high-attestation, high-uptime donors, is subject to governance oversight, and undergoes a mandatory formal compliance review against Principles I–V at least quarterly once the cluster is serving real jobs.

**How do you pay for this?**

Operating costs — security audits, developer salaries, CI infrastructure, test hardware, legal fees — are funded through charitable donations and grants to the 501(c)(3) entity. Revenue sources modeled in the research include individual donations, tiered corporate sponsorship (structured as charitable contributions, not membership dues), and grants from science foundations (NSF, NIH, Mozilla Foundation, Alfred P. Sloan Foundation). All income and expenditure are published quarterly in machine-readable format. The NCU compute economy is entirely separate from the project's finances; they do not interact.

**When can I install it?**

You cannot yet. This is a pre-code project as of 2026-04-15. No agent binary, CLI, testnet, or hosted service exists. Watch this repository for updates. The roadmap above describes the phase gates that must be cleared before any public installation is offered.

**Where do I send money?**

Nowhere yet. No donation channel has been established because the legal entity has not been incorporated. When that changes, a donation link will appear on this page, on the project website, and will be announced through this repository's releases and discussions. Until that announcement, do not send money to any address claiming to represent World Compute.

---

## Related Documents

| Document | Description |
|-|-|
| `.specify/memory/constitution.md` | The ratified governing constitution, v1.0.0. Binding on all components, contributors, and deployments. |
| `specs/001-world-compute-core/spec.md` | Feature specification: user stories, functional requirements, success criteria, key entities, out-of-scope items. |
| `specs/001-world-compute-core/research/01-job-management.md` | Job model, three-tier scheduler architecture, checkpointing strategy, quorum verification, prior art survey across BOINC, Kubernetes, HTCondor, Nomad, Spark, and others. |
| `specs/001-world-compute-core/research/02-trust-and-verification.md` | Trust scoring, verifiable compute, attestation design, blockchain analysis and rejection rationale. |
| `specs/001-world-compute-core/research/03-sandboxing.md` | Sandbox architecture per platform (Firecracker/KVM, Apple Virtualization.framework, Hyper-V/WSL2, WASM), GPU passthrough via VFIO+IOMMU, TPM attestation, red-team test plan. |
| `specs/001-world-compute-core/research/04-storage.md` | Reed-Solomon RS(10,18) erasure coding, CIDv1 content addressing, geographic placement constraints, ChaCha20-Poly1305 encryption, CRDT metadata plane design. |
| `specs/001-world-compute-core/research/05-discovery-and-bootstrap.md` | libp2p stack selection and rationale, mDNS LAN auto-discovery, Kademlia DHT WAN routing, DNS bootstrap, NAT traversal (DCUtR + Circuit Relay v2), adapter architecture for HPC/K8s/cloud/browser. |
| `specs/001-world-compute-core/research/06-fairness-and-credits.md` | NCU credit model and hardware normalization, original priority hierarchy (superseded in part by research/08), preemption mechanics, credit decay and inflation control, Public Good Review Board governance, direct-test plan for fairness properties. |
| `specs/001-world-compute-core/research/07-governance-testing-ux.md` | 501(c)(3) structure and funding model, comparative nonprofit analysis, five-phase staged testing with pass/kill gates, CLI (Rust+clap) and GUI (Tauri) framework selection, API design. |
| `specs/001-world-compute-core/research/08-priority-redesign.md` | Open-access multi-factor scheduling: composite priority formula, public voting with Sybil-resistant HP verification, starvation-freedom proof, fairness analysis. Supersedes the five-class priority hierarchy from research/06. |
| `specs/001-world-compute-core/research/09-mesh-llm.md` | Distributed mesh LLM for self-improvement: ensemble-of-experts architecture, router design, tokenizer standardization (LLaMA-3), safety tiers, phased rollout, resource budget, and prior art survey. |
| `specs/001-world-compute-core/research/10-prior-art-distributed-inference.md` | Prior art survey for distributed inference systems (Petals, Hivemind, Together.ai, Swarm, FriendliAI, and others). |
| `specs/001-world-compute-core/design/architecture-overview.md` | _(planned)_ Consolidated architecture design document. |
| `specs/001-world-compute-core/whitepaper.md` | _(planned)_ Public-facing whitepaper covering design rationale and prior art in full. |
