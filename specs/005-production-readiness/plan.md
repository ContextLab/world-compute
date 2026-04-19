# Implementation Plan: Production Readiness — eliminate all placeholders and cross firewalls

**Branch**: `005-production-readiness` | **Date**: 2026-04-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-production-readiness/spec.md`

## Summary

Spec 005 closes every remaining gap between "code ships" and "production-ready volunteer compute federation." It has three load-bearing themes: (1) **make the mesh actually form across real institutional firewalls** (issue #60) by adding a WebSocket-over-TLS-443 fallback transport, a DoH resolver fallback, hardened relay-reservation logic, and project-operated launch relays; (2) **eliminate every placeholder and stub** — all 16 inline placeholder sites identified by direct code audit (AMD/Intel/Rekor `[0u8; 32]` constants, `ban()` no-op, `load_model` placeholder, `current_load` constant, `assemble_rootfs` byte-concat, `b"placeholder-disk"`, `governance_service` stubs, etc.), enforced by a hard-blocking CI check with empty-allowlist precondition; (3) **replace the architecturally-wrong autoregressive mesh-LLM with a distributed-diffusion swarm** per `notes/parallel_mesh_of_diffusers_whitepaper.pdf` — Dream-class 7B masked-diffusion backbone + SSD-2 specialized experts + PCG score composition + ParaDiGMS parallel denoising + DistriFusion stale-activation pipelining. Supporting work includes deep attestation with real pinned root CAs and CI drift detection, real Firecracker rootfs assembly (mkfs.ext4 + loopback + OCI tar extraction), a real 72-hour churn harness, real platform-adapter enrollment (Slurm/K8s/cloud free-tier), Tauri GUI + Dockerfile + Helm + REST gateway built and smoke-tested, and reproducible signed releases with an empty-allowlist `.placeholder-allowlist` as the spec-completion gate.

## Technical Context

**Language/Version**: Rust stable 1.95+ (current CI matrix is 1.95.0 on Linux/macOS/Windows + Sandbox KVM + swtpm). Secondary languages: Swift 5.9+ for Apple VF helper binary (macOS-only); TypeScript + React for Tauri GUI frontend; shell (bash) for operator scripts.
**Primary Dependencies**: libp2p 0.54 (+ new: `libp2p-websocket`, `libp2p-tls`/`libp2p-websocket-websys` for WSS-over-443 transport; `hickory-resolver` with DoH for FR-005); wasmtime 27; candle 0.7+ OR `diffusers-rs` / custom PyTorch-via-FFI for the diffusion backbone (pending research); tonic 0.12 (gRPC); ed25519-dalek 2, ecdsa 0.16, rsa 0.9 (attestation); threshold_crypto 0.4 (BLS); reed-solomon-erasure 6; openraft 0.9; opentelemetry 0.27; clap 4; reqwest 0.12; rcgen 0.13; oci-spec 0.7 + tar 0.4 + `loopdev` or `fscommon`-style library for real Firecracker rootfs; `sysinfo` 0.33 + `nvml-wrapper` 0.10 (GPU metrics for `current_load`); `tss-esapi` 7 or `tpm2-tss` for TPM2-backed confidential compute sealing; Tauri 2 for GUI; `kube` 0.96 + `k8s-openapi` for K8s CRD operator.
**Storage**: CID-addressed content store (SHA-256) with RS(10,18) erasure coding (already in place); CRDT OR-Map ledger with BLS threshold signing (already in place); per-donor working directory (size-capped, wiped on agent exit) — implemented, no change.
**Testing**: `cargo test` (900+ tests target, up from 802); `cargo clippy --lib -- -D warnings` (zero-warnings policy); `scripts/e2e-phase1.sh` for multi-machine real-hardware runs on tensor01+tensor02+laptop; `scripts/verify-no-placeholders.sh` for CI hard-block on placeholders; `scripts/verify-release.sh` for reproducible-build and signature verification; `tc qdisc netem` for controlled WAN-latency benchmarks on the diffusion mesh.
**Target Platform**: Linux (primary: Ubuntu 24.04 + Rocky Linux 9 + Debian 12, x86_64 + aarch64) — full feature set including Firecracker; macOS 14+ — Apple VF sandbox + Tauri GUI; Windows 11 — Hyper-V sandbox + Tauri GUI (limited). Institutional/corporate networks behind stateful firewalls explicitly targeted as first-class deployment environments (not edge cases).
**Project Type**: Mixed — primary is a Rust workspace (library + binary + adapters); secondary is a TypeScript/React Tauri desktop app; tertiary is a Swift helper binary for macOS. Uses the existing single-workspace layout (no fork into `backend/frontend` directories).
**Performance Goals**: Cross-firewall mesh join ≤ 60 s for first relay reservation (FR-001); relay-reservation recovery ≤ 60 s after loss (FR-006); WASM job dispatch over relay circuit ≤ 5 s end-to-end for trivial workload (SC-002); Firecracker boot + trivial entrypoint ≤ 10 s (SC-004); 72-hour churn @ 30% rotation ≥ 80% completion (SC-005); distributed-diffusion ParaDiGMS speedup ≥ 2× over sequential denoising on 6 GPUs (FR-025); DistriFusion pipelining masks ≥ 50% of 100 ms RTT behind compute (FR-026); quickstart → running donor ≤ 15 min on fresh machine (SC-008).
**Constraints**: Shoestring budget — no ongoing paid cloud infra beyond 1–2 project-operated fallback relays (cheapest viable VM per cloud); "max 3 GPUs per job per cluster" on tensor01/tensor02 (operator-enforced hardware budget); sub-second preemption yield to local human user (constitution Principle III, already in place, must not regress); zero placeholders in `src/` production code (hard CI block); empty `.placeholder-allowlist` as spec-completion gate (SC-006).
**Scale/Scope**: 94 existing Rust source files growing to ~120 (estimated new modules: `network/wss_transport`, `network/doh_resolver`, `sandbox/firecracker/rootfs_builder`, `agent/mesh_llm_diffusion/*` replacing `mesh_llm/*`, `data_plane/confidential/tpm2_seal`, operator scripts). 802 existing tests growing to ~950 (est.: +40 for cross-firewall paths, +30 for diffusion primitives, +25 for real Firecracker, +20 for placeholder elimination, +10 each for K8s/Slurm/cloud live paths). Target donor scale by spec end: demonstrable 3-real-machine cluster with documented path to 100+; target aggregate mesh size (separate milestone): 100k+ libp2p peers per Trautwein et al. 2025 measurement.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-evaluated after Phase 1 design.*

### Principle I — Safety First (Sandboxing & Host Integrity)

- **Pass.** Spec 005 strengthens safety: FR-008/009/011a replace permissive-bypass attestation with real pinned-root-CA verification and CI drift checks; FR-012/013/014 replace broken Firecracker rootfs with real ext4 + OCI extraction (the current byte-concat would not boot and thus trivially "passes" while providing zero isolation — a regression when hit in production); FR-030–FR-038 eliminate all placeholder code paths that today bypass safety checks. FR-031 wires real `ban()`. FR-032 wires real receipt verification. FR-034 wires TPM2-backed key sealing (or explicit removal if the attested-release path subsumes it). Every placeholder currently represents a safety gap; removing them is pure principle-I gain.
- **Tension to manage**: The WebSocket-over-TLS-443 fallback transport (FR-003) widens the attack surface for the mesh — SSL-inspecting middleboxes can MITM it. Mitigated by the pin-mismatch detection in Edge Cases (operator must explicitly opt into `--allow-ssl-inspection`) and marking inspected connections as a distinct trust tier.
- **Direct-test requirement**: Real adversarial test cases already required by constitution; spec 005 adds real firewall-traversal cases (tensor02 from outside tensor02), real attestation-chain-tampering tests, and real Firecracker-boot tests.

### Principle II — Robustness & Graceful Degradation

- **Pass.** FR-006 makes relay-reservation loss a first-class case with ≤ 60 s recovery. FR-014 ensures Firecracker rootfs assembly failures clean up loopback devices (no leaked state). FR-016/017 convert the existing statistical churn simulator into a real kill-rejoin harness — a direct upgrade to principle-II evidence. FR-026 DistriFusion stale-activation pipelining is explicitly a robustness primitive for WAN latency. FR-029 makes the mesh-LLM kill switch diffusion-step-granular (faster bounded-halt budget than token-granular).
- **Tension to manage**: Churn-harness work (FR-017) will stress-test the ledger's CRDT merge + BLS threshold paths more intensely than before; must not regress the existing 802-test pass.

### Principle III — Fairness & Donor Sovereignty

- **Pass.** FR-033 wires real `current_load()` (CPU+GPU+memory), which is what the preemption supervisor consumes to make sovereignty decisions. Today the constant-0.1 stub silently degrades principle-III compliance — fixing it is a direct principle gain. FR-007a (project-hosted fallback relays only at launch, retire-able to volunteers without client update) preserves the "no lock-in, volunteer-sustainable" posture. FR-020a (free-tier cloud CI only, maintainer-gated dispatch) avoids ongoing project cost that could compromise the volunteer model.
- **Tension to manage**: Project-operated fallback relays at launch (FR-007a) momentarily contradict the pure-volunteer ideal. Resolved by: (a) strict limit of 1–2 relays, (b) documented volunteer migration path, (c) gossip + peer-exchange ensures clients don't hard-code on them.

### Principle IV — Efficiency, Performance & Self-Improvement

- **Pass.** FR-025 ParaDiGMS parallel denoising and FR-026 DistriFusion pipelining are pure efficiency wins (2–4× wall-clock speedup, hide 50%+ of WAN RTT behind compute). FR-043/044 reproducible signed builds are the self-improvement mechanism for the build pipeline itself. FR-028a records measured speedups as evidence artifacts, tracking the efficiency metric over time. Diffusion architecture choice (FR-022) is specifically because it tolerates WAN latency better than AR — a structural efficiency decision.
- **Tension to manage**: The diffusion swarm requires more aggregate VRAM than a single-model AR deployment at comparable capability. Mitigated by SSD-2-style small experts: one Dream-class 7B backbone + many small specialized experts, which is cheaper per-marginal-capability than scaling a single model.

### Principle V — Direct Testing (NON-NEGOTIABLE)

- **Pass.** Every SC has a real-hardware direct-test plan: SC-001 on tensor02 behind real Dartmouth firewall; SC-002 cross-machine dispatch on real networks; SC-003 with real AMD EPYC + real tampered quote; SC-004 real Firecracker boot on KVM host; SC-005 real 72-hour churn run; SC-008 fresh VM timed quickstart; SC-010 real 6-GPU diffusion swarm. Evidence artifacts committed under `evidence/phase1/<area>/`.
- **This is the most rigorous principle-V plan produced by the project.** No new complexity-exception entries.

### Constitution Check verdict: **PASS** (zero violations, zero complexity-tracking entries required)

## Project Structure

### Documentation (this feature)

```text
specs/005-production-readiness/
├── plan.md              # This file
├── research.md          # Phase 0 output — resolves NEEDS CLARIFICATION items below
├── data-model.md        # Phase 1 output — entities (RelayReservation, WssTransport, DiffusionExpert, etc.)
├── contracts/           # Phase 1 output — interface contracts (CLI, gRPC, REST, CI scripts)
│   ├── cli-worldcompute.md
│   ├── grpc-mesh-llm-diffusion.md
│   ├── rest-gateway.md
│   ├── ci-verify-no-placeholders.md
│   └── evidence-artifact-format.md
├── quickstart.md        # Phase 1 output — 15-minute fresh-machine operator path
├── checklists/
│   └── requirements.md  # From /speckit.specify + /speckit.clarify
└── tasks.md             # Phase 2 output (/speckit.tasks, NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/                            # Rust workspace library; 94 files → ~120
├── acceptable_use/              # (unchanged)
├── agent/
│   ├── daemon.rs                # MUTATE: replace current_load() stub with real sysinfo+NVML-backed metric (FR-033)
│   ├── lifecycle.rs             # MUTATE: remove duplicate stub path OR wire gossipsub broadcast (FR-030)
│   ├── mesh_llm/                # REMOVE: entire AR-ensemble module (FR-022, FR-023)
│   └── mesh_llm_diffusion/      # NEW: distributed-diffusion mesh LLM replacement
│       ├── backbone.rs          # Dream-class 7B masked-diffusion loader + per-step score producer
│       ├── expert.rs            # SSD-2-style small specialized expert
│       ├── pcg.rs               # PCG (predictor-corrector) score composition
│       ├── paradigms.rs         # ParaDiGMS Picard-iteration parallel denoising
│       ├── distrifusion.rs      # Stale-activation pipelined transport
│       ├── scheduler.rs         # Denoising-step scheduler (replaces token scheduler)
│       ├── safety.rs            # Safety tier + kill-switch at denoising-step granularity (FR-029)
│       └── service.rs           # gRPC service — real inference RPC (FR-027)
├── cli/
│   └── submitter.rs             # MUTATE: add diffusion-prompt dispatch path
├── credits/                     # (unchanged)
├── data_plane/
│   ├── cid_store.rs             # (unchanged)
│   └── confidential.rs          # MUTATE: replace simplified seal placeholder with TPM2-backed seal OR remove if attested-release subsumes (FR-034)
├── governance/
│   ├── admin_service.rs         # MUTATE: ban() writes trust registry + broadcasts action (FR-031)
│   ├── governance_service.rs    # MUTATE: SubmitProposal + CastVote persist + emit audit events (FR-036)
│   └── ...
├── incident/                    # (unchanged)
├── ledger/
│   └── transparency.rs          # MUTATE: pin real Rekor public key; fail-build if zero in production feature (FR-010, FR-011a)
├── network/
│   ├── discovery.rs             # MUTATE: add project-operated launch relays to PUBLIC_LIBP2P_BOOTSTRAP_RELAYS (FR-007a)
│   ├── wss_transport.rs         # NEW: WebSocket-over-TLS-443 libp2p transport with automatic fallback (FR-003)
│   ├── doh_resolver.rs          # NEW: DoH-backed /dnsaddr/ resolver fallback (FR-005)
│   ├── dial_logging.rs          # NEW: surface every DialFailure at info+ with transport + root cause (FR-004)
│   └── relay_reservation.rs     # NEW: reservation-loss detection + alternate-relay reacquire within 60s (FR-006)
├── policy/
│   ├── engine.rs                # MUTATE: single-pass signed-builder (eliminate vec![0u8; 64] placeholder) (FR-037)
│   └── rules.rs                 # MUTATE: same (FR-037)
├── preemption/                  # (unchanged)
├── registry/                    # (unchanged)
├── sandbox/
│   ├── firecracker/             # NEW submodule:
│   │   ├── mod.rs               # Re-export of existing surface
│   │   ├── rootfs_builder.rs    # NEW: real mkfs.ext4 + loopback + OCI tar extraction (FR-012, FR-013, FR-014)
│   │   └── vsock_io.rs          # NEW: vsock-based stdout/stderr capture
│   └── apple_vf.rs              # MUTATE: real disk prep on macOS OR Err::UnsupportedPlatform (FR-035)
├── scheduler/                   # (unchanged)
├── telemetry/                   # (unchanged)
├── verification/
│   ├── attestation.rs           # MUTATE: pin real AMD ARK + Intel DCAP fingerprints; fail-build in production feature (FR-008, FR-009, FR-011a)
│   └── receipt.rs               # MUTATE: wire coordinator public key; reject malformed/unsigned (FR-032)
├── error.rs                     # MUTATE: add new error variants (UnsupportedPlatform, DialFailureWithDetail, etc.)
├── features.rs                  # NEW: `production` cargo feature enforcement (fail-build on zero constants)
└── lib.rs                       # MUTATE: swap mesh_llm → mesh_llm_diffusion re-export

adapters/
├── slurm/                       # MUTATE: add live slurmrestd integration test against containerized Slurm (FR-018)
├── kubernetes/                  # MUTATE: Kind-in-CI CRD reconcile test (FR-019)
├── cloud/                       # MUTATE: free-tier IMDS tests gated to workflow_dispatch (FR-020, FR-020a)
└── apple_vf_helper/             # MUTATE: macOS-CI-built signed Swift helper binary (FR-021)

gui/
├── src-tauri/                   # MUTATE: wire the three primary flows + smoke tests (FR-039)
├── src/                         # React frontend — exercise flows via Playwright
└── tests/                       # NEW: Playwright smoke-test harness

ops/
├── Dockerfile                   # MUTATE: docker build passes in CI (FR-040)
├── docker-compose.yml           # (unchanged)
├── helm/                        # MUTATE: Kind-in-CI deploy + smoke test (FR-040)
└── release/
    ├── build-reproducible.sh    # NEW: two-runner bit-identical build (FR-043)
    ├── sign-release.sh          # NEW: detached Ed25519 signing (FR-044)
    └── verify-release.sh        # NEW: verify signature against pinned release public key (FR-044)

scripts/
├── e2e-phase1.sh                # NEW: stand up 3-machine real cluster, run workloads, emit evidence (FR-015)
├── churn-harness.sh             # NEW: real kill-rejoin over libp2p, 72h schedule (FR-017)
├── verify-no-placeholders.sh    # NEW: hard-block CI check (FR-038)
├── drift-check.sh               # NEW: CI weekly refetch AMD/Intel/Rekor values → open issue on mismatch (FR-011a)
├── quickstart-timed.sh          # NEW: verify 15-minute path on fresh VM (FR-042)
└── diffusion-smoke.sh           # NEW: 6-GPU cross-machine diffusion benchmark with tc netem (FR-028, FR-028a)

tests/                           # 802 existing → ~950
├── network/
│   ├── test_wss_transport.rs    # NEW (~15 tests)
│   ├── test_doh_resolver.rs     # NEW (~10 tests)
│   ├── test_relay_reservation.rs # NEW (~10 tests)
│   └── test_firewall_traversal.rs # NEW (~5 tests — real tensor02)
├── verification/
│   ├── test_real_attestation.rs # NEW (~15 tests, real AMD EPYC quote required)
│   └── test_rekor_real.rs       # NEW (~10 tests, live rekor.sigstore.dev)
├── sandbox/
│   └── firecracker/
│       └── test_real_rootfs.rs  # NEW (~15 tests, KVM required)
├── diffusion/
│   ├── test_backbone.rs         # NEW (~10)
│   ├── test_pcg.rs              # NEW (~10)
│   ├── test_paradigms.rs        # NEW (~10)
│   ├── test_distrifusion.rs     # NEW (~10)
│   └── test_e2e_diffusion.rs    # NEW (~5 — real 6-GPU)
├── adapters/
│   ├── test_slurm_live.rs       # NEW (~5 — containerized slurm)
│   ├── test_k8s_live.rs         # NEW (~5 — Kind)
│   └── test_cloud_live.rs       # NEW (~10 — workflow_dispatch only)
└── integration/
    ├── test_placeholder_cleanup.rs # NEW (~10 — assert each FR-030..037 fix)
    └── test_churn_72h_harness.rs   # NEW (~5 — smoke only; full 72h is evidence-artifact producer)

.placeholder-allowlist           # NEW: empty file; CI check enforces this at spec completion (FR-038, SC-006)
evidence/
└── phase1/                      # NEW: populated by scripts/ as real-hardware runs complete
    ├── firewall-traversal/
    ├── attestation/
    ├── diffusion-mesh/
    ├── cloud-adapter/
    ├── churn/
    └── quickstart/

.github/workflows/
├── drift-check.yml              # NEW: weekly AMD/Intel/Rekor refresh → issue (FR-011a)
├── reproducible-build.yml       # NEW: two-runner bit-identical check (FR-043)
├── cloud-live-tests.yml         # NEW: workflow_dispatch, maintainer-gated (FR-020a)
└── verify-no-placeholders.yml   # NEW: runs scripts/verify-no-placeholders.sh on every PR (FR-038)
```

**Structure Decision**: Single Rust workspace (existing) with the addition of (a) a fully new distributed-diffusion module replacing the AR-ensemble mesh_llm module, (b) new network/ submodules for WSS-443 and DoH fallback, (c) a new sandbox/firecracker/ submodule for real rootfs assembly, (d) ops/ and scripts/ for reproducible release pipeline and evidence-artifact-producing harnesses, (e) a mandatory empty `.placeholder-allowlist` at repository root as the spec-completion gate. The Tauri GUI and platform adapters already live in their respective subdirectories and are mutated in place rather than restructured.

## Complexity Tracking

> No Constitution Check violations. Table left empty by design.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-|-|-|
| _(none)_ | | |
