---
description: "Task list for spec 005-production-readiness implementation"
---

# Tasks: Production Readiness — eliminate all placeholders and cross firewalls

**Input**: Design documents from `/Users/jmanning/world-compute/specs/005-production-readiness/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Included. Required by constitution Principle V (Direct Testing, NON-NEGOTIABLE) and by CLAUDE.md ("all tests need to use real function calls"). Every user story gets real-hardware tests where hardware applies.

**Organization**: Tasks are grouped by user story. US1 (cross-firewall mesh), US2 (deep attestation), US3 (real Firecracker), US4 (Phase-1 cluster + churn) are P1. US5 (platform adapters) and US6 (diffusion mesh-LLM) and US7 (placeholder elimination) are P2. US8 (operations) is P3.

## Path Conventions

- Rust workspace at repository root `/Users/jmanning/world-compute/`.
- `src/` = library + binary code; `tests/` = integration tests; `adapters/` = platform adapters; `gui/` = Tauri GUI; `scripts/` = operator scripts; `ops/` = deployment; `.github/workflows/` = CI; `evidence/phase1/` = real-hardware artifacts.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add dependencies, feature flags, and the placeholder-elimination CI tooling that blocks every subsequent task.

- [ ] T001 Add new Cargo workspace dependencies to [Cargo.toml](Cargo.toml): `libp2p-websocket`, `libp2p-tls`, `hickory-resolver` with DoH feature, `sysinfo = "0.33"`, `nvml-wrapper = "0.10"`, `tss-esapi = "7"`, `oci-spec = "0.7"`, `tar = "0.4"`, `nix = { version = "0.29", features = ["mount", "fs"] }`, `candle-core = "0.7"`, `candle-nn = "0.7"`, `candle-transformers = "0.7"`. Pin all versions.
- [ ] T002 [P] Add `production` cargo feature gate in [Cargo.toml](Cargo.toml) and create [src/features.rs](src/features.rs) with compile-time `const _: () = assert!(...)` checks that fail the build when any of `AMD_ARK_SHA256_FINGERPRINT`, `INTEL_ROOT_CA_SHA256_FINGERPRINT`, `REKOR_PUBLIC_KEY` are all-zero under `feature = "production"`.
- [ ] T003 [P] Create empty [.placeholder-allowlist](.placeholder-allowlist) file at repository root with a single comment line explaining the format (per contracts/ci-verify-no-placeholders.md).
- [ ] T004 [P] Author [scripts/verify-no-placeholders.sh](scripts/verify-no-placeholders.sh) implementing the grep + allowlist logic per contracts/ci-verify-no-placeholders.md; exit codes 0/64/65; support `--list` and `--check-empty` flags.
- [ ] T005 [P] Create [.github/workflows/verify-no-placeholders.yml](.github/workflows/verify-no-placeholders.yml) that runs `scripts/verify-no-placeholders.sh` on every PR + push; runs with `--check-empty` on the `005-production-readiness` branch and on merges to `main`.
- [ ] T006 [P] Create [evidence/phase1/](evidence/phase1/) directory structure with subdirectories `firewall-traversal`, `attestation`, `firecracker-rootfs`, `diffusion-mesh`, `cloud-adapter`, `churn`, `quickstart` plus a top-level `README.md` explaining the format per contracts/evidence-artifact-format.md.
- [ ] T007 [P] Author [scripts/validate-evidence.sh](scripts/validate-evidence.sh) per contracts/evidence-artifact-format.md (validates `metadata.json`, `results.json`, file presence, size limits).
- [ ] T008 Update [CLAUDE.md](CLAUDE.md) remaining-stubs section to reference this spec's completion gate (empty `.placeholder-allowlist`) and remove the stale "Remaining Stubs and Placeholders" inventory (it moves into spec.md Background).

**Checkpoint**: Dependencies available, CI hard-block script in place, evidence scaffolding ready.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Infrastructure every user story depends on — new error variants, shared types, and the feature gate.

- [ ] T009 Add new error variants to [src/error.rs](src/error.rs): `UnsupportedPlatform`, `DialFailureWithDetail(String)`, `ReservationAcquisitionFailed`, `ParaDiGMSNonconvergence`, `AttestationRootMismatch`, `PlaceholderDetected`. Wire each to appropriate gRPC + HTTP status codes.
- [ ] T010 [P] Add new types to [src/types.rs](src/types.rs): `ReservationStatus`, `TransportKind`, `DialOutcome`, `SafetyTier` (if not already there), `ExpertId` (UUID wrapper), `DenoisingStep(u32)`.
- [ ] T011 [P] Wire the `production` cargo feature through [src/lib.rs](src/lib.rs) and [src/main.rs](src/main.rs) — the production binary built for release tags MUST set this feature.
- [ ] T012 Author [docs/releases.md](docs/releases.md) documenting the release procedure: drift-check gate, pin-at-release constants, two-runner reproducible build, evidence artifact requirements per SC (per plan.md + contracts/evidence-artifact-format.md).

**Checkpoint**: All user stories can now begin independently in parallel.

---

## Phase 3: User Story 1 — Cross-firewall mesh formation (Priority: P1) 🎯 MVP

**Goal**: Donor daemon on a machine behind a stateful institutional firewall joins the mesh, maintains a relay reservation for ≥ 10 min, and is reachable by remote dispatch. Per FR-001 through FR-007a.

**Independent Test**: Deploy on `tensor02.dartmouth.edu` (behind Dartmouth firewall). Run daemon in foreground. From laptop on different network, dispatch a real WASM job via the reserved circuit. Capture log + evidence to `evidence/phase1/firewall-traversal/<ts>/`. Assert: reservation persists 10+ min; job returns `Succeeded` with verified receipt.

### Tests for User Story 1

- [ ] T013 [P] [US1] Write integration test [tests/network/test_wss_transport.rs](tests/network/test_wss_transport.rs) exercising the WebSocket-over-TLS-443 transport: listener on 443, dial, handshake, echo. Use real rustls via `libp2p-tls`.
- [ ] T014 [P] [US1] Write integration test [tests/network/test_doh_resolver.rs](tests/network/test_doh_resolver.rs) against real Cloudflare + Google DoH endpoints (network-required test; mark it so it's skipped in offline CI but runs in normal CI).
- [ ] T015 [P] [US1] Write integration test [tests/network/test_relay_reservation.rs](tests/network/test_relay_reservation.rs) exercising `ReservationStatus` state machine including forced loss + reacquire-within-60s (FR-006).
- [ ] T016 [P] [US1] Write integration test [tests/network/test_dial_logging.rs](tests/network/test_dial_logging.rs) asserting every `DialFailure` event surfaces at `info` level with transport + root cause (FR-004).
- [ ] T017 [US1] Write real-hardware test [tests/network/test_firewall_traversal.rs](tests/network/test_firewall_traversal.rs) — runs daemon, dials public Protocol Labs relay + project fallback relay via WSS-443, waits for `ReservationReqAccepted`, dispatches local WASM, asserts round-trip success. Marked `#[ignore]` by default; run via `cargo test --ignored -- test_firewall_traversal` from tensor02.

