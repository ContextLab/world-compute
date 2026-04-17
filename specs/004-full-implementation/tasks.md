# Tasks: Full Functional Implementation

**Input**: Design documents from `/specs/004-full-implementation/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Exact file paths included in all descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add new dependencies needed across multiple user stories

- [x] T001 Add `rsa = "0.9"`, `p256 = "0.13"`, `p384 = "0.13"`, and `nix = { version = "0.29", features = ["signal", "process"] }` dependencies to Cargo.toml for certificate chain verification and SIGSTOP delivery
- [x] T002 [P] Add `aes-gcm = "0.10"` and `x25519-dalek = "2"` dependencies to Cargo.toml for confidential compute
- [x] T003 [P] Add `rcgen = "0.12"` and `tokio-rustls = "0.26"` dependencies to Cargo.toml for mTLS
- [x] T004 [P] Add `threshold-crypto = "0.2"` dependency to Cargo.toml for threshold signing (verify not already present)
- [x] T005 [P] Add `kube = "0.88"` and `k8s-openapi = "0.21"` dependencies to adapters/kubernetes/Cargo.toml
- [x] T006 [P] Add `candle-core`, `candle-transformers`, and `tokenizers` dependencies to Cargo.toml for mesh LLM (check crates.io for latest versions before adding — candle may be 0.6.x or 0.7.x)
- [x] T007 Verify build succeeds with all new dependencies: `cargo build --lib`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared types and structures needed by multiple user stories

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T008 Define `InclusionProof` struct (leaf_hash, tree_size, proof_hashes, signed_tree_head) in src/ledger/transparency.rs per data-model.md
- [x] T009 [P] Define `ConfidentialBundle` struct (ciphertext_cid, cipher, nonce, wrapped_key, confidentiality_level, attestation_requirement) in src/data_plane/confidential.rs per data-model.md
- [x] T010 [P] Define `Lease` struct (lease_id, task_id, node_id, issued_at, ttl_ms, renewed_at, status) with state transitions in src/scheduler/broker.rs per data-model.md
- [x] T011 [P] Define `CreditDecayEvent` struct (account_id, balance_before, balance_after, decay_rate, floor, timestamp) in src/credits/decay.rs per data-model.md
- [x] T012 [P] Define `MeshExpert` struct (expert_id, model_name, tokenizer, vram_mb, max_batch_size, health, last_heartbeat, latency_p50_ms) in src/agent/mesh_llm/expert.rs per data-model.md
- [x] T013 [P] Define `ActionTier` enum (ReadOnly, Suggest, SandboxTest, DeployMinor, DeployMajor) with approval requirements in src/agent/mesh_llm/safety.rs per data-model.md
- [x] T014 [P] Define `EgressAllowlist` struct (approved_endpoints, default_action=Deny) in src/policy/rules.rs per data-model.md
- [x] T015 [P] Define `StorageCap` struct (node_id, cap_bytes, used_bytes, last_gc_at) in src/data_plane/cid_store.rs per data-model.md
- [x] T016 [P] Add `allowed_endpoints: Vec<String>` and `confidentiality_level: Option<ConfidentialityLevel>` fields to JobManifest in src/scheduler/manifest.rs per data-model.md
- [x] T017 [P] Add `artifact_registry_result` and `egress_validation_result` fields to PolicyDecision in src/policy/engine.rs per data-model.md
- [x] T018 Run `cargo test` and `cargo clippy --lib -- -D warnings` to verify zero regressions

**Checkpoint**: Foundation ready — user story implementation can now begin in parallel

---

## Phase 3: US1 — Cryptographically Verified Attestation (Priority: P1) #28, #29

**Goal**: Full cryptographic chain verification for TPM2/SEV-SNP/TDX and Rekor Merkle inclusion proof verification.

**Independent Test**: Present known-good/bad cert chains → verify 100% correct accept/reject. Submit Rekor entry → verify inclusion proof.

### Certificate Chain Verification (#28)

- [ ] T019 [P] [US1] Implement RSA signature verification using `rsa` crate in src/verification/attestation.rs `validate_chain_structure()`: extract RSA public key from parent cert, verify child cert signature
- [ ] T020 [P] [US1] Implement ECDSA-P256/P384 signature verification using `p256`/`p384` crates in src/verification/attestation.rs: extract EC public key, verify signature
- [ ] T021 [US1] Wire RSA/ECDSA verification into `Tpm2ChainValidator::validate_chain()` in src/verification/attestation.rs: verify EK cert signature chain, check manufacturer OID (2.23.133.x) in leaf cert extensions
- [ ] T022 [P] [US1] Wire ECDSA-P384 verification into `SevSnpChainValidator::validate_chain()` in src/verification/attestation.rs: verify ARK→ASK→VCEK chain, compare root fingerprint against pinned AMD ARK SHA-256
- [ ] T023 [P] [US1] Wire ECDSA-P256 verification into `TdxChainValidator::validate_chain()` in src/verification/attestation.rs: verify Intel DCAP root→PCK chain, compare root fingerprint against pinned Intel CA SHA-256
- [ ] T024 [US1] Implement certificate expiry checking in all three validators: reject chains containing expired certificates
- [ ] T025 [US1] Replace TODO at src/verification/attestation.rs line ~627 with real Ed25519/ECDSA verification against platform root-of-trust
- [ ] T026 [US1] Add integration test: valid AMD SEV-SNP test vector → accepted; tampered chain → rejected in tests/verification/test_deep_attestation.rs
- [ ] T027 [P] [US1] Add integration test: valid Intel TDX test vector → accepted; wrong root → rejected in tests/verification/test_deep_attestation.rs
- [ ] T028 [P] [US1] Add integration test: valid TPM2 EK chain → accepted; expired cert → rejected in tests/verification/test_deep_attestation.rs

### Merkle Inclusion Proof (#29)

- [ ] T029 [P] [US1] Implement RFC 6962 Merkle inclusion proof verification in src/ledger/transparency.rs `verify_anchor()`: compute root from leaf_hash + proof_hashes, compare to signed_tree_head.root_hash
- [ ] T030 [US1] Pin Rekor public key as compile-time constant in src/ledger/transparency.rs (fetch from Rekor API `/api/v1/log/publicKey`)
- [ ] T031 [US1] Verify signed tree head signature with pinned Rekor public key in src/ledger/transparency.rs
- [ ] T032 [US1] Add integration test: submit entry to Rekor staging → retrieve inclusion proof → verify against signed tree head in tests/test_rekor_transparency.rs
- [ ] T033 [US1] Add integration test: tampered proof data → verification fails in tests/test_rekor_transparency.rs
- [ ] T034 [US1] Remove all `// TODO` comments from src/verification/attestation.rs and src/ledger/transparency.rs
- [ ] T035 [US1] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-001 partial (attestation TODOs resolved). FR-001, FR-002 satisfied.

