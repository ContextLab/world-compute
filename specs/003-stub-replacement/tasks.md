# Tasks: Replace Implementation Stubs

**Input**: Design documents from `/specs/003-stub-replacement/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Exact file paths included in all descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add new dependencies and shared utilities needed by multiple user stories

- [x] T001 Add reqwest dependency to Cargo.toml: `reqwest = { version = "0.12", features = ["json", "rustls-tls"] }`
- [x] T002 Add oauth2 dependency to Cargo.toml: `oauth2 = "4"`
- [x] T003 Add x509-parser dependency to Cargo.toml: `x509-parser = "0.16"`
- [x] T004 Verify build succeeds with new dependencies: `cargo build --lib`

---

## Phase 2: Foundational — CLI Dispatch (Blocking Prerequisites)

**Purpose**: Wire all CLI subcommands into main.rs. This MUST complete before any user story can be tested end-to-end via CLI.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

> **Story numbering note**: Spec User Story 1 (CLI operations) is covered here as foundational infrastructure. Tasks phases 3–9 map to spec User Stories 2–8 respectively (US2=Sandbox, US3=Attestation, US4=Identity, US5=Transparency, US6=Observability, US7=Consensus, US8=Network).

- [x] T005 [P] [FR-001] Change `Commands::Donor` from unit variant to `Donor(cli::donor::DonorCli)` and dispatch to `cli::donor::execute()` in src/main.rs
- [x] T006 [P] [FR-002] Change `Commands::Job` from unit variant to `Job(cli::submitter::SubmitterCli)` and dispatch to `cli::submitter::execute()` in src/main.rs
- [x] T007 [P] [FR-003] Create `ClusterCli` struct with `status`, `peers`, `ledger-head` subcommands in src/cli/mod.rs (or new src/cli/cluster.rs), wire into `Commands::Cluster` in src/main.rs
- [x] T008 [P] [FR-004] Change `Commands::Governance` from unit variant to `Governance(cli::governance::GovernanceCli)` and dispatch to `cli::governance::execute()` in src/main.rs
- [x] T009 [P] [FR-005] Change `Commands::Admin` from unit variant to `Admin(cli::admin::AdminCli)` and dispatch to `cli::admin::execute()` in src/main.rs
- [x] T010 [FR-001] Update src/cli/donor.rs `execute()` to call real agent lifecycle functions instead of returning placeholder strings
- [x] T011 [FR-002] Update src/cli/submitter.rs `execute()` to call real scheduler functions instead of returning placeholder strings
- [x] T012 [FR-004] Update src/cli/governance.rs `execute()` to call real governance module functions instead of returning placeholder strings
- [x] T013 [FR-005] Update src/cli/admin.rs `execute()` to call real admin service functions, enforcing OnCallResponder role
- [x] T014 Run `cargo test` and `cargo clippy --lib -- -D warnings` to verify zero regressions and zero warnings
- [x] T015 Verify each CLI command produces meaningful output (not "not yet implemented"): `cargo run -- donor status`, `cargo run -- job list`, etc.

**Checkpoint**: All 5 CLI command groups dispatch to real modules. SC-001 is satisfied.

---

## Phase 3: User Story 2 — Sandboxed Workload Execution (Priority: P1) 🎯 MVP

**Goal**: A submitted workload executes in a real sandbox (Firecracker, Apple VF, or WASM) and returns results.

**Independent Test**: Submit a sample WASM workload via CLI and verify it completes end-to-end within 60 seconds.

### WASM (cross-platform, start here)

- [x] T016 [FR-008] [US2] Implement CID store fetch in src/sandbox/wasm.rs `create()` (line 35): fetch WASM bytes from `data_plane::cid_store` by CID
- [x] T017 [FR-008] [US2] Implement wasmtime compilation in src/sandbox/wasm.rs `create()`: `wasmtime::Module::new(&engine, &wasm_bytes)`
- [x] T018 [FR-008] [US2] Implement WASI instantiation in src/sandbox/wasm.rs `start()` (line 43): create WASI context, instantiate module, call `_start` or entry function
- [x] T019 [FR-008] [US2] Implement stdout/stderr capture in src/sandbox/wasm.rs `run_module()` (line 106): return output bytes instead of empty Vec

### Firecracker (Linux)

- [ ] T020 [P] [FR-006] [US2] Implement Firecracker API socket HTTP client in src/sandbox/firecracker.rs: PUT requests over Unix domain socket using hyper
- [ ] T021 [FR-006] [US2] Implement VM configuration sequence in src/sandbox/firecracker.rs `start()` (line 227): PUT /machine-config → /boot-source → /drives/rootfs → /network-interfaces/eth0 → /actions InstanceStart
- [ ] T022 [FR-006] [US2] Implement snapshot creation in src/sandbox/firecracker.rs `checkpoint()` (line 274): PUT /snapshot/create with JSON body
- [ ] T023 [FR-006] [US2] Implement FirecrackerVmConfig struct with validation (vcpu_count ≥ 1, mem_size_mib ≥ 128) in src/sandbox/firecracker.rs
- [ ] T024 [FR-006a] [US2] Implement max-3-donor retry logic: on Firecracker API error, mark donor incompatible, reschedule; fail task after 3 attempts

### Apple Virtualization.framework (macOS)

- [ ] T025 [P] [FR-007] [US2] Create Swift helper binary `wc-apple-vf-helper` (new directory: tools/apple-vf-helper/) accepting JSON commands on stdin, returning JSON on stdout
- [ ] T026 [FR-007] [US2] Implement VZVirtualMachineConfiguration create/start in tools/apple-vf-helper/
- [ ] T027 [FR-007] [US2] Implement pause/resume/stop/checkpoint commands in tools/apple-vf-helper/
- [ ] T028 [FR-007] [US2] Wire src/sandbox/apple_vf.rs `start()` (line 138), `freeze()` (line 154), `checkpoint()` (line 173), `terminate()` (line 191) to call helper binary via subprocess

### Integration

- [ ] T029 [US2] Add integration test: submit WASM "hello world" workload → verify output in tests/sandbox/
- [ ] T030 [US2] Add integration test: Firecracker VM boot + execute + terminate (Linux only, requires KVM) in tests/sandbox/
- [ ] T031 [US2] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-002 satisfied — sample workload completes in under 60 seconds on at least one platform.

---

## Phase 4: User Story 3 — Hardware Attestation (Priority: P2)

**Goal**: Full certificate-chain validation for TPM2, SEV-SNP, TDX, and Apple SE; real Ed25519 signature verification.

**Independent Test**: Present known-good and known-bad attestation test vectors and verify 100% correct accept/reject.

### Ed25519 (no external deps)

- [ ] T032 [P] [FR-009] [US3] Replace structural signature check in src/policy/rules.rs `check_signature()` (line 60) with `ed25519_dalek::VerifyingKey::from_bytes()` → `.verify(&message, &signature)`

### Certificate Chain Validation

- [ ] T033 [P] [FR-010] [US3] Define `CertificateChainValidator` trait in src/verification/attestation.rs: `validate_chain(quote, certs) → Result<bool>` + `root_ca() → Certificate`
- [ ] T034 [FR-010] [US3] Implement `Tpm2ChainValidator`: parse EK certificate, verify AIK signature against EK, verify quote against AIK in src/verification/attestation.rs
- [ ] T035 [P] [FR-010] [US3] Implement `SevSnpChainValidator`: validate ARK → ASK → VCEK chain, verify attestation report signature in src/verification/attestation.rs
- [ ] T036 [P] [FR-010] [US3] Implement `TdxChainValidator`: validate Intel DCAP root → PCK cert → quote signature in src/verification/attestation.rs
- [ ] T037 [FR-010] [US3] Bundle AMD ARK/ASK and Intel DCAP root CA certificates as compile-time constants in src/verification/attestation.rs
- [ ] T038 [FR-010] [US3] Wire validators into `verify_tpm2()` (line 401), `verify_sev_snp()` (line 410), `verify_tdx()` (line 418), replacing stubbed `verify_quote_signature()`

### Apple Secure Enclave

- [ ] T039 [FR-011] [US3] Implement `AppleSeValidator` in src/verification/attestation.rs `verify_apple_se()` (line 426): HTTP POST to Apple App Attest API via reqwest, parse CBOR response

### Integration

- [ ] T040 [US3] Add integration tests with real certificate chain test vectors (AMD ARK/ASK/VCEK, Intel DCAP, TPM EK) in tests/attestation/
- [ ] T041 [US3] Add integration test for Ed25519 policy verification with real key pairs in tests/policy/
- [ ] T042 [US3] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-003 satisfied — 100% accuracy on test vectors. SC-010 regression baseline maintained.

---

## Phase 5: User Story 4 — Identity Verification (Priority: P2)

**Goal**: At least one identity path (BrightID, OAuth2, or phone) completes end-to-end.

**Independent Test**: Initiate a BrightID verification and confirm the HTTP call returns a valid status.

### BrightID

- [ ] T043 [FR-012] [US4] Replace `ureq_get_brightid()` stub in src/identity/personhood.rs (line 103) with reqwest async GET to `{BRIGHTID_NODE_URL}/node/v6/verifications/WorldCompute/{contextId}`
- [ ] T044 [FR-012] [US4] Parse JSON response into existing `BrightIdVerification` struct, map to `PersonhoodResult` enum

### OAuth2

- [ ] T045 [P] [FR-013] [US4] Implement `OAuth2ProviderConfig` struct in src/identity/oauth2.rs: load client_id, client_secret, auth_url, token_url, redirect_uri, scopes from environment variables
- [ ] T046 [FR-013] [US4] Implement authorization code flow in src/identity/oauth2.rs `verify_oauth2()` (line 27): generate auth URL → exchange code for token → fetch user profile → return OAuth2Result::Verified

### Phone/SMS

- [ ] T047 [P] [FR-014] [US4] Implement `SmsProviderConfig` struct in src/identity/phone.rs: load account_sid, auth_token, verify_service_sid from environment variables
- [ ] T048 [FR-014] [US4] Implement `send_verification_code()` in src/identity/phone.rs (line 18): POST to Twilio Verify API to send SMS code
- [ ] T049 [FR-014] [US4] Implement `verify_code()` in src/identity/phone.rs (line 25): POST to Twilio Verify API to check code, return PhoneResult

### Credential Error Handling

- [ ] T050 [US4] Add credential expiry/error handling across all providers: fail current operation with clear error message, no hot-reload (per clarification)

### Integration

- [ ] T051 [US4] Add integration test for BrightID verification using sandbox/test node in tests/identity/
- [ ] T052 [US4] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-004 satisfied — at least one identity path completes end-to-end.

---

## Phase 6: User Story 5 — Transparency Logging (Priority: P3)

**Goal**: Policy decisions and artifact signatures are recorded in Sigstore Rekor.

**Independent Test**: Submit a log entry to Rekor staging and verify retrieval.

- [ ] T053 [FR-015] [US5] Implement Rekor submission in src/registry/transparency.rs (line 60): POST hashedrekord entry to Rekor REST API via reqwest, parse response for log index, UUID, inclusion proof
- [ ] T054 [FR-015] [US5] Replace fake entry ID generation in src/ledger/transparency.rs (line 28): use real Rekor entry UUID instead of `stub-rekor-{hex_prefix}`
- [ ] T055 [FR-015] [US5] Implement real verification in src/ledger/transparency.rs `verify()` (line 51): check inclusion proof against Rekor signed tree head instead of always returning Ok(true)
- [ ] T056 [US5] Add integration test: submit entry to Rekor public staging, verify retrieval in tests/infrastructure/
- [ ] T057 [US5] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-005 satisfied — transparency entries are retrievable with verifiable timestamps.

---

## Phase 7: User Story 6 — Observability (Priority: P3)

**Goal**: Traces and metrics exported to configured OTLP endpoint.

**Independent Test**: Configure OTLP endpoint and verify telemetry appears within 30 seconds.

- [ ] T058 [FR-016] [US6] Implement OTLP exporter wiring in src/telemetry/mod.rs (line 20): when `otel_endpoint` is Some, create OTLP trace exporter via `opentelemetry_otlp::new_exporter().tonic()`, add batch span processor, connect tracing-opentelemetry layer
- [ ] T059 [FR-016] [US6] Implement OtlpConfig struct in src/telemetry/mod.rs: endpoint, service_name, batch_size, export_interval_secs with defaults
- [ ] T060 [US6] Add integration test: start with OTLP endpoint, verify traces arrive within 30 seconds in tests/infrastructure/
- [ ] T061 [US6] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-006 satisfied — telemetry data appears at OTLP endpoint within 30 seconds.

---

## Phase 8: User Story 7 — Raft Consensus (Priority: P3)

**Goal**: Multi-coordinator cluster with leader election and log replication.

**Independent Test**: Start 3 coordinators, verify leader election and single-node failure survival.

- [ ] T062 [FR-017] [US7] Implement `RaftCoordinatorStorage` in src/scheduler/coordinator.rs: implement openraft `RaftStorage` trait with in-memory log + optional WAL
- [ ] T063 [FR-017] [US7] Implement Raft network adapter in src/scheduler/coordinator.rs: implement openraft `RaftNetworkFactory` trait using libp2p gossipsub for RPC transport
- [ ] T064 [FR-017] [US7] Wire `Raft::new()` into coordinator startup, replacing stub `start_election()` (line 55) and `become_leader()` (line 64)
- [ ] T065 [US7] Add integration test: 3-node cluster leader election + single-node failure recovery in tests/infrastructure/
- [ ] T066 [US7] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-007 satisfied — 3-node cluster survives single-node failure.

---

## Phase 9: User Story 8 — Network Discovery (Priority: P3)

**Goal**: NAT detection and DNS seed bootstrap.

**Independent Test**: Run NAT detection against STUN server; resolve DNS seeds.

- [ ] T067 [P] [FR-018] [US8] Implement STUN-based NAT detection in src/network/nat.rs (line 35): send STUN binding request to public servers (Google, Cloudflare), classify NAT type from response
- [ ] T068 [P] [FR-019] [US8] Replace placeholder DNS seed addresses in src/network/discovery.rs (line 63) with configurable seed list (env var or config file, placeholder as fallback)
- [ ] T069 [US8] Add integration test: NAT detection against real STUN server in tests/network/
- [ ] T070 [US8] Add integration test: DNS seed resolution returns valid multiaddrs in tests/network/
- [ ] T071 [US8] Run `cargo test` to verify zero regressions

**Checkpoint**: SC-008 and SC-009 satisfied — NAT types detected, DNS seeds resolved.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all stories

- [ ] T072 [P] Run full regression: `cargo test` — all existing tests must pass (SC-010)
- [ ] T073 [P] Run full clippy: `cargo clippy --lib -- -D warnings` — zero warnings
- [ ] T074 Verify no "not yet implemented" strings remain: grep for "not yet implemented" across src/
- [ ] T075 [P] Update CLAUDE.md test count and stub count to reflect current state
- [ ] T076 Run quickstart.md validation: execute each command from specs/003-stub-replacement/quickstart.md
- [ ] T077 Verify SC-002: end-to-end WASM workload completes in under 60 seconds

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational CLI (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **User Stories (Phases 3–9)**: All depend on Foundational CLI completion
  - US2 (Sandbox) and US3 (Attestation) and US4 (Identity) can proceed in parallel
  - US5 (Rekor), US6 (OTLP), US7 (Raft), US8 (Network) can proceed in parallel
- **Polish (Phase 10)**: Depends on all user stories being complete

### User Story Dependencies

- **US2 (Sandbox)**: Can start after Phase 2 — no dependency on other stories
- **US3 (Attestation)**: Can start after Phase 2 — no dependency on other stories
- **US4 (Identity)**: Can start after Phase 2 — no dependency on other stories
- **US5 (Rekor)**: Can start after Phase 2 — no dependency on other stories
- **US6 (OTLP)**: Can start after Phase 2 — no dependency on other stories
- **US7 (Raft)**: Can start after Phase 2 — no dependency on other stories
- **US8 (Network)**: Can start after Phase 2 — no dependency on other stories

### Within Each User Story

- Models/structs before service logic
- Service logic before integration wiring
- Integration wiring before tests
- Verify `cargo test` passes after each story

### Parallel Opportunities

- T005–T009: All CLI wiring tasks touch different files — fully parallel
- T020, T025: Firecracker and Apple VF helper are independent — parallel
- T032, T033: Ed25519 and CertificateChainValidator are independent — parallel
- T035, T036: SEV-SNP and TDX validators are independent — parallel
- T043, T045, T047: BrightID, OAuth2 config, SMS config are independent — parallel
- T067, T068: NAT and DNS are independent — parallel
- All user stories (Phases 3–9) can run in parallel if staffed

---

## Parallel Example: Phase 3 (Sandbox)

```bash
# WASM tasks are sequential (each builds on prior):
T016 → T017 → T018 → T019