### Implementation for User Story 1

- [ ] T018 [P] [US1] Implement [src/network/wss_transport.rs](src/network/wss_transport.rs) per data-model.md A.2 (FR-003): `WssTransportConfig` + `build_wss_transport()` function returning a `libp2p::Transport` composing `libp2p-websocket` + `libp2p-tls` + `yamux`. Support listen + dial.
- [ ] T019 [P] [US1] Implement [src/network/doh_resolver.rs](src/network/doh_resolver.rs) per data-model.md A.4: wrap `hickory-resolver` in DoH mode with Cloudflare + Google upstreams; engage only on OS-resolver failure with 5 s timeout (FR-005).
- [ ] T020 [P] [US1] Implement [src/network/dial_logging.rs](src/network/dial_logging.rs) per data-model.md A.3: `DialAttempt` struct + `emit_dial_event()` function invoked from the swarm event loop on every `DialFailure` / `DialSuccess`.
- [ ] T021 [US1] Implement [src/network/relay_reservation.rs](src/network/relay_reservation.rs) per data-model.md A.1: `RelayReservation` struct + state machine + `reacquire_on_loss()` async task that fires within 60 s of a detected loss event.
- [ ] T022 [US1] Extend [src/network/discovery.rs](src/network/discovery.rs) to add project-operated launch relays to `PUBLIC_LIBP2P_BOOTSTRAP_RELAYS` with WSS/443 multiaddrs (FR-002, FR-007a). Add a config option to designate "is a relay server" which enables WSS-443 listener.
- [ ] T023 [US1] Modify [src/agent/daemon.rs](src/agent/daemon.rs) swarm builder to: (a) add the WSS transport as fallback priority 2 (QUIC=0, TCP=1, WSS=2); (b) wire DoH resolver via a custom `Dnsaddr` resolver; (c) emit `DialAttempt` events from the swarm loop; (d) use `RelayReservation` manager for reservations; (e) honor `--allow-ssl-inspection` flag.
- [ ] T024 [US1] Add new CLI flags to [src/cli/donor.rs](src/cli/donor.rs) per contracts/cli-worldcompute.md: `--allow-ssl-inspection`, `--wss-listen`, `--doh-only`.
- [ ] T025 [US1] Add `worldcompute admin firewall-diagnose` subcommand in [src/cli/admin.rs](src/cli/admin.rs) that runs 5-minute debug-log capture and writes an evidence bundle to `evidence/phase1/firewall-traversal/<ts>/`.
- [ ] T026 [US1] Stand up the project-operated fallback relay on tensor02 or a cooperating public machine (actual operator step, not a code task). Document in [docs/operators/running-a-relay.md](docs/operators/running-a-relay.md) per FR-007a.
- [ ] T027 [US1] Run the real-hardware test on tensor02 per T017, commit the evidence bundle under `evidence/phase1/firewall-traversal/<ts>/` including `run.log`, `metadata.json`, `results.json`, and `index.md`.

**Checkpoint**: SC-001 + SC-002 pass. Cross-firewall mesh demonstrably works.

---

## Phase 4: User Story 2 — Deep attestation with pinned root CAs (Priority: P1)

**Goal**: Real AMD/Intel/Rekor pins, no zero-bypass in production build, CI drift-check running. Per FR-008 through FR-011a.

**Independent Test**: Build with `--features production`; verify build fails if any constant is zero. Run real attestation test against a real AMD EPYC quote (from swtpm + `snpguest` on sandboxed KVM runner). Run transparency test against live `rekor.sigstore.dev`. Evidence in `evidence/phase1/attestation/<ts>/`.

### Tests for User Story 2

- [ ] T028 [P] [US2] Write [tests/verification/test_real_attestation.rs](tests/verification/test_real_attestation.rs) — loads a real AMD SEV-SNP quote from test vectors, verifies it chains to the pinned ARK fingerprint; also loads a tampered copy and asserts rejection. Use the existing swtpm-KVM CI job.
- [ ] T029 [P] [US2] Write [tests/verification/test_rekor_real.rs](tests/verification/test_rekor_real.rs) — fetches a real log entry from `https://rekor.sigstore.dev`, verifies both the Merkle inclusion proof AND the signed tree head using the pinned Ed25519 public key.
- [ ] T030 [P] [US2] Write [tests/verification/test_production_feature_gate.rs](tests/verification/test_production_feature_gate.rs) — a compile-fail test (using `trybuild` or similar) asserting the build fails under `feature = "production"` when any constant is `[0u8; 32]`.
- [ ] T031 [P] [US2] Write [tests/verification/test_drift_check.rs](tests/verification/test_drift_check.rs) — mocks an upstream mismatch and verifies `DriftCheckResult` opens an issue (test uses `gh` in dry-run mode).

### Implementation for User Story 2

