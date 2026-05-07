# Phase 1 Data Model — Spec 005 Production Readiness

**Feature**: 005-production-readiness
**Date**: 2026-04-19
**Scope**: Define entities, fields, relationships, validation rules, and lifecycle/state transitions for every new or materially changed concept introduced by spec 005.

Entities are grouped by subsystem. Each entity lists its Rust module location (target after implementation), attributes with types, relationships, validation rules, and state transitions if applicable.

---

## Group A — Cross-firewall transport

### A.1 `RelayReservation`

**Location**: `src/network/relay_reservation.rs` (NEW)

**Purpose**: Represents a libp2p Relay v2 reservation held by this agent on a remote relay so NATed peers can reach it.

**Attributes**:
- `relay_peer_id: libp2p::PeerId` — the relay server's PeerId
- `circuit_multiaddr: libp2p::Multiaddr` — the reserved circuit address (`/p2p/<relay>/p2p-circuit/p2p/<self>`)
- `expires_at: chrono::DateTime<chrono::Utc>` — reservation expiry from the relay
- `renew_at: chrono::DateTime<chrono::Utc>` — scheduled time to renew (derived: expires_at minus 30s)
- `status: ReservationStatus` — see state machine
- `lost_at: Option<chrono::DateTime<chrono::Utc>>` — set when reservation is detected lost

**Validation rules**:
- `circuit_multiaddr` MUST contain exactly one `/p2p-circuit/` component.
- `renew_at < expires_at` MUST hold.
- `lost_at` is `Some` iff `status == Lost`.

**State transitions** (`ReservationStatus`):
```
Requesting → Active (on ReservationReqAccepted event from relay)
Requesting → Failed (on ReservationReqDenied or timeout)
Active → Renewing (at renew_at)
Renewing → Active (on successful renew)
Active → Lost (on connection drop or explicit ReservationExpired)
Lost → Requesting (within 60s per FR-006)
```

**Relationships**: Many-to-one with `NetworkIdentity` (an agent can hold multiple reservations simultaneously for redundancy).

---

### A.2 `WssTransportConfig`

**Location**: `src/network/wss_transport.rs` (NEW)

**Purpose**: Configuration for the WebSocket-over-TLS-443 fallback transport.

**Attributes**:
- `enabled: bool` — default `true`
- `listen_on_443: bool` — if this node should listen on 443 (typically only relays)
- `fallback_priority: u8` — order in transport preference (QUIC=0, TCP=1, WSS=2 by default)
- `middlebox_pin_check: bool` — default `true`, checks known-relay fingerprints
- `allow_ssl_inspection: bool` — default `false`, opt-in via `--allow-ssl-inspection`

**Validation rules**:
- `fallback_priority` is unique across enabled transports within a single daemon.
- `middlebox_pin_check == false` requires `allow_ssl_inspection == true` (safety tier downgrade).

**Relationships**: One-to-one with the `NodeBehaviour` swarm configuration.

---

### A.3 `DialAttempt`

**Location**: `src/network/dial_logging.rs` (NEW)

**Purpose**: Single observable record of a libp2p dial, surfaced at `info`+ per FR-004.

**Attributes**:
- `timestamp: chrono::DateTime<chrono::Utc>`
- `target_multiaddr: libp2p::Multiaddr`
- `transport: TransportKind` — enum `Tcp | Quic | Wss | Relay`
- `outcome: DialOutcome` — enum `Success | Timeout | TransportError(String) | Denied(String)`
- `root_cause: Option<String>` — surfaced from libp2p's DialError

**Validation rules**:
- `root_cause` is `Some` iff `outcome != Success`.

**Relationships**: Emitted as `tracing::info!` events; no persistent storage required.

---

### A.4 `DohResolverConfig`

**Location**: `src/network/doh_resolver.rs` (NEW)

**Purpose**: Configuration for the DoH fallback resolver.

**Attributes**:
- `upstreams: Vec<String>` — default `["https://cloudflare-dns.com/dns-query", "https://dns.google/dns-query"]`
- `timeout_ms: u32` — default 5000
- `engage_on_os_failure_only: bool` — default `true`

**Validation rules**:
- `upstreams` MUST contain ≥ 2 distinct hostnames.
- All upstreams MUST be `https://` URLs.

**Relationships**: One-to-one with the daemon's DNS resolution policy.

---

## Group B — Deep attestation

### B.1 `PinnedRootCa`