# Firecracker and Apple VF can proceed in parallel with WASM:
T020 → T021 → T022 → T023 → T024  (Firecracker track)
T025 → T026 → T027 → T028          (Apple VF track)

# Integration tests after all three tracks:
T029, T030 (parallel), then T031
```

---

## Implementation Strategy

### MVP First (CLI + WASM Sandbox)

1. Complete Phase 1: Setup (T001–T004)
2. Complete Phase 2: CLI Wiring (T005–T015)
3. Complete WASM tasks only from Phase 3 (T016–T019, T029, T031)
4. **STOP and VALIDATE**: End-to-end WASM workload via CLI in under 60 seconds
5. This delivers a working system on all platforms

### Incremental Delivery

1. Setup + CLI → Foundation ready
2. WASM sandbox → MVP (any platform can run workloads)
3. Firecracker / Apple VF → Platform-native performance
4. Attestation + Identity → Production security
5. Rekor + OTLP + Raft + Network → Operational maturity

### Parallel Team Strategy

With multiple developers after Phase 2 completes:

- Developer A: Sandbox (Phase 3) — WASM first, then Firecracker
- Developer B: Attestation (Phase 4) — Ed25519, then chain validators
- Developer C: Identity (Phase 5) — BrightID, then OAuth2/phone
- Developer D: Infrastructure (Phases 6–8) — Rekor, OTLP, Raft
- Developer E: Network (Phase 9) — NAT, DNS seeds

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Constitution Principle V: each phase must include integration tests on real resources