- [ ] T032 [US2] Fetch real AMD ARK SHA-256 fingerprint from `https://kdsintf.amd.com/vcek/v1/Milan/cert_chain` (and Genoa), and Intel DCAP root CA from `https://api.trustedservices.intel.com/sgx/certification/v4/rootcacrl`. Replace the `[0u8; 32]` placeholders in [src/verification/attestation.rs](src/verification/attestation.rs) with the real fingerprints. Record source URL + verified-at timestamp.
- [ ] T033 [US2] Fetch real Sigstore Rekor Ed25519 public key from `https://rekor.sigstore.dev/api/v1/log/publicKey`. Replace the `[0u8; 32]` placeholder in [src/ledger/transparency.rs](src/ledger/transparency.rs). Record provenance.
- [ ] T034 [US2] Remove the "if pinned fingerprint is all-zeros, skip the check" bypass logic at `src/verification/attestation.rs:395,440` under `feature = "production"` (FR-009). In non-production (test) builds, keep a clearly-commented bypass guarded by `#[cfg(not(feature = "production"))]`.
- [ ] T035 [US2] Remove the "if pinned key is all-zeros, skip verification" bypass at `src/ledger/transparency.rs:170` under `feature = "production"` (FR-010). Same pattern as T034.
- [ ] T036 [P] [US2] Author [scripts/drift-check.sh](scripts/drift-check.sh) that refetches all three constants from upstream, diffs against the in-tree values, and on any mismatch runs `gh issue create --title "..." --label "drift-check"`.
- [ ] T037 [P] [US2] Create [.github/workflows/drift-check.yml](.github/workflows/drift-check.yml) running weekly (`cron: '0 3 * * 1'`) invoking `scripts/drift-check.sh`; run with repo-level write permission so it can open issues.
- [ ] T038 [US2] Add `worldcompute admin drift-check` CLI subcommand (wraps the script for operators) per contracts/cli-worldcompute.md.
- [ ] T039 [US2] Run the real-attestation tests on the Sandbox-KVM CI runner; commit evidence bundle to `evidence/phase1/attestation/<ts>/`.

**Checkpoint**: SC-003 passes. No production path enters attestation bypass.

---

## Phase 5: User Story 3 — Real Firecracker rootfs (Priority: P1)

**Goal**: Real bootable ext4 rootfs from CID-stored OCI layers; Firecracker boots, entrypoint runs, stdout captured via vsock. Per FR-012 through FR-014.

**Independent Test**: On a Linux KVM+Firecracker host (e.g., tensor01), build a minimal OCI image with a 200-byte static `hello` binary, push to CID store, dispatch to this node, assert stdout `"hello\n"` returned. Evidence in `evidence/phase1/firecracker-rootfs/<ts>/`.

### Tests for User Story 3

- [ ] T040 [P] [US3] Write [tests/sandbox/firecracker/test_oci_layer.rs](tests/sandbox/firecracker/test_oci_layer.rs) — validates `OciLayer` digest matching and size enforcement (data-model C.1).
- [ ] T041 [P] [US3] Write [tests/sandbox/firecracker/test_manifest.rs](tests/sandbox/firecracker/test_manifest.rs) — parses real OCI manifests from test fixtures (data-model C.2).
- [ ] T042 [P] [US3] Write [tests/sandbox/firecracker/test_rootfs_assembly.rs](tests/sandbox/firecracker/test_rootfs_assembly.rs) — uses a temp loopback device, runs `mkfs.ext4`, extracts fixture layers, verifies with `fsck.ext4`; asserts scope-guard cleanup on panic path (data-model C.3).
- [ ] T043 [P] [US3] Write real-hardware test [tests/sandbox/firecracker/test_real_boot.rs](tests/sandbox/firecracker/test_real_boot.rs) (ignored by default) — boots Firecracker with an assembled rootfs on the swtpm-KVM runner, verifies entrypoint exit code and stdout.

### Implementation for User Story 3

- [ ] T044 [P] [US3] Create [src/sandbox/firecracker/](src/sandbox/firecracker/) subdirectory. Move existing `src/sandbox/firecracker.rs` content into `src/sandbox/firecracker/mod.rs` preserving its public API.
- [ ] T045 [US3] Implement [src/sandbox/firecracker/rootfs_builder.rs](src/sandbox/firecracker/rootfs_builder.rs): `OciLayer`, `OciManifest`, `RootfsAssembly` structs per data-model C.*. Implement `build_rootfs(manifest, target_file)` with the four-stage pipeline: pull+verify layers, `mkfs.ext4` via shell-out, loopback mount via `nix::mount`, extract each tar layer with `tar` crate applying OCI whiteouts. Scope-guard drop ensures `umount` then `losetup -d` on any error path.
- [ ] T046 [US3] Replace `assemble_rootfs` in `src/sandbox/firecracker/mod.rs` with a call into `rootfs_builder::build_rootfs` (FR-013). Delete the byte-concat placeholder code.
- [ ] T047 [US3] Implement [src/sandbox/firecracker/vsock_io.rs](src/sandbox/firecracker/vsock_io.rs) — vsock-based capture of guest stdout/stderr and exit code. Wire into the Firecracker launch path.
- [ ] T048 [US3] Modify Firecracker boot args in the driver to use `init=/sbin/init console=ttyS0 reboot=k panic=1 pci=off` (production-appropriate) per research.md §4.
- [ ] T049 [US3] Run the real-boot test on tensor01 or the swtpm-KVM runner; commit evidence bundle.

**Checkpoint**: SC-004 passes. Firecracker executes real OCI workloads.

---

## Phase 6: User Story 4 — End-to-end Phase 1 cluster + real churn (Priority: P1)

**Goal**: Three real machines form a mesh, accept jobs, survive kills, and pass a real 72-hour churn run at 30 % rotation with ≥ 80 % completion. Per FR-015 through FR-017.

**Independent Test**: `scripts/e2e-phase1.sh` builds the binary, deploys to tensor01 + tensor02 + laptop, submits 100 workloads, kills nodes on schedule, and asserts ≥ 80 % completion. The 72-hour variant runs via `scripts/churn-harness.sh` separately and commits evidence.

### Tests for User Story 4

- [ ] T050 [P] [US4] Write [tests/integration/test_e2e_three_node.rs](tests/integration/test_e2e_three_node.rs) — in-process three-daemon variant (fast CI version) using localhost libp2p + forced-kill.
- [ ] T051 [P] [US4] Write [tests/integration/test_churn_harness_smoke.rs](tests/integration/test_churn_harness_smoke.rs) — 1-hour smoke that exercises every kill/rejoin code path.