---

## Phase 4: US2 — Agent Lifecycle and Preemption (Priority: P1) #30, #45

**Goal**: Real heartbeat, pause/checkpoint, withdrawal, and sub-10ms preemption.

**Independent Test**: Enroll agent → heartbeat → pause → resume → withdraw → scan for zero residue. Inject keyboard event → measure SIGSTOP < 10ms.

### Agent Lifecycle (#30)

- [ ] T036 [US2] Implement `heartbeat()` in src/agent/lifecycle.rs: send periodic state update (node capabilities, active leases, resource usage) to broker via gossipsub, receive lease offers in response
- [ ] T037 [US2] Implement `pause()` in src/agent/lifecycle.rs: SIGSTOP all sandbox processes, attempt checkpoint for each active sandbox, transition AgentState to Paused, stop accepting new leases
- [ ] T038 [US2] Implement `withdraw()` in src/agent/lifecycle.rs: checkpoint all active sandboxes, terminate them, wipe scoped working directory (`rm -rf work_dir`), revoke Ed25519 keypair, notify broker of withdrawal, verify zero host residue
- [ ] T039 [US2] Wire heartbeat loop in src/agent/mod.rs: spawn tokio task that calls `heartbeat()` every 30 seconds while agent is in Idle or Working state
- [ ] T040 [US2] Add integration test: enroll → heartbeat → receive lease → pause → verify checkpoint saved → resume → withdraw → scan for zero files/processes in tests/agent/test_lifecycle.rs
- [ ] T041 [US2] Add integration test: rapid pause/resume cycling (10 events/second) → verify stability in tests/agent/test_lifecycle.rs

### Preemption Supervisor (#45)

- [ ] T042 [US2] Wire `event_rx` channel in src/preemption/supervisor.rs: connect sovereignty trigger detection (keyboard/mouse/thermal/battery) to supervisor via tokio mpsc channel
- [ ] T043 [US2] Implement preemption handler in src/preemption/supervisor.rs: on event → record Instant::now() → SIGSTOP all sandbox PIDs via `nix::sys::signal::kill(pid, Signal::SIGSTOP)` → record elapsed → log latency
- [ ] T044 [US2] Implement checkpoint-or-kill escalation in src/preemption/supervisor.rs: after SIGSTOP, attempt checkpoint within 500ms budget; if timeout, send SIGKILL and reschedule from last committed checkpoint
- [ ] T045 [US2] Implement GPU kernel window handling in src/preemption/supervisor.rs: for GPU workloads, wait up to 200ms for kernel completion before SIGSTOP
- [ ] T046 [US2] Add integration test: inject simulated keyboard event → measure SIGSTOP latency → assert < 10ms in tests/preemption/test_supervisor.rs
- [ ] T047 [US2] Add integration test: checkpoint failure → SIGKILL escalation → verify sandbox terminated in tests/preemption/test_supervisor.rs
- [ ] T048 [US2] Remove all `// TODO` comments from src/agent/lifecycle.rs and src/preemption/supervisor.rs
- [ ] T049 [US2] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-003, FR-004, FR-005 satisfied. SC-005, SC-006 verifiable.

---

## Phase 5: US3 — Policy Engine Completion (Priority: P1) #31

**Goal**: Artifact registry lookup, egress allowlist validation, LLM advisory wiring.

**Independent Test**: Submit job with valid/invalid CID → verify accept/reject. Submit job with approved/unapproved endpoints → verify accept/reject.