**Location**: `src/verification/attestation.rs` (MUTATED)

**Purpose**: A compile-time pinned manufacturer root CA fingerprint that anchors attestation chains.

**Attributes**:
- `manufacturer: Manufacturer` — enum `AmdArk | IntelDcap`
- `sha256_fingerprint: [u8; 32]` — MUST NOT be all zeros in `production` feature build
- `source_url: &'static str` — URL from which the fingerprint was verified at release-cut time
- `verified_at: &'static str` — ISO-8601 timestamp from release procedure

**Validation rules** (enforced at compile time via `const` assertion when `feature = "production"`):
- `sha256_fingerprint != [0u8; 32]`
- `source_url` MUST start with `https://`

**Relationships**: Consumed by `CertificateChainValidator::validate_chain()`.

---

### B.2 `PinnedRekorKey`

**Location**: `src/ledger/transparency.rs` (MUTATED)

**Purpose**: Pinned Sigstore Rekor Ed25519 public key used to verify signed tree heads.

**Attributes**:
- `public_key: [u8; 32]` — MUST NOT be all zeros in `production` feature build
- `key_id: String` — Rekor's published key ID for drift comparison
- `verified_at: &'static str`

**Validation rules**: Identical zero-check as `PinnedRootCa`.

**Relationships**: Consumed by `TransparencyLog::verify_tree_head_signature()`.

---

### B.3 `DriftCheckResult`

**Location**: `src/verification/drift_check.rs` (NEW, minimal — primary logic in `scripts/drift-check.sh`)

**Purpose**: Record of a weekly CI drift check comparing pinned constants against upstream.

**Attributes**:
- `checked_at: chrono::DateTime<chrono::Utc>`
- `amd_ark_matches: bool`
- `intel_dcap_matches: bool`
- `rekor_key_matches: bool`
- `opened_issues: Vec<u64>` — GitHub issue numbers opened on mismatch

**Validation rules**: `opened_issues` is non-empty iff any `*_matches` is false.

**Relationships**: Emitted as a structured log by `.github/workflows/drift-check.yml`.

---

## Group C — Real Firecracker rootfs

### C.1 `OciLayer`

**Location**: `src/sandbox/firecracker/rootfs_builder.rs` (NEW)

**Purpose**: A single OCI image layer identified by CID.

**Attributes**:
- `cid: worldcompute::types::Cid`
- `expected_digest: [u8; 32]` — SHA-256 digest declared in the OCI manifest
- `size_bytes: u64`
- `media_type: String` — e.g., `application/vnd.oci.image.layer.v1.tar+gzip`

**Validation rules**:
- `expected_digest` MUST match the SHA-256 of the fetched layer bytes.
- `size_bytes` MUST match the fetched length.

**Relationships**: Many-to-one with `OciManifest`.

---

### C.2 `OciManifest`

**Location**: `src/sandbox/firecracker/rootfs_builder.rs` (NEW)

**Purpose**: An OCI image manifest describing a bootable workload.

**Attributes**:
- `manifest_cid: Cid`
- `layers: Vec<OciLayer>` — ordered; applied in sequence
- `config: OciConfig` — entrypoint, env, user
- `rootfs_size_bytes: u64` — declared target rootfs size (default 1 GB)

**Validation rules**:
- `layers.len() >= 1`.
- `rootfs_size_bytes >= sum(layers.size_bytes)` × 1.1 (10 % overhead).

---

### C.3 `RootfsAssembly`

**Location**: `src/sandbox/firecracker/rootfs_builder.rs` (NEW)

**Purpose**: In-progress assembly of an ext4 rootfs.

**Attributes**:
- `target_file: PathBuf`
- `loopback_device: Option<PathBuf>` — e.g., `/dev/loop3`, dropped via scope-guard
- `mount_point: Option<PathBuf>` — e.g., `/tmp/wc-mnt-xyz`
- `status: AssemblyStatus` — enum `Created | Formatted | Mounted | Extracting | Complete | Failed`

**Validation rules** (invariants):
- If `status == Failed`, loopback and mount MUST be cleaned up before the struct drops.

**State transitions**:
```
Created → Formatted (mkfs.ext4 succeeds)
Formatted → Mounted (losetup + mount succeed)
Mounted → Extracting (first layer extraction begins)
Extracting → Complete (all layers extracted, umount + losetup -d succeed)
<any> → Failed (any error; scope-guard cleanup runs)
```