### Implementation for User Story 4

- [ ] T052 [P] [US4] Author [scripts/e2e-phase1.sh](scripts/e2e-phase1.sh) per plan.md: takes a host-list file, rsyncs the binary, starts daemons via SSH, submits workloads, emits evidence bundle.
- [ ] T053 [P] [US4] Author [scripts/churn-harness.sh](scripts/churn-harness.sh) per research.md §12: spawns N local daemons + SSH-remote daemons on tensor01/tensor02, kills/restarts on a Poisson schedule at 30 %/hr rotation; emits hourly ledger dumps; validates 72 h @ 30 % ≥ 80 % completion.
- [ ] T054 [US4] Refactor `src/churn/simulator.rs` (the statistical model) to expose the same API but internally invoke the real harness for a `real` variant. Keep the statistical variant available for quick CI runs behind a feature flag.
- [ ] T055 [US4] Run `scripts/e2e-phase1.sh` on tensor01 + tensor02 + laptop; commit evidence bundle.
- [ ] T056 [US4] Run `scripts/churn-harness.sh` for 72 hours; commit evidence bundle (this is the canonical FR-016 evidence producer).

**Checkpoint**: SC-005 passes. Real multi-machine mesh proven stable under churn.

---

## Phase 7: User Story 5 — Real platform adapters (Priority: P2)

**Goal**: Live Slurm, K8s, and cloud adapter enrollment tests. Per FR-018 through FR-021.

**Independent Test**: Containerized Slurm and Kind-on-CI tests run automatically; cloud adapter runs via `workflow_dispatch` gated to maintainer+. Apple VF helper binary built in macOS CI.

### Tests for User Story 5

- [ ] T057 [P] [US5] Write [tests/adapters/test_slurm_live.rs](tests/adapters/test_slurm_live.rs) — submits a real `sbatch` via a containerized slurmrestd (docker-compose up in the test), polls for completion, returns result.
- [ ] T058 [P] [US5] Write [tests/adapters/test_k8s_live.rs](tests/adapters/test_k8s_live.rs) — installs the ClusterDonation CRD in a Kind cluster (spawned in-test via `kind create cluster`), applies a resource, asserts the operator reconciles.
- [ ] T059 [P] [US5] Write [tests/adapters/test_cloud_live.rs](tests/adapters/test_cloud_live.rs) — `#[ignore]` by default; when run, fetches real IMDS identity doc per provider and asserts signature validity. Runs in `workflow_dispatch` only.

### Implementation for User Story 5

- [ ] T060 [P] [US5] Update [adapters/slurm/](adapters/slurm/) to wire the existing HTTP client to real slurmrestd; add docker-compose-based containerized Slurm fixture for CI.
- [ ] T061 [P] [US5] Update [adapters/kubernetes/](adapters/kubernetes/) to install the CRD + operator on a Kind cluster spawned in CI.
- [ ] T062 [P] [US5] Update [adapters/cloud/](adapters/cloud/) to add real IMDS fetchers for AWS/GCE/Azure with upstream signature verification.
- [ ] T063 [P] [US5] Create [.github/workflows/cloud-live-tests.yml](.github/workflows/cloud-live-tests.yml) with `workflow_dispatch` trigger; gated by `permissions: contents: read, actions: read, issues: write` and a maintainer-check step (`github.event.sender.login` in org maintainers). Spins up AWS `t3.micro` / GCE `e2-micro` / Azure B1s using free-tier credits; runs the `#[ignore]`d test; tears down.
- [ ] T064 [US5] Build [adapters/apple_vf_helper/](adapters/apple_vf_helper/) Swift binary in macOS CI; sign with project developer ID; attach to release artifacts.
- [ ] T065 [US5] Run the cloud live tests manually once via `workflow_dispatch`; commit evidence per provider under `evidence/phase1/cloud-adapter/<provider>/<ts>/`.

**Checkpoint**: FR-018, FR-019, FR-020, FR-020a, FR-021 pass.

---

## Phase 8: User Story 6 — Distributed-diffusion mesh LLM (Priority: P2)

**Goal**: Replace the AR-ensemble mesh LLM with distributed diffusion per whitepaper. Per FR-022 through FR-029, FR-028a.

**Independent Test**: Smoke test on 6 GPUs (3 tensor01 + 3 tensor02) with LLaDA-8B backbone + 2+ SSD-2-style experts; constraint-satisfaction / planning / code-infilling prompt returns coherent answer. ParaDiGMS ≥ 2× speedup and DistriFusion ≥ 50 % RTT-masking measured via `tc netem` 100 ms RTT. Evidence in `evidence/phase1/diffusion-mesh/<ts>/`.

### Tests for User Story 6

- [ ] T066 [P] [US6] Write [tests/diffusion/test_backbone.rs](tests/diffusion/test_backbone.rs) — loads a tiny stub masked-diffusion model (fixture, not a real 7B download) and verifies `produce_score` returns shape-correct tensors.
- [ ] T067 [P] [US6] Write [tests/diffusion/test_pcg.rs](tests/diffusion/test_pcg.rs) — verifies PCG composition with known synthetic score fields matches closed-form expected output; exercises the clipping-bound logic.
- [ ] T068 [P] [US6] Write [tests/diffusion/test_paradigms.rs](tests/diffusion/test_paradigms.rs) — verifies Picard iteration converges on a test fixed-point problem within `max_iterations`; verifies sequential fallback on forced non-convergence.
- [ ] T069 [P] [US6] Write [tests/diffusion/test_distrifusion.rs](tests/diffusion/test_distrifusion.rs) — two in-process workers exchange activation tensors; asserts staleness bound is honored.
- [ ] T070 [P] [US6] Write real-hardware test [tests/diffusion/test_e2e_diffusion.rs](tests/diffusion/test_e2e_diffusion.rs) (ignored by default) — runs the 6-GPU smoke test with real LLaDA-8B backbone + experts; asserts ≥ 2× ParaDiGMS speedup and ≥ 50 % RTT masking.