- [ ] T050 [US3] Implement `check_artifact_registry()` in src/policy/rules.rs: resolve CID against ApprovedArtifact registry, verify signer ≠ approver (separation of duties), check release channel validity (dev→staging→production only)
- [ ] T051 [US3] Implement `check_egress_allowlist()` in src/policy/rules.rs: validate each declared endpoint in `job.allowed_endpoints` against EgressAllowlist.approved_endpoints, reject undeclared endpoints
- [ ] T052 [US3] Wire LLM advisory flag in src/policy/engine.rs: set `decision.llm_advisory_flag = false` by default; when mesh LLM is available (Phase G), route manifest through advisory classification
- [ ] T053 [US3] Add integration test: job with valid artifact CID → accepted in tests/policy/test_artifact_check.rs
- [ ] T054 [P] [US3] Add integration test: job with unknown CID → rejected with WC-006 in tests/policy/test_artifact_check.rs
- [ ] T055 [P] [US3] Add integration test: same identity as signer+approver → rejected in tests/policy/test_artifact_check.rs
- [ ] T056 [US3] Add integration test: job with approved endpoints → accepted; unapproved → rejected in tests/policy/test_egress.rs
- [ ] T057 [US3] Remove all `// TODO` comments from src/policy/rules.rs and src/policy/engine.rs
- [ ] T058 [US3] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-006, FR-007 satisfied. Policy engine 10-step pipeline fully operational.

---

## Phase 6: US4 — Sandbox Depth (Priority: P1) #32, #33, #34

**Goal**: GPU verification, Firecracker rootfs from CID store, real incident containment.

**Independent Test**: Enumerate GPUs → verify IOMMU. Store OCI image → assemble rootfs → boot Firecracker. Trigger FreezeHost → verify processes stopped.

### GPU Passthrough (#32)

- [ ] T059 [P] [US4] Implement PCI device enumeration via sysfs in src/sandbox/gpu.rs `check_linux_gpu()`: read `/sys/bus/pci/devices/*/class` for VGA controllers (0x030000)
- [ ] T060 [US4] Implement IOMMU group check in src/sandbox/gpu.rs: read `/sys/bus/pci/devices/{dev}/iommu_group/devices/` and verify GPU is sole member
- [ ] T061 [US4] Implement ACS-override detection in src/sandbox/gpu.rs: check `/sys/module/vfio/parameters/enable_unsafe_noiommu_mode` and kernel command line for `pcie_acs_override`
- [ ] T062 [US4] Add integration test: GPU in singleton IOMMU group → allowed; shared group → rejected in tests/sandbox/test_gpu.rs

### Firecracker Rootfs (#33)

- [ ] T063 [P] [US4] Implement OCI image fetch from CID store in src/sandbox/firecracker.rs `prepare_rootfs()`: retrieve layer CIDs from manifest, fetch each layer blob
- [ ] T064 [US4] Implement OCI layer extraction and overlay in src/sandbox/firecracker.rs: extract tar layers in order, create ext4 filesystem image via `mkfs.ext4` + loop mount + copy
- [ ] T065 [US4] Wire rootfs into Firecracker VM config in src/sandbox/firecracker.rs `start()`: mount assembled rootfs.ext4 as root drive
- [ ] T066 [US4] Add integration test: store minimal OCI image → prepare rootfs → boot Firecracker → verify output in tests/sandbox/test_firecracker_rootfs.rs

### Incident Containment (#34)

- [ ] T067 [P] [US4] Implement FreezeHost in src/incident/containment.rs: enumerate all sandbox PIDs on target host, send SIGSTOP to each, block new lease assignments for host
- [ ] T068 [US4] Implement QuarantineWorkloadClass in src/incident/containment.rs: add class to policy engine's quarantine list so `check_workload_class()` rejects it
- [ ] T069 [US4] Implement BlockSubmitter in src/incident/containment.rs: add submitter to ban list, cancel all in-flight jobs from submitter, reject new submissions
- [ ] T070 [US4] Implement RevokeArtifact in src/incident/containment.rs: remove CID from ApprovedArtifact registry, halt all running jobs that loaded the revoked artifact
- [ ] T071 [US4] Implement DrainHostPool in src/incident/containment.rs: migrate all active workloads to other nodes (checkpoint + reschedule), block new assignments to pool
- [ ] T072 [US4] Add integration test for each containment primitive: execute → verify enforcement effect in tests/incident/test_enforcement.rs
- [ ] T073 [US4] Remove all `// TODO` comments from src/sandbox/gpu.rs, src/sandbox/firecracker.rs, src/incident/containment.rs
- [ ] T074 [US4] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-008, FR-009, FR-010 satisfied.

---

## Phase 7: US5 — Security Hardening (Priority: P1) #35, #46, #47, #53

**Goal**: All adversarial tests implemented, confidential compute, mTLS, reproducible builds.

**Independent Test**: Run adversarial tests on KVM host. Encrypt/decrypt round-trip. mTLS handshake. Reproducible build comparison.

### Adversarial Tests (#35)

- [ ] T075 [P] [US5] Implement `malformed_peer_flood` test in tests/adversarial/test_flood_resilience.rs: inject malformed gossipsub messages for 60s, verify cluster remains operational
- [ ] T076 [P] [US5] Implement `job_submit_flood_rate_limited` test in tests/adversarial/test_flood_resilience.rs: submit 1000 jobs in 1s, verify rate limiting activates
- [ ] T077 [P] [US5] Implement `sandbox_escape_via_ptrace` test in tests/adversarial/test_sandbox_escape.rs: attempt ptrace from inside Firecracker VM, verify blocked
- [ ] T078 [P] [US5] Implement `sandbox_escape_via_container_runtime` test in tests/adversarial/test_sandbox_escape.rs: attempt container breakout, verify blocked
- [ ] T079 [P] [US5] Implement `network_escape_via_host_bridge` test in tests/adversarial/test_network_isolation.rs: attempt to reach host bridge from sandbox, verify blocked
- [ ] T080 [P] [US5] Implement `network_escape_via_dns_intercept` test in tests/adversarial/test_network_isolation.rs: attempt DNS hijacking from sandbox, verify blocked
- [ ] T081 [P] [US5] Implement `byzantine_data_corruption` test in tests/adversarial/test_byzantine_donor.rs: inject corrupted result, verify detection within 100 audited tasks
- [ ] T082 [P] [US5] Implement `byzantine_quorum_bypass` test in tests/adversarial/test_byzantine_donor.rs: attempt to bypass quorum with colluding nodes, verify detected
- [ ] T083 [US5] Remove all `#[ignore]` and `unimplemented!()` from tests/adversarial/

