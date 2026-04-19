# Implementation Plan: Full Functional Implementation

**Branch**: `004-full-implementation` | **Date**: 2026-04-17 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-full-implementation/spec.md`
**Master Issue**: #57 — 28 sub-issues (#28–#56)

## Summary

Complete the World Compute system from "stubs replaced" to "fully functional distributed system" by implementing all 28 sub-issues from the master plan. This covers deep cryptographic verification, real agent lifecycle, complete policy engine, adversarial testing, platform adapters, runtime systems (credits, scheduler, ledger), GUI, deployment infrastructure, and the distributed mesh LLM. Every component must pass real-hardware tests on `tensor01.dartmouth.edu` and CI.

## Technical Context

**Language/Version**: Rust stable (tested on 1.95.0)
**Primary Dependencies**: libp2p 0.54, tonic 0.12, ed25519-dalek 2, wasmtime 27, openraft 0.9, opentelemetry 0.27, clap 4, reqwest 0.12, oauth2 4, x509-parser 0.16, reed-solomon-erasure 6, cid 0.11, multihash 0.19
**New Dependencies Required**: rsa (for cert chain verification), ecdsa/p256/p384 (for ECDSA verification), aes-gcm (for confidential compute), chacha20poly1305 (alternative cipher), rcgen (cert generation), tokio-rustls (mTLS), threshold-crypto (threshold signing), sysinfo (energy metering), k8s-openapi + kube (K8s adapter)
**Storage**: CID-addressed content store (SHA-256), erasure-coded RS(10,18)
**Testing**: cargo test + cargo clippy --lib -- -D warnings + real hardware on tensor01.dartmouth.edu
**Target Platform**: Linux (primary), macOS, Windows
**Project Type**: CLI + library + desktop app (Tauri) + adapters
**Performance Goals**: 10ms preemption latency, 3.2 tokens/sec mesh LLM, 80% completion at 30% churn
**Constraints**: Zero host residue on withdrawal, default-deny egress, no unsafe code
**Scale/Scope**: 94+ source files → ~150+, 489 tests → 700+, 20 modules

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-|-|-|
| I. Safety First | PASS | FR-001 (crypto attestation), FR-011 (adversarial tests), FR-012/13 (confidential compute), FR-016/17 (code signing), all containment primitives enforced |
| II. Robustness | PASS | FR-019 (80% at 30% churn), FR-028a (graceful degradation on quorum loss), checkpoint/resume throughout |
| III. Fairness & Donor Sovereignty | PASS | FR-005 (10ms preemption), FR-004 (zero host residue), FR-025 (credit decay with floor protection) |
| IV. Efficiency & Self-Improvement | PASS | FR-034 (energy metering), FR-037-043 (mesh LLM self-improvement) |
| V. Direct Testing | PASS | SC-012 (every FR has passing test on real hardware), real hardware testing on tensor01.dartmouth.edu |

**Gate Result**: PASS — all five principles satisfied by functional requirements.

## Project Structure

### Documentation (this feature)

```text
specs/004-full-implementation/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── attestation-contract.md
│   ├── containment-contract.md
│   ├── scheduler-contract.md
│   └── mesh-llm-contract.md
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/                        # ~150+ Rust source files after this spec
  acceptable_use/           # + real filter enforcement
  agent/                    # + heartbeat, pause, withdraw, lifecycle
    mesh_llm/               # NEW: router, expert, aggregator, safety, self-prompt
  cli/                      # + enhanced commands
  credits/                  # + 45-day decay, DRF accounting
  data_plane/               # + storage GC, confidential compute
  governance/               # existing — minor additions
  identity/                 # existing — integration tests
  incident/                 # + real containment enforcement
  ledger/                   # + threshold signing, CRDT merge
  network/                  # + mTLS, rate limiting
  policy/                   # + artifact registry, egress allowlist
  preemption/               # + real supervisor wiring
  registry/                 # existing — integration tests
  sandbox/                  # + GPU verification, rootfs prep
  scheduler/                # + real broker matchmaking, lease mgmt
  telemetry/                # + energy metering
  verification/             # + deep crypto chain verification