### Implementation for User Story 6

- [ ] T071 [US6] **Remove** the existing AR-ensemble module: delete [src/agent/mesh_llm/](src/agent/mesh_llm/) entirely (router.rs, aggregator.rs, expert.rs, safety.rs, self_prompt.rs, service.rs, subset.rs, mod.rs). Remove its `proto/mesh_llm.proto`. Remove its tests under `tests/mesh_llm/`.
- [ ] T072 [P] [US6] Create [proto/mesh_llm_diffusion.proto](proto/mesh_llm_diffusion.proto) per contracts/grpc-mesh-llm-diffusion.md; wire it into the tonic build in `build.rs`.
- [ ] T073 [P] [US6] Implement [src/agent/mesh_llm_diffusion/backbone.rs](src/agent/mesh_llm_diffusion/backbone.rs) — `DiffusionBackbone` loading LLaDA-8B via candle (or `tch` fallback); exposes `produce_score(x_t, t, mask) → Tensor`.
- [ ] T074 [P] [US6] Implement [src/agent/mesh_llm_diffusion/expert.rs](src/agent/mesh_llm_diffusion/expert.rs) — `DiffusionExpert` for small specialized experts with backbone-compatibility check (FR-024).
- [ ] T075 [US6] Implement [src/agent/mesh_llm_diffusion/pcg.rs](src/agent/mesh_llm_diffusion/pcg.rs) — PCG score composition per research.md §9 and data-model E.4 (FR-023, FR-024); exposes `compose_scores(backbone_score, expert_scores, weights, tau) → Tensor` with audit record emission.
- [ ] T076 [US6] Implement [src/agent/mesh_llm_diffusion/paradigms.rs](src/agent/mesh_llm_diffusion/paradigms.rs) — Picard-iteration parallel denoising per research.md §10 (FR-025); `ParaDiGMSBlock` per data-model E.5; sequential fallback on non-convergence.
- [ ] T077 [US6] Implement [src/agent/mesh_llm_diffusion/distrifusion.rs](src/agent/mesh_llm_diffusion/distrifusion.rs) — stale-activation pipelining over libp2p request-response protocol `/worldcompute/diffusion-activation/1.0.0` (FR-026); CBOR + zstd encoding of fp16 tensors.
- [ ] T078 [US6] Implement [src/agent/mesh_llm_diffusion/scheduler.rs](src/agent/mesh_llm_diffusion/scheduler.rs) — denoising-step scheduler (not token-step); manages ParaDiGMS blocks; invokes DistriFusion transport.
- [ ] T079 [US6] Implement [src/agent/mesh_llm_diffusion/safety.rs](src/agent/mesh_llm_diffusion/safety.rs) — denoising-step-granular kill switch (FR-029); polled before each step via `PollKillSwitch` RPC.
- [ ] T080 [US6] Implement [src/agent/mesh_llm_diffusion/service.rs](src/agent/mesh_llm_diffusion/service.rs) — tonic-generated service handler; streaming `Infer` RPC emitting `DenoisingStepTelemetry`, `ParaDiGMSBlockReport`, `DistriFusionPipelineReport`, and terminal `InferComplete` / `InferHalted` / `InferError` per contracts/grpc-mesh-llm-diffusion.md (FR-023, FR-027).
- [ ] T081 [US6] Wire the diffusion service into the daemon's gRPC server in [src/agent/daemon.rs](src/agent/daemon.rs). Add new CLI flag `--diffusion-gpu-role backbone|expert|none` to register a node's role on startup.
- [ ] T082 [US6] Extend [src/cli/submitter.rs](src/cli/submitter.rs) with the `--diffusion`, `--backbone`, `--experts`, `--denoising-steps`, `--paradigms-block-size`, `--staleness`, `--clipping-tau` flags per contracts/cli-worldcompute.md.
- [ ] T083 [US6] Update [src/lib.rs](src/lib.rs) to re-export `mesh_llm_diffusion` in place of the removed `mesh_llm`. Update [Cargo.toml](Cargo.toml) if any crate-level features referenced `mesh_llm`.
- [ ] T084 [US6] Download + mirror LLaDA-8B weights into the CID store (operator step); record the `weights_cid` in documentation.
- [ ] T085 [US6] Author [scripts/diffusion-smoke.sh](scripts/diffusion-smoke.sh) — stands up 6-GPU cross-machine smoke test with `tc qdisc netem` 100 ms RTT on tensor01↔tensor02; runs a constraint-satisfaction prompt; records wall-clock speedups and RTT-masking percentages; emits evidence bundle.
- [ ] T086 [US6] Run the 6-GPU diffusion smoke test; commit evidence bundle.

**Checkpoint**: SC-010 passes. Distributed diffusion demonstrably works.

---

## Phase 9: User Story 7 — Eliminate all remaining placeholders (Priority: P2)

**Goal**: Zero placeholders in production `src/`; `.placeholder-allowlist` empty. Per FR-030 through FR-038.

**Independent Test**: `scripts/verify-no-placeholders.sh --check-empty` exits 0. `grep -rn 'placeholder\|stub\|TODO\|todo!\|unimplemented!' src/` returns no matches.

### Tests for User Story 7