### Confidential Compute (#46)

- [ ] T084 [P] [US5] Implement client-side AES-256-GCM encryption in src/data_plane/confidential.rs: generate ephemeral 256-bit key via OsRng, encrypt job inputs, store ciphertext in CID store
- [ ] T085 [US5] Implement key wrapping in src/data_plane/confidential.rs: wrap ephemeral key with submitter's public key via X25519 key agreement (x25519-dalek), store wrapped key in ConfidentialBundle
- [ ] T086 [US5] Implement TPM-attested key release for confidential-medium in src/data_plane/confidential.rs: verify node attestation before releasing wrapped key
- [ ] T087 [US5] Implement guest-measurement sealed key for confidential-high in src/data_plane/confidential.rs: key released only to sandbox matching expected guest measurement hash
- [ ] T088 [US5] Add integration test: encrypt → store → execute on attested node → decrypt → verify correct result in tests/data_plane/test_confidential.rs
- [ ] T089 [US5] Add integration test: attempt key release without attestation → denied in tests/data_plane/test_confidential.rs

### mTLS and Rate Limiting (#47)

- [ ] T090 [P] [US5] Implement Ed25519 certificate issuance in src/network/tls.rs: generate self-signed CA, issue per-account certificates using rcgen
- [ ] T091 [US5] Implement 90-day auto-rotation in src/network/tls.rs: check cert expiry on heartbeat, trigger renewal when < 7 days remaining
- [ ] T092 [US5] Implement token bucket rate limiter in src/network/rate_limit.rs: DONOR_HEARTBEAT 120/min, JOB_SUBMIT 10/min, GOVERNANCE 5/min, CLUSTER_STATUS 30/min with Retry-After header
- [ ] T093 [US5] Add integration test: mTLS handshake succeeds with valid cert, fails without in tests/network/test_tls.rs
- [ ] T094 [US5] Add integration test: exceed rate limit → verify 429 with Retry-After in tests/network/test_rate_limit.rs

### Supply Chain (#53)

- [ ] T095 [P] [US5] Implement reproducible build configuration in build.rs: set deterministic flags (RUSTFLAGS=-Cdebuginfo=0, source date epoch)
- [ ] T096 [US5] Implement Ed25519 binary signing in src/agent/mod.rs: sign release binary with project key, verify signature on agent startup
- [ ] T097 [US5] Implement agent version verification in src/agent/lifecycle.rs: on heartbeat, check peer's agent version against known-good list, reject unknown versions
- [ ] T098 [US5] Add integration test: two builds from same commit → identical binary in tests/test_reproducible_build.rs
- [ ] T099 [US5] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-011 through FR-017 satisfied. SC-002 (zero ignored tests).

---

## Phase 8: US6 — Integration Test Coverage and Validation (Priority: P1) #36, #51, #42

**Goal**: Every src/ module has integration tests. Churn simulation. Phase 1 LAN testnet.

**Independent Test**: `cargo test` reports 700+ tests. Churn sim reports 80%+ completion. 3-node cluster forms in <5s.

### Module Integration Tests (#36)

- [ ] T100 [P] [US6] Add integration tests for src/acceptable_use/ in tests/acceptable_use/test_filter.rs: test workload classification, prohibited class rejection
- [ ] T101 [P] [US6] Add integration tests for src/agent/ in tests/agent/test_enrollment.rs: enrollment flow, state transitions, config loading
- [ ] T102 [P] [US6] Add integration tests for src/cli/ in tests/cli/test_commands.rs: each CLI subcommand produces expected output
- [ ] T103 [P] [US6] Add integration tests for src/credits/ in tests/credits/test_ncu.rs: NCU computation, caliber matching, DRF accounting
- [ ] T104 [P] [US6] Add integration tests for src/data_plane/ in tests/data_plane/test_cid_store.rs: put/get/has/delete, erasure encode/decode
- [ ] T105 [P] [US6] Add integration tests for src/ledger/ in tests/ledger/test_crdt.rs: OR-Map operations, merge, balance verification
- [ ] T106 [P] [US6] Add integration tests for src/network/ in tests/network/test_discovery.rs: mDNS, Kademlia, gossipsub message passing
- [ ] T107 [P] [US6] Add integration tests for src/preemption/ in tests/preemption/test_triggers.rs: sovereignty event detection, timer accuracy
- [ ] T108 [P] [US6] Add integration tests for src/registry/ in tests/registry/test_artifacts.rs: approved artifact CRUD, release channel enforcement
- [ ] T109 [P] [US6] Add integration tests for src/scheduler/ in tests/scheduler/test_broker.rs: task matching, lease lifecycle, priority scoring
- [ ] T110 [P] [US6] Add integration tests for src/telemetry/ in tests/telemetry/test_redaction.rs: PII redaction, span creation, metric reporting
- [ ] T111 [P] [US6] Add integration tests for src/verification/ in tests/verification/test_trust_score.rs: trust score computation, tier classification, quorum verification
- [ ] T112 [US6] Remove empty test directories (tests/contract/, tests/integration/, tests/unit/) or populate them