tests/                      # 700+ tests after this spec
  acceptable_use/           # NEW
  adversarial/              # 8 tests fully implemented (no #[ignore])
  agent/                    # NEW
  cli/                      # NEW
  contract/                 # populate or remove
  credits/                  # NEW
  data_plane/               # NEW
  egress/                   # existing
  governance/               # existing
  identity/                 # existing + expanded
  incident/                 # existing + containment tests
  integration/              # populate or remove
  ledger/                   # NEW
  mesh_llm/                 # NEW
  network/                  # NEW
  policy/                   # existing + expanded
  preemption/               # NEW
  red_team/                 # existing
  registry/                 # NEW
  sandbox/                  # existing + expanded
  scheduler/                # NEW
  telemetry/                # NEW
  verification/             # NEW
  churn/                    # NEW: 72-hour churn simulator

adapters/
  slurm/src/main.rs         # + real slurmrestd/sbatch integration
  kubernetes/src/main.rs    # + CRD watch loop, Pod creation
  cloud/src/main.rs         # + IMDSv2/metadata attestation

gui/src-tauri/
  src/main.rs               # + real Tauri window + backend IPC
  src/commands.rs            # + real command handlers

tools/
  apple-vf-helper/          # Swift binary for Virtualization.framework
```

**Structure Decision**: Extends existing Cargo workspace layout. No structural reorganization needed — new code goes into existing module directories, new test files mirror src/ structure.

## Complexity Tracking

> No constitution violations to justify.

## Implementation Phases

### Phase A: Core Infrastructure Depth (Issues #28, #29, #30, #31, #32, #33, #34, #45)

**Priority**: P1 — blocks everything else
**Estimated scope**: ~2000 lines of Rust across 8 modules
**Dependencies**: None — works on existing codebase

1. **Deep attestation (#28)**: Add rsa + ecdsa crates. Implement real RSA/ECDSA signature verification in `CertificateChainValidator` implementations. Pin AMD ARK/Intel DCAP root fingerprints as compile-time constants. Add certificate expiry and revocation checking.

2. **Rekor Merkle proofs (#29)**: Implement RFC 6962 Merkle inclusion proof verification in `src/ledger/transparency.rs`. Verify leaf hash → proof path → signed tree root. Pin Rekor public key for signature verification.

3. **Agent lifecycle (#30)**: Wire `heartbeat()`, `pause()`, `withdraw()` in `src/agent/lifecycle.rs`. Heartbeat sends periodic state to broker via gossipsub. Pause checkpoints all sandboxes. Withdraw wipes working directory and revokes keypair.

4. **Policy engine (#31)**: Implement artifact CID resolution in `check_artifact_registry()`. Add `allowed_endpoints` field to JobManifest and implement `check_egress_allowlist()`. Wire LLM advisory flag (initially always false until mesh LLM is built).

5. **GPU passthrough (#32)**: Implement PCI device enumeration via sysfs on Linux. Check IOMMU groups. Detect ACS-override patch. Report GPU capabilities.

6. **Firecracker rootfs (#33)**: Implement OCI image fetch from CID store → layer extraction → ext4 filesystem assembly → mount as Firecracker root drive.

7. **Incident containment (#34)**: Wire enforcement effects: FreezeHost sends SIGSTOP, QuarantineWorkloadClass updates policy engine, BlockSubmitter cancels in-flight jobs, RevokeArtifact halts affected jobs, DrainHostPool migrates workloads.

8. **Preemption supervisor (#45)**: Wire `event_rx` channel. On sovereignty trigger → SIGSTOP all sandbox PIDs within 10ms → attempt checkpoint within 500ms → SIGKILL fallback. Log measured latency.

**Test plan**: Integration tests for each module on tensor01.dartmouth.edu. Real TPM2 quotes if available, AMD/Intel test vectors otherwise. Real STUN/Rekor calls. Measured preemption latency.

### Phase B: Security Hardening (Issues #35, #46, #47, #53)

**Priority**: P1 — required before external deployment
**Estimated scope**: ~1500 lines
**Dependencies**: Phase A (containment, attestation)

1. **Adversarial tests (#35)**: Implement all 8 `#[ignore]` tests. Sandbox escape: attempt ptrace/container escape inside Firecracker. Network isolation: attempt host bridge/DNS intercept. Byzantine: inject wrong results, verify detection. Flood: malformed gossip for 60s.