- [ ] T087 [P] [US7] Write [tests/integration/test_placeholder_cleanup.rs](tests/integration/test_placeholder_cleanup.rs) — an integration test that invokes `scripts/verify-no-placeholders.sh --check-empty` and asserts exit 0.
- [ ] T088 [P] [US7] Write [tests/agent/test_lifecycle_gossip.rs](tests/agent/test_lifecycle_gossip.rs) — heartbeat / pause / withdraw actually publish gossip messages (FR-030).
- [ ] T089 [P] [US7] Write [tests/governance/test_ban_real.rs](tests/governance/test_ban_real.rs) — `ban()` updates the trust registry + broadcasts a governance action (FR-031).
- [ ] T090 [P] [US7] Write [tests/verification/test_receipt_real.rs](tests/verification/test_receipt_real.rs) — real coordinator pub key wired; valid receipts pass, malformed/unsigned reject (FR-032).
- [ ] T091 [P] [US7] Write [tests/agent/test_current_load.rs](tests/agent/test_current_load.rs) — `current_load()` returns non-constant values across CPU/GPU/memory stress scenarios (FR-033).
- [ ] T092 [P] [US7] Write [tests/data_plane/test_confidential_seal.rs](tests/data_plane/test_confidential_seal.rs) — TPM2-backed seal/unseal on the swtpm-KVM runner OR verify graceful fallback with trust-tier downgrade (FR-034).
- [ ] T093 [P] [US7] Write [tests/sandbox/test_apple_vf_platform.rs](tests/sandbox/test_apple_vf_platform.rs) — on non-macOS returns `Error::UnsupportedPlatform`; on macOS CI produces a real disk (FR-035).
- [ ] T094 [P] [US7] Write [tests/governance/test_service_persists.rs](tests/governance/test_service_persists.rs) — `SubmitProposal` and `CastVote` persist to the governance store and emit audit events (FR-036).
- [ ] T095 [P] [US7] Write [tests/policy/test_signed_builder.rs](tests/policy/test_signed_builder.rs) — the one-pass signed-builder produces valid signed manifests without ever exposing a `vec![0u8; 64]` intermediate state (FR-037).

### Implementation for User Story 7

- [ ] T096 [US7] Fix [src/agent/lifecycle.rs](src/agent/lifecycle.rs) per FR-030: either delete the standalone functions and migrate callers to the daemon's gossipsub broadcast, OR wire the standalone functions to publish directly. Remove the "serializes to JSON, and returns the payload plus a placeholder response" comment.
- [ ] T097 [US7] Fix [src/governance/admin_service.rs](src/governance/admin_service.rs) `ban()` per FR-031: update the trust registry (in-memory + persistent store) and broadcast a `GovernanceAction::Ban` message.
- [ ] T098 [US7] Fix [src/verification/receipt.rs](src/verification/receipt.rs) per FR-032: wire the coordinator public key (pass it through from the Raft coordinator leader election), cryptographically verify `receipt.signature` against the message + key, reject on mismatch.
- [ ] T099 [US7] Fix [src/agent/daemon.rs](src/agent/daemon.rs) `current_load()` per FR-033: replace the `0.1` constant with a `LoadSample` built from `sysinfo` (CPU + memory) + `nvml-wrapper` (GPU). Cache 500 ms. Return `max(cpu, gpu, mem)`.
- [ ] T100 [US7] Decide on TPM2 path for [src/data_plane/confidential.rs](src/data_plane/confidential.rs) per FR-034 + research.md §6: either wire `tss-esapi` PCR-bound seal/unseal, or remove the function entirely if attested-key-release subsumes it. Document the decision inline.
- [ ] T101 [US7] Fix [src/sandbox/apple_vf.rs](src/sandbox/apple_vf.rs) per FR-035: on macOS call the Swift helper binary to produce a real VZDiskImage; on non-macOS return `Error::UnsupportedPlatform`. Remove the `b"placeholder-disk"` writes at lines 176 and 239.
- [ ] T102 [US7] Fix [src/governance/governance_service.rs](src/governance/governance_service.rs) per FR-036: `SubmitProposal` persists to the governance `Proposals` store (filesystem or CRDT-backed); `CastVote` persists a vote record + emits an audit event.
- [ ] T103 [US7] Refactor [src/policy/rules.rs](src/policy/rules.rs) and [src/policy/engine.rs](src/policy/engine.rs) per FR-037: replace the build-then-resign two-step with a single-pass signed-builder that never exposes a `vec![0u8; 64]` intermediate. Delete the "placeholder — signed below" comments.
- [ ] T104 [US7] Remove any remaining bypass comments / dead-code paths in [src/verification/attestation.rs](src/verification/attestation.rs) that were made unreachable by T034.
- [ ] T105 [US7] Audit every `stub` / `placeholder` / `TODO` / `todo!` / `unimplemented!` token in `src/` via `scripts/verify-no-placeholders.sh --list`; remove or fix each remaining occurrence until the list is empty.
- [ ] T106 [US7] Assert that `.placeholder-allowlist` is empty. Run `scripts/verify-no-placeholders.sh --check-empty` and confirm exit code 0.

**Checkpoint**: SC-006 passes. Every placeholder gone.

---

## Phase 10: User Story 8 — Operations, deployment, release pipeline (Priority: P3)

**Goal**: Tauri GUI buildable + smoke-tested; Dockerfile + Helm chart CI-verified; REST gateway bound; reproducible signed releases. Per FR-039 through FR-044.

**Independent Test**: Fresh-VM quickstart (`scripts/quickstart-timed.sh`) finishes in ≤ 15 min; release pipeline produces bit-identical signed artifacts from two runners; `curl http://localhost:8443/v1/health` returns OK.

### Tests for User Story 8

- [ ] T107 [P] [US8] Write Playwright harness [gui/tests/smoke.spec.ts](gui/tests/smoke.spec.ts) covering enroll, submit, monitor flows (FR-039).
- [ ] T108 [P] [US8] Write [.github/workflows/quickstart-timed.yml](.github/workflows/quickstart-timed.yml) running `scripts/quickstart-timed.sh` on fresh Ubuntu 24.04, macOS 14, Windows 11 runners per release (FR-042).
- [ ] T109 [P] [US8] Write [tests/integration/test_rest_gateway.rs](tests/integration/test_rest_gateway.rs) exercising each endpoint in contracts/rest-gateway.md (FR-041).
- [ ] T110 [P] [US8] Write [tests/integration/test_reproducible_build.rs](tests/integration/test_reproducible_build.rs) — placeholder assertion; real reproducible-build check is the CI workflow in T113.

### Implementation for User Story 8