### Churn Simulator (#51)

- [ ] T113 [US6] Build churn simulator harness in tests/churn/simulator.rs: configurable node count, churn rate, job stream, checkpoint/resume tracking
- [ ] T114 [US6] Implement random node kill/rejoin logic in tests/churn/simulator.rs: select random node, kill process, wait random interval, rejoin
- [ ] T115 [US6] Implement job completion tracking in tests/churn/simulator.rs: track submitted vs completed vs failed, report completion rate
- [ ] T116 [US6] Add integration test: 20+ simulated nodes, 30% churn, run for configurable duration, assert >= 80% completion in tests/churn/test_churn.rs

### Phase 1 LAN Testnet (#42)

- [ ] T117 [US6] Create multi-node test harness in tests/integration/test_lan_testnet.rs: spawn 3+ agent processes on the same host (multi-process simulation acceptable for CI; real multi-machine test on tensor01.dartmouth.edu for Phase 1 evidence artifact), verify mDNS discovery < 5 seconds
- [ ] T118 [US6] Add R=3 job execution test in tests/integration/test_lan_testnet.rs: submit job → verify dispatched to 3 nodes → collect quorum result
- [ ] T119 [US6] Add failure recovery test in tests/integration/test_lan_testnet.rs: kill one node mid-job → verify job reschedules from checkpoint → correct result
- [ ] T120 [US6] Add preemption test in tests/integration/test_lan_testnet.rs: inject keyboard event → verify preemption < 1s → verify job continues after resume
- [ ] T121 [US6] Generate evidence artifact JSON for Phase 1 in evidence/phase1/results.json
- [ ] T122 [US6] Run `cargo test` to verify 700+ total tests passing

**Checkpoint**: FR-018, FR-019, FR-020 satisfied. SC-003, SC-004, SC-007, SC-008 verifiable.

---

## Phase 9: US7 — Runtime Systems (Priority: P2) #44, #49, #55, #56

**Goal**: Credit decay, storage GC, real broker matchmaking, threshold signing.

**Independent Test**: Simulate 90-day credits → verify decay curve. Fill storage to cap → verify GC. Multi-node matchmaking. 5-coordinator threshold signing.

### Credits (#44)

- [ ] T123 [P] [US7] Implement 45-day half-life credit decay in src/credits/decay.rs: `balance_after = balance_before * 0.5^(days/45)`, apply daily, create CreditDecayEvent ledger entry
- [ ] T124 [US7] Implement floor protection in src/credits/decay.rs: `floor = trailing_30d_earn_rate * 30`, do not decay below floor for active donors
- [ ] T125 [US7] Implement anti-hoarding in src/credits/decay.rs: if outstanding credits > 110% of trailing redemption demand, multiply decay rate by 1.5
- [ ] T126 [US7] Add integration test: simulate 90 days → verify decay matches half-life within 1% in tests/credits/test_decay.rs

### Storage GC and Acceptable Use (#49)

- [ ] T127 [P] [US7] Implement per-donor storage tracking in src/data_plane/cid_store.rs: track used_bytes per node, reject new data when cap exceeded
- [ ] T128 [US7] Implement GC for expired/orphaned data in src/data_plane/cid_store.rs: scan for data past retention period or from withdrawn donors, delete and reclaim space
- [ ] T129 [US7] Implement acceptable-use filter in src/acceptable_use/filter.rs: classify workload at submission, reject prohibited classes (scanning, malware, surveillance, credential cracking)
- [ ] T130 [US7] Implement shard residency enforcement in src/data_plane/placement.rs: enforce per-donor shard-category allowlist (EU/US/UK/JP data placed only on matching-jurisdiction nodes)
- [ ] T131 [US7] Add integration test: fill to cap → verify rejection → GC → verify space freed in tests/data_plane/test_storage_gc.rs

### Scheduler (#55)

- [ ] T132 [P] [US7] Implement ClassAd-style matchmaking in src/scheduler/broker.rs: compare task requirements (CPU, GPU, memory, trust tier, region) against node capabilities, return ranked matches
- [ ] T133 [US7] Implement lease issuance in src/scheduler/broker.rs: create Lease with configurable TTL (default 300s), track in broker's lease table
- [ ] T134 [US7] Implement lease renewal in src/scheduler/broker.rs: on heartbeat from leased node, update `renewed_at`, extend TTL
- [ ] T135 [US7] Implement lease expiry handling in src/scheduler/broker.rs: detect expired leases, mark Expired, trigger rescheduling from last checkpoint
- [ ] T136 [US7] Implement R=3 disjoint-AS placement in src/scheduler/broker.rs: ensure 3 replicas are on nodes in different autonomous systems
- [ ] T137 [US7] Add integration test: submit job → broker matches to capable node → verify lease lifecycle in tests/scheduler/test_matchmaking.rs

### Ledger (#56)