**Relationships**: One-per-workload; does not persist beyond a single Firecracker boot.

---

## Group D — Load metric

### D.1 `LoadSample`

**Location**: `src/agent/daemon.rs` (MUTATED; replaces stub `current_load()`)

**Purpose**: Real OS-derived load snapshot.

**Attributes**:
- `cpu_usage: f32` — 0.0..=1.0
- `gpu_usage: f32` — 0.0..=1.0, max across GPUs (0.0 if no GPUs)
- `memory_usage: f32` — 0.0..=1.0
- `sampled_at: chrono::DateTime<chrono::Utc>`

**Validation rules**:
- All three usage fields MUST be in `[0.0, 1.0]`.
- `sampled_at` MUST be within the last 500 ms at read time (cache invalidation).

**Derived field**: `overall = max(cpu_usage, gpu_usage, memory_usage)` — this is what the sovereignty supervisor consumes.

---

## Group E — Distributed-diffusion mesh LLM

### E.1 `DiffusionBackbone`

**Location**: `src/agent/mesh_llm_diffusion/backbone.rs` (NEW)

**Purpose**: A Dream-class 7B masked-discrete-diffusion LM loaded on a GPU node.

**Attributes**:
- `model_id: String` — e.g., `GSAI-ML/LLaDA-8B-Instruct`
- `weights_cid: Cid` — CID-mirrored weights
- `device: DeviceHandle` — candle Device enum (CUDA idx | Metal | CPU)
- `quantization: Quantization` — enum `None | Int8 | Int4Awq | Gguf(String)`
- `vocab_size: u32`
- `context_length: u32` — 2048 or 4096 typically

**Validation rules**:
- `model_id` MUST match one of the project-approved backbones (LLaDA 8B, Dream 7B, DiffuLLaMA) unless `--allow-experimental-backbone`.
- `weights_cid` MUST verify against the CID store's SHA-256.

**Relationships**: One-per-GPU-node (typically); many backbones in the swarm.

---

### E.2 `DiffusionExpert`

**Location**: `src/agent/mesh_llm_diffusion/expert.rs` (NEW)

**Purpose**: A small SSD-2-style specialized diffusion expert that contributes a conditional score signal.

**Attributes**:
- `expert_id: ExpertId` — unique ID in the mesh
- `specialization_domain: String` — e.g., `"code"`, `"math"`, `"planning"`
- `weights_cid: Cid`
- `backbone_compat_version: u32` — must match the backbone's tokenizer/dims
- `guidance_weight: f32` — default 1.0, operator-tunable per request

**Validation rules**:
- `guidance_weight >= 0.0`
- `backbone_compat_version` MUST match the `DiffusionBackbone` it's paired with.

**Relationships**: Many-to-one with `DiffusionBackbone` (compatible experts per backbone); many-to-many with `DiffusionRequest` (a request can select multiple experts).

---

### E.3 `DiffusionRequest`

**Location**: `src/agent/mesh_llm_diffusion/service.rs` (NEW)

**Purpose**: A single client-submitted prompt for distributed-diffusion inference.

**Attributes**:
- `request_id: uuid::Uuid`
- `prompt: String`
- `selected_backbone_peers: Vec<PeerId>` — typically 1
- `selected_experts: Vec<ExpertId>` — typically 2+ with `guidance_weight`
- `denoising_steps: u32` — default 64
- `paradigms_parallel_block_size: u32` — default 4
- `distrifusion_staleness: u32` — default 1 (0 means fully synchronous)
- `safety_tier: SafetyTier` — as today
- `clipping_tau: f32` — PCG clipping bound, default 10.0

**Validation rules**:
- `selected_experts.len() >= 1`.
- `denoising_steps` in `[8, 256]`.
- Sum of `guidance_weights` > 0.

**State transitions**:
```
Pending → InProgress (router dispatches)
InProgress → Halted (kill switch fires; halts before next denoising step per FR-029)
InProgress → Failed (ParaDiGMS non-convergence or any node failure)
InProgress → Completed (final denoising step returns)
```

---

### E.4 `DenoisingStepRecord`

**Location**: `src/agent/mesh_llm_diffusion/pcg.rs` (NEW)

**Purpose**: Per-step audit record for PCG score composition (supports the auditable-per-expert-weights requirement of FR-023).