- [ ] T111 [US8] Build the Tauri GUI on macOS/Linux/Windows CI per FR-039: fix `gui/src-tauri` compilation; wire enroll/submit/monitor flows; run Playwright smoke tests.
- [ ] T112 [US8] Verify [ops/Dockerfile](ops/Dockerfile) builds in CI per FR-040; add multi-stage build and set `CMD ["worldcompute"]`.
- [ ] T113 [P] [US8] Create [.github/workflows/reproducible-build.yml](.github/workflows/reproducible-build.yml) per research.md §13: Nix-based hermetic build on two independent runners; diff outputs with `diffoscope`; fail on any difference (FR-043).
- [ ] T114 [P] [US8] Create [ops/release/build-reproducible.sh](ops/release/build-reproducible.sh), [ops/release/sign-release.sh](ops/release/sign-release.sh), [ops/release/verify-release.sh](ops/release/verify-release.sh) implementing the three-script pipeline. Ship `RELEASE_PUBLIC_KEY` constant in the verify script (FR-044).
- [ ] T115 [US8] Deploy the Helm chart in [ops/helm/](ops/helm/) to a Kind cluster in CI; run a smoke test (FR-040).
- [ ] T116 [US8] Bind the REST gateway HTTP listener in [src/agent/daemon.rs](src/agent/daemon.rs) per FR-041 + contracts/rest-gateway.md. Implement each endpoint. Wire rate-limiting + mTLS from spec-004.
- [ ] T117 [US8] Add `worldcompute admin verify-release` subcommand (wraps `verify-release.sh`) per contracts/cli-worldcompute.md.
- [ ] T118 [P] [US8] Author [scripts/quickstart-timed.sh](scripts/quickstart-timed.sh) that runs the quickstart.md steps in a fresh VM and measures wall-clock time (FR-042).
- [ ] T119 [US8] Write or update [README.md](README.md) quickstart pointer to [specs/005-production-readiness/quickstart.md](specs/005-production-readiness/quickstart.md).
- [ ] T120 [US8] Run `scripts/quickstart-timed.sh` on fresh Ubuntu/macOS/Windows VMs; commit evidence bundle under `evidence/phase1/quickstart/<platform>/<ts>/`.
- [ ] T121 [US8] Cut a dry-run release tag; verify reproducible build passes; verify signatures; commit evidence.

**Checkpoint**: SC-007 + SC-008 + SC-009 pass.

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Finalize docs, ensure test count grows, clean up, and run the full evidence-artifact suite.

- [ ] T122 [P] Update [specs/001-world-compute-core/whitepaper.md](specs/001-world-compute-core/whitepaper.md) with v0.5 entry describing spec 005 outcomes.
- [ ] T123 [P] Ensure 900+ tests pass (`cargo test` reports ≥ 900; count was 802 at start of spec 005).
- [ ] T124 [P] Run `cargo clippy --lib --tests -- -D warnings` and fix any new warnings.
- [ ] T125 [P] Run `cargo fmt --check` and fix any formatting drift.
- [ ] T126 [P] Run the full evidence-artifact suite and confirm every SC has at least one `overall: pass` bundle: SC-001 (firewall-traversal), SC-003 (attestation), SC-004 (firecracker-rootfs), SC-005 (churn), SC-008 (quickstart), SC-010 (diffusion-mesh).
- [ ] T127 Write session notes to [notes/session-2026-04-NN-spec-005-implementation.md](notes/session-2026-04-NN-spec-005-implementation.md) per CLAUDE.md global instructions.
- [ ] T128 Close issues #28, #29, #30, #33, #34, #37, #38, #39, #40, #41, #43, #51, #52, #53, #56, #27, #54, #60 on GitHub with comments pointing at the completed PR and spec 005 evidence bundles.
- [ ] T129 Update [CLAUDE.md](CLAUDE.md) "Remaining Stubs and Placeholders" section to reflect the now-empty state (replace with "None — enforced by `scripts/verify-no-placeholders.sh --check-empty` on every PR").
- [ ] T130 Final check: run `scripts/verify-no-placeholders.sh --check-empty` from a clean checkout. Exit 0 is the spec-005 completion gate.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately.
- **Phase 2 (Foundational)**: Depends on Phase 1 complete — BLOCKS all user stories.
- **Phase 3 (US1)**, **Phase 4 (US2)**, **Phase 5 (US3)**, **Phase 6 (US4)**: All depend on Phase 2 only; mutually independent, can run in parallel.
- **Phase 7 (US5)** can start after Phase 2; does not depend on US1–US4.
- **Phase 8 (US6)** depends on Phase 2 but NOT on US1–US5; can run in parallel with all of them. Requires real GPU hardware (tensor01 + tensor02) for T070, T086.
- **Phase 9 (US7)** can start after Phase 2 but its real-hardware pieces (T092 TPM2 seal, T093 Apple VF) require the swtpm-KVM and macOS CI runners from Phase 2.
- **Phase 10 (US8)** depends on Phase 2 and also on T116 REST gateway landing before the timed-quickstart runs.
- **Phase 11 (Polish)**: Depends on all user stories being complete.

### User Story Dependencies

- **US1 ↔ US2 ↔ US3 ↔ US4**: Independent.
- **US5**: Independent.
- **US6**: Independent; consumes mesh-formation (US1) for cross-machine test but the unit tests do not require it.
- **US7**: Independent of other user stories but finishes LAST because it verifies that no other story introduced a new placeholder.
- **US8**: Requires the REST gateway route handlers from `src/rest/*` (which land as part of T116) + the reproducible-build workflow (T113).

### Within Each User Story

- Tests are written BEFORE or IN PARALLEL with implementation (per constitution Principle V: direct-hardware tests). For real-hardware tests that require live machines (T017, T039, T049, T055, T056, T065, T086, T120), the test-framework code lands first; the actual run happens once the implementation + environment are ready.

### Parallel Opportunities

- Phase 1: T002–T007 all parallel.
- Phase 2: T009–T012 mostly parallel (T010 and T011 parallel with T009; T012 depends on nothing).
- **Phase 3 ↔ 4 ↔ 5 ↔ 6** can be implemented concurrently by up to four contributors.
- Within US1: T013–T016 tests parallel; T018–T020 implementation parallel.
- Within US2: T028–T031 tests parallel; T036–T037 CI drift workflow parallel with T032–T033 pins.
- Within US3: T040–T043 tests parallel; T044 + T045 sequential (mod.rs before submodules).
- Within US4: T052 + T053 scripts parallel.
- Within US5: T057–T059 tests parallel; T060–T063 adapter mutations parallel.
- Within US6: T066–T069 tests parallel; T073–T074 parallel (backbone + expert in different files); but T075 PCG depends on data-model stabilization (T010). T077 DistriFusion is parallel-safe with T076 ParaDiGMS.
- Within US7: T087–T095 tests all parallel; T096–T103 implementations mostly parallel (different files).
- Within US8: T107–T110 tests parallel; T113–T118 implementations mostly parallel.