- [ ] T138 [P] [US7] Implement t-of-n threshold signing in src/ledger/threshold_sig.rs: use threshold-crypto for 3-of-5 BLS threshold signatures, dealer key generation, share distribution
- [ ] T139 [US7] Implement CRDT OR-Map merge in src/ledger/crdt.rs: merge function for coordinator replicas, conflict resolution via causal ordering
- [ ] T140 [US7] Implement cross-shard MerkleRoot computation in src/ledger/transparency.rs: compute root of all coordinator log heads every 10 minutes, anchor to Rekor
- [ ] T141 [US7] Implement local balance verification in src/credits/ncu.rs: O(log n) proof verification for `worldcompute donor credits --verify`
- [ ] T142 [US7] Implement graceful degradation (FR-028a) in src/scheduler/broker.rs: when coordinator quorum lost, continue dispatching from cached leases, queue ledger writes locally, CRDT merge on rejoin
- [ ] T143 [US7] Add integration test: 5 coordinators → sign entry → verify 3-of-5 threshold in tests/ledger/test_threshold.rs
- [ ] T144 [US7] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-025 through FR-028a satisfied.

---

## Phase 10: US8 — Platform Adapters (Priority: P2) #37, #38, #39, #52

**Goal**: Slurm, Kubernetes, Cloud, and Apple VF adapters functional with real backends.

**Independent Test**: Slurm adapter dispatches job via sbatch. K8s operator creates Pod. Cloud adapter verifies instance identity. Apple VF boots VM.

### Slurm (#37)

- [ ] T145 [P] [US8] Implement slurmrestd HTTP client in adapters/slurm/src/main.rs: connect to Slurm REST API, GET /slurm/v0.0.40/nodes for capacity reporting
- [ ] T146 [US8] Implement job dispatch via sbatch in adapters/slurm/src/main.rs: POST /slurm/v0.0.40/job/submit with job script, track job ID
- [ ] T147 [US8] Implement result collection in adapters/slurm/src/main.rs: poll GET /slurm/v0.0.40/job/{id} until COMPLETED, fetch output
- [ ] T148 [US8] Add integration test: submit SHA-256 test job to Slurm → verify correct result in adapters/slurm/tests/test_slurm.rs (if no real Slurm cluster available, test uses mock slurmrestd server returning known responses; document limitation in test comments)

### Kubernetes (#38)

- [ ] T149 [P] [US8] Implement CRD watch loop in adapters/kubernetes/src/main.rs: use kube::runtime::watcher for ClusterDonation CRD changes
- [ ] T150 [US8] Implement Pod creation in adapters/kubernetes/src/main.rs: on CRD create, create Pod with resource limits from CRD spec
- [ ] T151 [US8] Implement result collection and cleanup in adapters/kubernetes/src/main.rs: watch Pod status, collect logs on completion, delete Pod
- [ ] T152 [US8] Create Helm chart in adapters/kubernetes/helm/: deployment, service, RBAC, CRD definition
- [ ] T153 [US8] Add integration test: deploy on minikube → apply CRD → verify Pod created → verify result collected in adapters/kubernetes/tests/test_k8s.rs

### Cloud (#39)

- [ ] T154 [P] [US8] Implement AWS IMDSv2 attestation in adapters/cloud/src/main.rs: GET token → GET instance identity document → verify signature against AWS public key
- [ ] T155 [P] [US8] Implement GCP metadata attestation in adapters/cloud/src/main.rs: GET instance identity token → verify JWT against Google public keys
- [ ] T156 [P] [US8] Implement Azure IMDS attestation in adapters/cloud/src/main.rs: GET attested data → verify signature against Azure certificate
- [ ] T157 [US8] Add integration test on real cloud instance: verify identity attestation in adapters/cloud/tests/test_cloud.rs (if no real cloud instance available, test verifies parsing logic against known IMDSv2/GCP/Azure response fixtures; document limitation in test comments)

### Apple VF (#52)

- [ ] T158 [P] [US8] Create Swift package in tools/apple-vf-helper/Package.swift: target macOS 13+, import Virtualization framework
- [ ] T159 [US8] Implement VM create/start in tools/apple-vf-helper/Sources/main.swift: VZVirtualMachineConfiguration with CPU, memory, disk, network; VZVirtualMachine.start()
- [ ] T160 [US8] Implement pause/resume/stop/checkpoint in tools/apple-vf-helper/Sources/main.swift: JSON command protocol on stdin/stdout
- [ ] T161 [US8] Wire Rust integration in src/sandbox/apple_vf.rs: spawn helper binary, send JSON commands, parse responses
- [ ] T162 [US8] Add integration test (macOS only): boot VM → execute workload → capture output in tests/sandbox/test_apple_vf.rs
- [ ] T163 [US8] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-021 through FR-024 satisfied.

---

## Phase 11: US9 — User-Facing Features (Priority: P2) #40, #43

**Goal**: Tauri GUI and REST gateway functional.

**Independent Test**: Launch Tauri → submit job through GUI. Call REST endpoint → verify response matches CLI.

### Tauri GUI (#40)