**Attributes**:
- `step_index: u32` — 0..denoising_steps
- `per_expert_scores: Vec<(ExpertId, f32)>` — norm of each expert's score vector
- `per_expert_weights: Vec<(ExpertId, f32)>`
- `clipping_activated_for: Vec<ExpertId>` — experts whose scores were clipped
- `composed_score_norm: f32`
- `timestamp: chrono::DateTime<chrono::Utc>`

**Validation rules**: `clipping_activated_for ⊆ per_expert_scores.keys()`.

**Relationships**: Many-to-one with `DiffusionRequest`. Emitted as a telemetry event for the > 10 %-clipping observability signal.

---

### E.5 `ParaDiGMSBlock`

**Location**: `src/agent/mesh_llm_diffusion/paradigms.rs` (NEW)

**Purpose**: A block of denoising steps solved in parallel via Picard iteration.

**Attributes**:
- `block_start: u32`
- `block_size: u32`
- `convergence_threshold: f32` — default 1e-3
- `max_iterations: u32` — default 10
- `iterations_used: Option<u32>` — set on completion
- `converged: bool` — false triggers sequential fallback
- `wall_clock_ms: u32`

**Validation rules**: `block_size >= 2`; otherwise sequential is the right path.

---

### E.6 `DistriFusionActivation`

**Location**: `src/agent/mesh_llm_diffusion/distrifusion.rs` (NEW)

**Purpose**: A stale activation tensor transmitted between diffusion workers to pipeline communication behind compute.

**Attributes**:
- `source_peer: PeerId`
- `destination_peer: PeerId`
- `step_index: u32`
- `tensor_cid: Cid` — CID-addressed activation tensor (zstd-compressed CBOR fp16)
- `staleness: u32` — timesteps between production and consumption (default 1)

**Validation rules**:
- `staleness <= 3`.

**Relationships**: Transported via `/worldcompute/diffusion-activation/1.0.0` libp2p request-response protocol.

---

## Group F — Placeholder elimination state

### F.1 `PlaceholderAllowlistEntry`

**Location**: `.placeholder-allowlist` (NEW, repository-root text file) — NOT a Rust struct

**Purpose**: A single line in the allowlist text file; documented here for completeness.

**Format**: `<path>:<line> — <rationale>\n`

**Validation rules** (enforced by `scripts/verify-no-placeholders.sh`):
- `<path>` MUST start with `src/`.
- `<line>` MUST be a positive integer.
- `<rationale>` MUST be non-empty.
- **During spec-005 implementation, the file MUST be empty (FR-038, SC-006).**

---

## Group G — Evidence artifacts

### G.1 `EvidenceBundle`

**Location**: `evidence/phase1/<area>/<timestamp>/` (filesystem only; no Rust struct required)

**Purpose**: The committed bundle of artifacts produced by a real-hardware test run.

**Required files**:
- `run.log` — full stderr/stdout
- `metadata.json` — `{run_id, area, machines: [...], software_version, git_sha, started_at, ended_at}`
- `results.json` — `{assertions: [{name, expected, observed, pass}], overall: pass|fail}`
- `trace.jsonl` — NDJSON event trace (optional)
- `screenshots/*.png` — optional
- `index.md` — human-readable summary + links

**Validation rules**:
- `metadata.json.git_sha` MUST match HEAD of the `005-production-readiness` branch at evidence-commit time.
- `results.json.overall == "pass"` required for release-stable tagging (where applicable).

---

## Entity-relationship summary

```
NetworkIdentity 1..N RelayReservation (reservations per peer)
NodeBehaviour 1..1 WssTransportConfig
Daemon 1..1 DohResolverConfig
Daemon 1..N DialAttempt (log events)
ReleaseArtifact 1..3 PinnedRootCa + PinnedRekorKey (compile-time)
DriftCheckResult 1..* (weekly)
Workload 1..1 OciManifest 1..N OciLayer
OciManifest 1..1 RootfsAssembly (per boot)
Daemon 1..1 LoadSample (cached 500ms)
DiffusionRequest 1..N DiffusionBackbone (selected backbone peers)
DiffusionRequest 1..N DiffusionExpert (selected experts)
DiffusionRequest 1..N DenoisingStepRecord
DiffusionRequest 1..N ParaDiGMSBlock
DiffusionStep 1..N DistriFusionActivation (stale-activation transmissions)
RepoRoot 1..1 .placeholder-allowlist (empty at spec-005 completion)
FeatureMilestone 1..N EvidenceBundle
```

All entities are ready for contract extraction in Phase 1 step 2.