---

## Parallel Example: User Story 1 (cross-firewall mesh)

```bash
# Launch all US1 tests in parallel:
Task: "T013 Write integration test tests/network/test_wss_transport.rs"
Task: "T014 Write integration test tests/network/test_doh_resolver.rs"
Task: "T015 Write integration test tests/network/test_relay_reservation.rs"
Task: "T016 Write integration test tests/network/test_dial_logging.rs"

# Launch all US1 parallel-safe implementations:
Task: "T018 Implement src/network/wss_transport.rs"
Task: "T019 Implement src/network/doh_resolver.rs"
Task: "T020 Implement src/network/dial_logging.rs"
# T021 (relay_reservation) sequential after T010 (types)
# T022 (discovery.rs) sequential after T018+T019
# T023 (daemon.rs) sequential after T018–T022 all land
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (Setup) and Phase 2 (Foundational).
2. Complete Phase 3 (US1) — cross-firewall mesh formation.
3. **STOP and VALIDATE**: Run the tensor02 test (T017 + T027). If a donor behind Dartmouth's firewall successfully forms the mesh and round-trips a job, the most important gap is closed.
4. This is the demoable state — cut an `0.5.0-mvp` tag, ship for early volunteer feedback.

### Incremental Delivery

1. MVP (US1) → SC-001 + SC-002.
2. Add US2 (attestation) → SC-003 + real safety guarantees.
3. Add US3 (Firecracker) → SC-004 + Linux sandbox usable.
4. Add US4 (Phase-1 cluster + churn) → SC-005 + real multi-machine stability.
5. Add US7 placeholder sweep → SC-006 + zero-placeholder guarantee.
6. Add US5 (adapters) → expand donor base.
7. Add US6 (diffusion mesh-LLM) → SC-010 + the project's headline feature.
8. Add US8 (operations) → SC-007 + SC-008 + SC-009 + public adoption.

### Parallel Team Strategy

With N contributors (e.g., 4):

1. All contributors together: Phase 1 + Phase 2.
2. After Phase 2 closes:
   - Contributor A: US1 (network / transport specialist)
   - Contributor B: US2 + US3 (verification + sandbox specialist)
   - Contributor C: US4 + US5 (cluster + adapters)
   - Contributor D: US6 (ML specialist; diffusion swarm) — see risk flag below
3. US7 (placeholder sweep) done LAST by any contributor — depends on all other stories landing so it catches anything accidentally introduced.
4. US8 (operations) runs alongside US4–US6 once REST gateway core (T116) is drafted.

### Risk Flag: US6 (Distributed Diffusion Mesh-LLM) — HIGH

**Why high-risk**: US6 contains 21 tasks — the single largest story in this spec. It also introduces the most novel engineering (LLaDA-8B inference via `candle` has no prior art in this codebase; there is no published end-to-end system that combines distributed diffusion LMs with Petals-style libp2p hosting; PCG score composition at this scale has no open-source reference implementation). Research.md §7–§11 resolve the *architectural* questions, but the *performance* questions (Is 2× ParaDiGMS speedup achievable on consumer GPUs? Is 50% RTT-masking achievable with 100ms netem on real tensor01↔tensor02 link? Does a 7B masked-diffusion backbone fit in a single H100's 80GB with ≥ 2 experts co-resident?) only resolve at test time.

**Mitigations**:
1. **Split US6 across 2–3 contributors** if team size permits:
    - Contributor D1: T066–T070 tests + T071 removal + T072 proto
    - Contributor D2: T073 backbone + T075 PCG + T076 ParaDiGMS (the ML core)
    - Contributor D3: T077 DistriFusion + T078 scheduler + T079 safety + T080 service + T081 daemon wiring (the distributed-systems layer)
2. **Land the smoke infrastructure first**: T085 `diffusion-smoke.sh` should be authored early so benchmarks can be iterated against while T073–T080 land, not just once at the end.
3. **Timebox benchmark validation**: if T086 fails to achieve ≥ 2× speedup or ≥ 50% RTT masking on first real-hardware run, treat it as a research finding (not a blocking defect) and file a follow-up issue; the architectural goal — *any* working end-to-end distributed-diffusion inference — is still the minimum bar for SC-010. Document observed speedup / masking in the evidence bundle regardless.
4. **Fallback model**: if LLaDA-8B weights or its tokenizer prove incompatible with candle inside the project timeline, fall back to Dream 7B or DiffuLLaMA per research.md §7; the PCG / ParaDiGMS / DistriFusion primitives (T075–T077) are backbone-agnostic.

Risk-tracked as follow-up issue at implementation time; do not treat benchmark non-achievement alone as a blocker for the rest of the spec.

---

## Notes

- **[P] tasks** = different files, no blocking dependency on an incomplete task.
- **[Story] label** ties each task to a user story for traceability and MVP selection.
- **Real-hardware tests** (`#[ignore]`-gated ones at T017, T043, T070, T092, T120) are run by operators on the designated machines and committed as evidence artifacts; they are NOT in the default `cargo test` run.
- **Commit frequently**: per CLAUDE.md global instructions, commit after each task or logical group, back up work to GitHub, and keep notes current.
- **Direct-test evidence**: every SC (SC-001 through SC-010) must end the spec with at least one `overall: pass` evidence bundle under `evidence/phase1/<area>/`. Phase 11 task T126 verifies this.
- **Completion gate**: `.placeholder-allowlist` MUST be empty at T130. This is the single bit that determines whether spec 005 passes.
- **Avoid**: introducing new placeholders while fixing old ones; breaking existing 802 tests; adding cross-story dependencies that break independence.