- [ ] T164 [P] [US9] Initialize Tauri window in gui/src-tauri/src/main.rs: replace print-only demo with real Tauri::Builder, create window
- [ ] T165 [US9] Implement backend IPC commands in gui/src-tauri/src/commands.rs: replace stub returns with real agent/scheduler/governance calls
- [ ] T166 [US9] Create React frontend scaffold in gui/src/: package.json, tsconfig.json, index.html, App.tsx
- [ ] T167 [P] [US9] Implement DonorDashboard page in gui/src/pages/DonorDashboard.tsx: enrollment status, credit balance, trust score, active leases
- [ ] T168 [P] [US9] Implement SubmitterDashboard page in gui/src/pages/SubmitterDashboard.tsx: job submission form, job list, status, results
- [ ] T169 [P] [US9] Implement GovernanceBoard page in gui/src/pages/GovernanceBoard.tsx: proposal list, create, vote, results
- [ ] T170 [US9] Implement Settings page in gui/src/pages/Settings.tsx: workload class opt-in/out, CPU cap, storage cap, OTel endpoint

### REST Gateway (#43)

- [ ] T171 [P] [US9] Implement HTTP+JSON gateway in src/network/rest_gateway.rs: expose all 6 gRPC services via tonic-web with JSON transcoding
- [ ] T172 [US9] Wire rate limiting into REST gateway in src/network/rest_gateway.rs: apply per-class rate limits from FR-015
- [ ] T173 [US9] Wire Ed25519 token authentication into REST gateway in src/network/rest_gateway.rs
- [ ] T174 [US9] Add integration test: REST API submit job → verify completion in tests/network/test_rest.rs

### Web Dashboard SPA (FR-031)

- [ ] T174a [P] [US9] Create static web dashboard SPA scaffold in gui/src/web/: index.html, package.json (React + TypeScript), build to gui/src/web/dist/ for CDN deployment
- [ ] T174b [US9] Implement donor status page in gui/src/web/pages/DonorStatus.tsx: fetch from REST gateway, display credit balance, trust score, active leases
- [ ] T174c [P] [US9] Implement job submission page in gui/src/web/pages/JobSubmit.tsx: form for manifest upload, job list with status, result download
- [ ] T174d [US9] Add integration test: load web dashboard → submit job via REST → verify result displayed in tests/network/test_web_dashboard.rs

- [ ] T175 [US9] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-029 through FR-031 satisfied.

---

## Phase 12: US10 — Operations (Priority: P2) #41, #48, #50

**Goal**: Docker, energy metering, documentation.

**Independent Test**: `docker build` → verify image. Energy estimate within 20% of wall-meter. README quickstart works on clean machine.

### Deployment (#41)

- [ ] T176 [P] [US10] Create multi-stage Dockerfile at repository root: stage 1 rust:1.95-bookworm build, stage 2 distroless runtime
- [ ] T177 [US10] Create docker-compose.yml at repository root: 3 services (coordinator, broker, agent) with shared network, verify cluster formation
- [ ] T178 [US10] Create Helm chart in deploy/helm/worldcompute/: Chart.yaml, values.yaml, templates for coordinator StatefulSet + agent DaemonSet

### Energy Metering (#48)

- [ ] T179 [P] [US10] Implement RAPL energy reading in src/telemetry/energy.rs: read `/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj` before/after job, compute joules
- [ ] T180 [US10] Implement GPU power reading via NVML in src/telemetry/energy.rs: `nvmlDeviceGetPowerUsage()` for NVIDIA GPUs (optional — skip if no GPU)
- [ ] T181 [US10] Implement aggregate carbon footprint in src/telemetry/energy.rs: multiply watts by regional carbon intensity (configurable g CO2/kWh)
- [ ] T182 [US10] Add integration test: run workload → read RAPL → verify non-zero joules in tests/telemetry/test_energy.rs
- [ ] T182a [US10] Calibration test on tensor01.dartmouth.edu: run standardized workload, compare RAPL reading against wall-meter measurement (if available) or known TDP, document calibration factor, assert estimates within 20% (SC target)

### Documentation (#50)

- [ ] T183 [P] [US10] Write comprehensive README.md at repository root: project overview, architecture, quickstart, API reference, contribution guide
- [ ] T184 [US10] Create evidence artifact JSON schema in evidence/schema.json: jobs run, systems tested, expected vs observed outputs
- [ ] T185 [US10] Create incident disclosure policy in docs/security/incident-disclosure-policy.md
- [ ] T186 [US10] Create legal entity placeholder in docs/legal/entity.md (501(c)(3), bylaws, quarterly report template)
- [ ] T187 [US10] Verify README quickstart works on clean machine
- [ ] T188 [US10] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-032 through FR-036 satisfied.

---

## Phase 13: US11 — Distributed Mesh LLM (Priority: P3) #54

**Goal**: Ensemble-of-experts inference with router, aggregator, self-prompting, safety tiers, kill switch.

**Independent Test**: 4+ GPU nodes → register → generate 100 tokens → verify coherent output. Kill switch → verify halt.

### Router and Expert

- [ ] T189 [P] [US11] Implement K-of-N expert selection in src/agent/mesh_llm/router.rs: select K experts (default 4) based on health, latency, load; dispatch prompt in parallel via gRPC
- [ ] T190 [P] [US11] Implement expert registration and health tracking in src/agent/mesh_llm/expert.rs: register with router, report model name/tokenizer/VRAM/health, periodic heartbeat
- [ ] T191 [US11] Implement model loading via candle in src/agent/mesh_llm/expert.rs: load LLaMA-3-8B-Q4_K_M.gguf, run inference, return top-256 (token_id, logit) pairs

### Aggregator