2. **Confidential compute (#46)**: Add aes-gcm crate. Implement client-side AES-256-GCM encryption. Per-job ephemeral keys wrapped with submitter public key. TPM-attested key release for confidential-medium. Guest-measurement sealed keys for confidential-high.

3. **mTLS and rate limiting (#47)**: Add rcgen + tokio-rustls. Ed25519 cert issuance, 90-day auto-rotation. Token bucket rate limiter per contracts/README.md classes.

4. **Supply chain (#53)**: Reproducible build configuration. Ed25519 binary signing. Agent version verification on heartbeat.

**Test plan**: Run adversarial tests on tensor01.dartmouth.edu with KVM. Encrypt/decrypt round-trip. mTLS handshake test. Rate limit exceed → verify 429.

### Phase C: Test Coverage + Validation (Issues #36, #51, #42)

**Priority**: P1 — Principle V gate
**Estimated scope**: ~3000 lines of tests
**Dependencies**: Phases A and B (need real implementations to test)

1. **Integration tests (#36)**: Add integration tests for all 12 untested modules: acceptable_use, agent, cli, credits, data_plane, ledger, network, preemption, registry, scheduler, telemetry, verification. Each module gets 3+ tests.

2. **Churn simulator (#51)**: Build configurable churn harness. Random node kill/rejoin at configurable rate. Track job completion rates. Target: 80% at 30% churn over 72 hours. Can run as background process on tensor01.

3. **Phase 1 LAN testnet (#42)**: Deploy 3+ agent instances on tensor01 (use separate processes/containers to simulate physical machines). Verify mDNS discovery <5s, R=3 job, preemption, failure recovery. Generate evidence artifact.

**Test plan**: This IS the testing phase. Run full suite, measure coverage, produce evidence artifacts.

### Phase D: Runtime Systems (Issues #44, #49, #55, #56)

**Priority**: P2 — sustained operation
**Estimated scope**: ~2500 lines
**Dependencies**: Phase A (scheduler, ledger foundations)

1. **Credits (#44)**: Implement 45-day half-life decay. Floor protection. DRF dominant-dimension accounting. Anti-hoarding mechanism.

2. **Storage GC and acceptable-use (#49)**: Per-donor storage cap tracking. GC for expired/orphaned data. Content classification at submission. Shard residency enforcement.

3. **Scheduler (#55)**: ClassAd-style matchmaking (task requirements ↔ agent capabilities). Lease issuance with TTL. Heartbeat renewal. Expired lease rescheduling. R=3 disjoint-AS placement.

4. **Ledger (#56)**: Add threshold-crypto crate. Implement t-of-n threshold signing (3-of-5). CRDT OR-Map merge. Cross-shard MerkleRoot every 10 minutes. Local balance verification.

**Test plan**: Simulate 90-day credit scenarios. Fill storage to cap. Multi-node matchmaking. 5-coordinator threshold signing.

### Phase E: Platform Adapters (Issues #37, #38, #39, #52)

**Priority**: P2 — extends reach
**Estimated scope**: ~2000 lines across 4 adapter crates
**Dependencies**: Phase A (scheduler for task dispatch)

1. **Slurm (#37)**: Connect to slurmrestd REST API. Advertise capacity. Dispatch via sbatch. Collect results via sacct.

2. **Kubernetes (#38)**: Add kube + k8s-openapi crates. Watch ClusterDonation CRD. Create Pods with resource limits. Collect results. Cleanup. Helm chart.

3. **Cloud (#39)**: AWS IMDSv2 token → identity document → verify. GCP metadata → JWT → verify. Azure IMDS → attested data → verify.

4. **Apple VF (#52)**: Swift binary using VZVirtualMachine. JSON command protocol on stdin/stdout. Create/start/pause/resume/stop/checkpoint.

**Test plan**: Slurm on tensor01 if available, otherwise minimal 2-node setup. K8s on minikube in CI. Cloud on spot instance. Apple VF on macOS dev machine.

### Phase F: User-Facing Features (Issues #40, #43, #48, #50, #41)

**Priority**: P2 — public-facing operation
**Estimated scope**: ~4000 lines (Rust + TypeScript + config)
**Dependencies**: Phases A-D (need working backend)

1. **Tauri GUI (#40)**: Initialize real Tauri window. React/TypeScript frontend. Donor dashboard, submitter dashboard, governance board, settings page. WCAG 2.1 AA compliance.

2. **REST gateway (#43)**: tonic-web or custom HTTP+JSON gateway from proto files. Rate limiting. Auth via Ed25519 tokens.

3. **Energy metering (#48)**: RAPL on Intel Linux, PowerCap on Linux, IOReport on macOS. Per-node CPU/GPU-time reporting. Regional carbon intensity calculation.

4. **Documentation (#50)**: README with working quickstart. Evidence artifact JSON schema. Incident disclosure policy. Legal placeholders.

5. **Deployment (#41)**: Multi-stage Dockerfile. Docker Compose for 3-node local cluster. Helm chart for coordinator deployment. Release pipeline.

**Test plan**: Launch Tauri on each platform. REST API integration tests. Energy estimates vs wall-meter on tensor01. Follow quickstart on clean machine. Docker compose cluster test.

### Phase G: Distributed Mesh LLM (Issue #54)

**Priority**: P3 — requires GPU nodes
**Estimated scope**: ~3000 lines
**Dependencies**: Phases A-D (need functioning cluster)

1. **Router** (`src/agent/mesh_llm/router.rs`): K-of-N expert selection per token. LLaMA-3 tokenizer (128K vocab). Expert health tracking.

2. **Expert node** (`src/agent/mesh_llm/expert.rs`): Registration with router. Health reporting. Capacity advertisement. Model loading (LLaMA-3-8B at 4-bit).

3. **Aggregator** (`src/agent/mesh_llm/aggregator.rs`): Receive top-256 logits from K experts. Weighted average. Temperature sampling.

4. **Self-prompting loop** (`src/agent/mesh_llm/self_prompt.rs`): Observe cluster metrics. Generate improvement tasks. 1-24 hour cadence.

5. **Safety system** (`src/agent/mesh_llm/safety.rs`): Action tier classification. Governance kill switch. Revert last 3 changes on kill.

6. **gRPC service**: RegisterExpert, GetRouterStatus, SubmitSelfTask, HaltMesh handlers.

**Test plan**: Deploy 4+ GPU nodes (cloud spot instances if tensor01 lacks GPUs). Generate 100 tokens. Measure tokens/second. Test kill switch. Test self-prompting output quality.

## Risk Register

| Risk | Impact | Mitigation |
|-|-|-|
| tensor01 lacks GPU hardware | Mesh LLM testing blocked | Use cloud spot instances (AWS g4dn.xlarge ~$0.50/hr) |
| tensor01 lacks KVM | Firecracker tests blocked | Verify KVM support first; fall back to WASM-only testing |
| Slurm not available on tensor01 | Adapter test blocked | Deploy minimal 2-node Slurm or test on partner cluster |
| 72-hour churn sim takes too long | Blocks Phase C completion | Run as background job; proceed with other phases |
| Mesh LLM quality insufficient at 4-node scale | Demo not compelling | Focus on correctness (token generation works) not quality |
| Apple VF requires macOS hardware | Cannot test in CI | Test on developer workstation; CI tests macOS compilation only |

## Execution Strategy

1. Start Phase A immediately — all tasks are independent and can be parallelized
2. Phase B starts when Phase A attestation and containment are complete
3. Phase C starts when Phase B adversarial tests are ready
4. Phases D-F can overlap — they touch different modules
5. Phase G starts last — requires Phases A-D and GPU hardware
6. The 72-hour churn sim (Phase C) runs as a background job while other work continues
7. All phases must pass `cargo test` + `cargo clippy --lib -- -D warnings` + `cargo fmt --check`
8. Each phase produces a commit with passing CI before the next phase begins