- [ ] T192 [US11] Implement sparse logit aggregation in src/agent/mesh_llm/aggregator.rs: receive top-256 logits from K experts, compute weighted average, apply temperature, sample next token
- [ ] T193 [US11] Implement tokenizer integration in src/agent/mesh_llm/aggregator.rs: use LLaMA-3 tokenizer (128K vocab) via `tokenizers` crate for encode/decode

### Self-Prompting and Safety

- [ ] T194 [US11] Implement self-prompting loop in src/agent/mesh_llm/self_prompt.rs: observe cluster metrics → generate improvement task → classify action tier → route for approval → execute if approved → measure → repeat on 1-24 hour cadence
- [ ] T195 [US11] Implement action tier classification in src/agent/mesh_llm/safety.rs: parse mesh output, classify into ReadOnly/Suggest/SandboxTest/DeployMinor/DeployMajor based on content analysis
- [ ] T196 [US11] Implement governance kill switch in src/agent/mesh_llm/safety.rs: on signed GossipSub halt message from any governance participant → immediately stop all inference streams, revert last 3 applied changes, enter read-only mode

### gRPC Service

- [ ] T197 [US11] Implement MeshLLMService gRPC handlers in src/agent/mesh_llm/service.rs: RegisterExpert, GetRouterStatus, SubmitSelfTask, HaltMesh per contracts/mesh-llm-contract.md
- [ ] T198 [US11] Implement graceful degradation below 280 nodes in src/agent/mesh_llm/router.rs: fall back to centralized model when insufficient experts available

### Integration

- [ ] T199 [US11] Add integration test: register 4 mock experts → generate token via sparse aggregation → verify valid token ID in tests/mesh_llm/test_inference.rs
- [ ] T200 [US11] Add integration test: trigger kill switch → verify all streams halted within 1 second in tests/mesh_llm/test_safety.rs
- [ ] T201 [US11] Add integration test: submit self-task → verify action tier classification → verify governance gating in tests/mesh_llm/test_self_prompt.rs
- [ ] T202 [US11] Run `cargo test` to verify zero regressions

**Checkpoint**: FR-037 through FR-043 satisfied. SC-010, SC-011 verifiable on GPU hardware.

---

## Phase 14: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all stories

- [ ] T203 [P] Run full regression: `cargo test` — all tests must pass, count >= 700 (SC-004)
- [ ] T204 [P] Run full clippy: `cargo clippy --lib -- -D warnings` — zero warnings
- [ ] T205 [P] Run full fmt: `cargo fmt --check` — clean
- [ ] T206 Verify zero TODO comments: `grep -rn "// TODO" src/` returns 0 results (SC-001)
- [ ] T207 Verify zero ignored tests: `grep -rn '#\[ignore\]' tests/` returns 0 results (SC-002)
- [ ] T208 Verify all 12 previously untested modules have integration tests (SC-003)
- [ ] T209 Run quickstart validation: execute each command from specs/004-full-implementation/quickstart.md
- [ ] T210 Update CLAUDE.md: test count, module count, remaining stubs (should be zero)
- [ ] T211 Update notes/ with session summary

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 Attestation (Phase 3)**: Depends on Phase 2 only
- **US2 Lifecycle (Phase 4)**: Depends on Phase 2 only
- **US3 Policy (Phase 5)**: Depends on Phase 2 only
- **US4 Sandbox (Phase 6)**: Depends on Phase 2 only
- **US5 Security (Phase 7)**: Depends on Phases 3, 4, 6 (needs attestation, preemption, containment)
- **US6 Test Coverage (Phase 8)**: Depends on Phases 3–7 (needs implementations to test)
- **US7 Runtime (Phase 9)**: Depends on Phase 2 only; can run parallel with 3–6
- **US8 Adapters (Phase 10)**: Depends on Phase 9 (needs scheduler for dispatch)
- **US9 GUI/REST (Phase 11)**: Depends on Phases 3–9 (needs backend functionality)
- **US10 Operations (Phase 12)**: Depends on Phase 2 only; can run parallel with 3–9
- **US11 Mesh LLM (Phase 13)**: Depends on Phases 3–9 (needs functioning cluster)
- **Polish (Phase 14)**: Depends on ALL phases

### Parallel Opportunities

- T019–T023: All cert validators touch different code paths — fully parallel
- T059–T063, T067: GPU, rootfs, containment are independent modules — parallel
- T075–T082: All adversarial tests are independent — fully parallel
- T100–T111: All module integration tests are independent — fully parallel
- T145, T149, T154, T158: All adapter tracks are independent — fully parallel
- T164–T170: All GUI pages are independent — fully parallel
- T189–T190: Router and expert are independent — parallel

---

## Implementation Strategy

### MVP First (Phases 1–6)

1. Setup + Foundational (T001–T018)
2. Deep attestation + Rekor (T019–T035)
3. Agent lifecycle + preemption (T036–T049)
4. Policy engine (T050–T058)
5. Sandbox depth (T059–T074)
6. **STOP and VALIDATE**: All P1 core infrastructure working, tested on real hardware

### Security Gate (Phase 7)

7. Adversarial tests + confidential compute + mTLS + supply chain (T075–T099)
8. **STOP and VALIDATE**: All Principle I requirements met

### Full Coverage (Phase 8)

9. Integration tests + churn sim + LAN testnet (T100–T122)
10. **STOP and VALIDATE**: Principle V evidence artifacts produced

### Incremental Delivery (Phases 9–14)

11. Runtime systems, adapters, GUI, operations, mesh LLM (T123–T211)
12. Final polish and validation
