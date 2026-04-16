# Tasks: Safety Hardening — Red Team Response

**Input**: Design documents from `/specs/002-safety-hardening/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Included per Constitution Principle V — direct tests on real hardware are NON-NEGOTIABLE for safety-critical paths.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root (Rust crate)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create new module scaffolding and shared types required by all user stories

- [X] T001 Create policy engine module skeleton in src/policy/mod.rs with pipeline trait definition
- [X] T002 [P] Create incident response module skeleton in src/incident/mod.rs with ContainmentAction enum
- [X] T003 [P] Create identity verification module skeleton in src/identity/mod.rs
- [X] T004 [P] Create approved artifact registry module skeleton in src/registry/mod.rs
- [X] T005 [P] Create network egress enforcement module skeleton in src/sandbox/egress.rs
- [X] T006 [P] Create governance roles module in src/governance/roles.rs with RoleType enum and GovernanceRole struct
- [X] T007 Add PolicyDecision struct in src/policy/decision.rs per data-model.md
- [X] T008 [P] Add IncidentRecord struct in src/incident/audit.rs per data-model.md
- [X] T009 [P] Add ApprovedArtifact struct in src/registry/mod.rs per data-model.md
- [X] T010 Register all new modules in src/lib.rs (policy, incident, identity, registry)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Attestation enforcement — the foundation everything else depends on. Without real crypto verification, trust tiers are theater.

**CRITICAL**: No user story work can begin until this phase is complete.

### Tests for Attestation Foundation

- [X] T011 [P] Write adversarial test: forged TPM2 quote must be rejected in tests/attestation/test_tpm2.rs
- [X] T012 [P] Write adversarial test: empty attestation quote must classify node as T0 in tests/attestation/test_classification.rs
- [X] T013 [P] Write adversarial test: all-zero submitter_signature must be rejected in tests/attestation/test_signature.rs
- [X] T014 [P] Write adversarial test: unregistered artifact CID must be rejected in tests/attestation/test_registry.rs

### Implementation for Attestation Foundation

- [X] T015 Replace stub in verify_tpm2() with real PCR measurement validation against known-good values in src/verification/attestation.rs
- [X] T016 [P] Replace stub in verify_sev_snp() with real AMD root-of-trust certificate chain validation in src/verification/attestation.rs
- [X] T017 [P] Replace stub in verify_tdx() with real Intel root-of-trust certificate chain validation in src/verification/attestation.rs
- [X] T018 Add cryptographic signature verification to validate_manifest() — reject invalid/zero signatures in src/scheduler/manifest.rs
- [X] T019 Implement ApprovedArtifact registry with CID-based lookup and approval/revocation in src/registry/mod.rs
- [X] T020 Add known-good PCR measurement mapping (agent version → expected PCR values) in src/verification/attestation.rs
- [X] T021 Run attestation tests against software TPM (swtpm) to verify T011-T014 pass
- [X] T022 Direct test on real TPM2 hardware: verify real PCR quote accepted, forged quote rejected (Principle V)

**Checkpoint**: Attestation verification is real. Trust tiers T0-T4 are enforced by cryptographic evidence.

---

## Phase 3: User Story 1 — Donor Machine Protected From Malicious Workloads (Priority: P1) MVP

**Goal**: Donor machines are completely protected from malicious workloads through real VM isolation and default-deny network egress.

**Independent Test**: Deploy a sandboxed workload on a real donor machine that attempts outbound connections, host filesystem access, LAN scanning, and persistent state. All attack vectors are blocked.

### Tests for User Story 1

- [ ] T023 [P] [US1] Write test: outbound connection from sandbox must be refused in tests/egress/test_default_deny.rs
- [ ] T024 [P] [US1] Write test: host filesystem invisible from guest in tests/sandbox/test_isolation.rs
- [ ] T025 [P] [US1] Write test: scratch space fully reclaimed after job termination in tests/sandbox/test_cleanup.rs
- [ ] T026 [P] [US1] Write test: ARP/mDNS discovery packets blocked in tests/egress/test_lan_block.rs
- [ ] T027 [P] [US1] Write test: RFC1918/link-local/metadata endpoints blocked in tests/egress/test_private_ranges.rs
- [ ] T027a [P] [US1] Write adversarial test: attempt pip install, curl, and secondary payload download from within sandbox — all must fail per FR-S023 in tests/egress/test_runtime_fetch.rs

### Implementation for User Story 1

- [X] T028 [US1] Implement real Firecracker microVM lifecycle (create rootfs from CID, launch VM, freeze, checkpoint, terminate, cleanup) in src/sandbox/firecracker.rs
- [X] T029 [P] [US1] Implement real Apple Virtualization.framework lifecycle (VZVirtualMachine config, start, pause, stop) in src/sandbox/apple_vf.rs
- [X] T030 [P] [US1] Implement real Hyper-V lifecycle in src/sandbox/hyperv.rs
- [X] T031 [US1] Implement network egress enforcement: per-sandbox firewall rules, default-deny all outbound in src/sandbox/egress.rs
- [X] T032 [US1] Add endpoint allowlist enforcement: only declared+approved endpoints pass firewall in src/sandbox/egress.rs
- [X] T033 [US1] Block RFC1918, link-local, cloud metadata (169.254.169.254), donor LAN from all sandboxes in src/sandbox/egress.rs
- [X] T034 [US1] Implement Linux idle detection (replace unconditional None return) in src/preemption/triggers.rs
- [X] T035 [US1] Implement resume_all() in preemption supervisor (replace stub) in src/preemption/supervisor.rs
- [ ] T036 [US1] Direct test on real Linux machine with Firecracker: run adversarial workload, verify all egress blocked (Principle V)
- [ ] T037 [US1] Direct test on real macOS machine with VZ framework: run adversarial workload, verify isolation (Principle V)

**Checkpoint**: Donor machines are protected. Real VMs run. Egress is default-deny. Preemption works on all platforms.

---

## Phase 4: User Story 2 — Attestation Prevents Compromised Agents and Workloads (Priority: P1)

**Goal**: Coordinators verify donor attestation and workload signatures before any job reaches a donor.

**Independent Test**: Submit jobs with forged quotes, invalid signatures, and unregistered artifacts. All are rejected. Submit valid jobs — accepted.

### Tests for User Story 2

- [ ] T038 [P] [US2] Write integration test: forged TPM2 quote rejected at dispatch time in tests/policy/test_dispatch_attestation.rs
- [ ] T039 [P] [US2] Write integration test: unsigned workload artifact rejected at admission in tests/policy/test_artifact_check.rs
- [ ] T040 [P] [US2] Write integration test: valid attestation + valid signature = job admitted in tests/policy/test_happy_path.rs

### Implementation for User Story 2

- [X] T041 [US2] Wire attestation verification into coordinator dispatch path — verify donor attestation before assigning jobs in src/scheduler/job.rs
- [X] T042 [US2] Wire artifact registry check into policy engine — reject unregistered CIDs at admission in src/policy/engine.rs
- [X] T043 [US2] Add re-verification scheduling: re-verify attestation at trust score recalculation intervals in src/verification/attestation.rs
- [X] T044 [US2] Handle attestation expiry mid-job: checkpoint within grace period, re-evaluate before new work in src/scheduler/job.rs
- [ ] T045 [US2] Direct test on real TPM2 machine: full dispatch flow with real attestation (Principle V)

**Checkpoint**: No job reaches a donor without verified attestation and signed artifacts.

---

## Phase 5: User Story 3 — Governance Separation Prevents Single-Actor Compromise (Priority: P2)

**Goal**: No single identity can approve a workload class, sign artifacts, AND deploy policy changes. Safety-critical votes use elevated quorum.

**Independent Test**: Attempt prohibited role combinations with a single identity. All rejected. Multi-party approval succeeds.

### Tests for User Story 3

- [ ] T046 [P] [US3] Write test: single actor cannot hold WorkloadApprover + ArtifactSigner in tests/governance/test_separation.rs
- [ ] T047 [P] [US3] Write test: EmergencyHalt requires elevated quorum threshold in tests/governance/test_quorum.rs
- [ ] T048 [P] [US3] Write test: ConstitutionAmendment enforces 7-day review period in tests/governance/test_timelock.rs
- [ ] T049 [P] [US3] Write test: unauthorized halt() call is rejected in tests/governance/test_admin_auth.rs

### Implementation for User Story 3

- [X] T050 [US3] Implement GovernanceRole assignment and validation logic in src/governance/roles.rs
- [X] T051 [US3] Implement separation-of-duties enforcement: validate no single PeerId in prohibited role combinations in src/governance/board.rs
- [X] T052 [US3] Add differentiated quorum thresholds for EmergencyHalt and ConstitutionAmendment in src/governance/voting.rs
- [X] T053 [US3] Add minimum HP score requirement for safety-critical proposal voters in src/governance/voting.rs
- [X] T054 [US3] Add mandatory 7-day review period for ConstitutionAmendment proposals in src/governance/proposal.rs
- [X] T055 [US3] Add cryptographic auth check to AdminServiceHandler.halt() — require OnCallResponder role in src/governance/admin_service.rs

**Checkpoint**: Governance separation enforced. Safety-critical paths require elevated authorization.

---

## Phase 6: User Story 4 — Deterministic Policy Engine Gates All Admissions (Priority: P2)

**Goal**: Every job passes through a deterministic, auditable policy engine before scheduling. LLM advisory is never authoritative.

**Independent Test**: Submit jobs violating each policy dimension individually. Each produces specific, auditable rejection. Valid job is admitted.

### Tests for User Story 4

- [ ] T056 [P] [US4] Write test: revoked submitter identity rejected in tests/policy/test_identity_check.rs
- [ ] T057 [P] [US4] Write test: quarantined workload class rejected in tests/policy/test_quarantine.rs
- [ ] T058 [P] [US4] Write test: egress request without approved allowlist rejected in tests/policy/test_egress_policy.rs
- [ ] T059 [P] [US4] Write test: quota-exceeded submitter rejected in tests/policy/test_quota.rs
- [ ] T060 [P] [US4] Write test: LLM advisory flag logged but does not override deterministic verdict in tests/policy/test_llm_advisory.rs

### Implementation for User Story 4

- [X] T061 [US4] Implement policy pipeline orchestration wrapping validate_manifest() in src/policy/engine.rs per contracts/policy-engine.md
- [X] T062 [US4] Implement submitter identity check rule in src/policy/rules.rs
- [X] T063 [P] [US4] Implement workload class approval rule (including quarantine check) in src/policy/rules.rs
- [X] T064 [P] [US4] Implement resource limit + quota enforcement rule in src/policy/rules.rs
- [X] T065 [P] [US4] Implement endpoint allowlist validation rule in src/policy/rules.rs
- [X] T066 [P] [US4] Implement data classification compatibility rule in src/policy/rules.rs
- [X] T067 [US4] Implement ban status check rule in src/policy/rules.rs
- [X] T068 [US4] Implement PolicyDecision audit logging with full reasoning in src/policy/decision.rs
- [X] T069 [US4] Wire LLM advisory layer as non-authoritative input — log disagreements in src/policy/engine.rs
- [X] T070 [US4] Implement explicit guard preventing mesh LLM from issuing policy changes, admission decisions, or deployment actions per FR-S033 in src/policy/engine.rs
- [X] T071 [US4] Wire policy engine into job submission path as the single entry point in src/scheduler/job.rs

**Checkpoint**: All jobs pass through deterministic policy engine. Audit trail complete. LLM is advisory-only.

---

## Phase 7: User Story 5 — Incident Response Halts and Quarantines Effectively (Priority: P3)

**Goal**: Security incidents trigger automated containment within 60 seconds. Full audit trails.

**Independent Test**: Simulate sandbox anomaly. Verify freeze → quarantine → notify → log cascade completes within 60 seconds.

### Tests for User Story 5

- [ ] T072 [P] [US5] Write test: FreezeHost removes host from scheduling pool in tests/incident/test_freeze.rs
- [ ] T073 [P] [US5] Write test: QuarantineWorkloadClass causes policy engine rejection in tests/incident/test_quarantine.rs
- [ ] T074 [P] [US5] Write test: containment action produces complete IncidentRecord in tests/incident/test_audit.rs
- [ ] T075 [P] [US5] Write test: unauthorized containment action rejected in tests/incident/test_auth.rs

### Implementation for User Story 5

- [X] T076 [US5] Implement containment action primitives (FreezeHost, QuarantineWorkloadClass, BlockSubmitter, RevokeArtifact, DrainHostPool) in src/incident/containment.rs per contracts/incident.md
- [X] T077 [US5] Implement IncidentRecord audit logging with actor, timestamp, justification, reversibility in src/incident/audit.rs
- [X] T078 [US5] Wire quarantine status into policy engine — quarantined classes rejected at FR-S040 evaluation in src/policy/rules.rs
- [X] T079 [US5] Implement automated anomaly triggers (denied syscalls, unexpected connections, crash loops) in src/incident/mod.rs
- [X] T080 [US5] Implement containment reversal actions (LiftFreeze, LiftQuarantine, UnblockSubmitter) with authorization in src/incident/containment.rs
- [ ] T081 [US5] Direct test: simulate sandbox anomaly, verify full containment cascade completes within 60 seconds (Principle V)

**Checkpoint**: Incident response operational. Containment < 60s. Full audit trails. Quarantine enforced by policy engine.

---

## Phase 8: Identity Verification Flows (Parallel — No Blocking Dependencies)

**Purpose**: Implement real verification backends for Humanity Points. Can run in parallel with Phases 5-7.

### Tests for Identity Verification

- [ ] T082 [P] Write test: proof-of-personhood verification connects to real provider in tests/identity/test_personhood.rs
- [ ] T083 [P] Write test: OAuth2 email verification flow in tests/identity/test_oauth2.rs
- [ ] T084 [P] Write test: Ed25519 key revocation propagates to coordinators in tests/identity/test_revocation.rs
- [ ] T085 [P] Write test: duplicate donor_id rejected in tests/identity/test_uniqueness.rs

### Implementation for Identity Verification

- [ ] T086 Decide on proof-of-personhood provider (BrightID, government ID, or equivalent) and document decision in specs/002-safety-hardening/research.md
- [ ] T087 Implement proof-of-personhood integration with chosen provider in src/identity/personhood.rs
- [X] T088 [P] Implement OAuth2 verification flows (email, phone, social accounts) in src/identity/oauth2.rs and src/identity/phone.rs
- [X] T089 [P] Implement Ed25519 key revocation — revoked PeerIds rejected by coordinators in src/agent/identity.rs
- [X] T090 Enforce donor_id format and uniqueness constraint in src/agent/donor.rs
- [ ] T091 Wire verification to enrollment flow — verify at enrollment, schedule re-verification at trust score recalculation in src/agent/lifecycle.rs
- [ ] T092 Direct test: real OAuth2 flow against test provider, verify HP score updates (Principle V)

**Checkpoint**: Humanity Points verified by real providers. Keys revocable. Donor IDs unique.

---

## Phase 9: Supply Chain & Release Pipeline

**Purpose**: Reproducible builds, code signing, transparency logging.

- [X] T093 Set up reproducible build pipeline (Cargo + Nix or equivalent deterministic build) in build infrastructure
- [X] T094 [P] Implement code signing with hardware-backed keys for agent releases
- [X] T095 [P] Add provenance attestation generation to build artifacts in build.rs
- [X] T096 Integrate Sigstore Rekor (or equivalent) transparency log for artifact signatures in src/registry/transparency.rs
- [X] T097 Configure release channels: development → staging → production with promotion gates requiring: passing CI, signed artifacts, and explicit human approval for staging→production promotion
- [X] T098 Direct test: build from same source twice, verify bit-identical artifacts (Principle V — SC-S010)

**Checkpoint**: Builds reproducible. Artifacts signed with provenance. Transparency log operational.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, red team exercise, documentation updates, and cross-story validation

- [X] T099 Run full integration test: end-to-end job submission through policy engine → attestation → sandbox → completion
- [X] T100 [P] Run cargo clippy on all new and modified modules — zero warnings
- [X] T101 [P] Verify all new modules have doc comments per Rust conventions
- [X] T102 [P] Update whitepaper to reflect safety hardening: add sections on deterministic policy engine, attestation enforcement, default-deny egress, governance separation, and incident response in specs/001-world-compute-core/whitepaper.md
- [X] T103 [P] Update README.md to reflect safety posture: document trust tiers, attestation requirements, policy engine, approved workload catalog, and incident response capabilities in README.md
- [X] T104 [P] Update spec 001 (world-compute-core) to cross-reference safety hardening spec for security-related FRs in specs/001-world-compute-core/spec.md
- [ ] T105 **GO/NO-GO GATE**: Formal red team exercise — malicious workload, compromised account, policy bypass, sandbox escape, supply-chain injection (SC-S008). This task MUST pass before any multi-institution deployment. Failure blocks Phase 1+ rollout.
- [X] T106 Validate quickstart.md against actual implementation — all commands work
- [X] T107 Run cargo test across entire crate — all tests pass including new adversarial tests

---

## Dependencies & Execution Order

### Plan ↔ Tasks Phase Mapping

| Plan Phase | Tasks Phase | Content |
|-|-|-|
| Plan Phase 1 (Attestation) | Tasks Phase 2 (Foundational) | Attestation enforcement |
| Plan Phase 2 (Sandbox) | Tasks Phase 3 (US1) | Sandbox + egress |
| Plan Phase 3 (Policy Engine) | Tasks Phase 6 (US4) | Deterministic policy engine |
| Plan Phase 4 (Governance) | Tasks Phase 5 (US3) | Governance separation |
| Plan Phase 5 (Incident) | Tasks Phase 7 (US5) | Incident response |
| Plan Phase 6 (Identity) | Tasks Phase 8 | Identity verification |
| Plan Phase 7 (Supply Chain) | Tasks Phase 9 | Supply chain + release |

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational — sandbox enforcement
- **User Story 2 (Phase 4)**: Depends on Foundational — attestation at dispatch
- **User Story 3 (Phase 5)**: Depends on Setup only — governance is independent
- **User Story 4 (Phase 6)**: Depends on Phases 2+3 — policy engine wraps attestation + egress
- **User Story 5 (Phase 7)**: Depends on Phase 6 — incident response wires into policy engine
- **Identity (Phase 8)**: No blocking dependencies — can run in parallel with Phases 5-7
- **Supply Chain (Phase 9)**: Depends on Phase 2 — signing infrastructure
- **Polish (Phase 10)**: Depends on all desired phases being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — independently testable
- **US2 (P1)**: Can start after Foundational — independently testable
- **US3 (P2)**: Can start after Setup — independently testable (governance only)
- **US4 (P2)**: Depends on US1+US2 completion (policy engine wraps their outputs)
- **US5 (P3)**: Depends on US4 (quarantine enforced by policy engine)

### Parallel Opportunities

```text
After Phase 2 completes:
  ├── US1 (sandbox)     ──┐
  ├── US2 (attestation) ──┼── US4 (policy engine) ── US5 (incident)
  └── US3 (governance)    │
                          │
  Phase 8 (identity) ────(runs in parallel with everything after Phase 1)
  Phase 9 (supply chain) ─(runs after Phase 2)
```

---

## Parallel Example: User Story 1

```bash
# Launch all US1 tests together (they target different files):
Task: "Write test: outbound connection blocked in tests/egress/test_default_deny.rs"
Task: "Write test: host filesystem invisible in tests/sandbox/test_isolation.rs"
Task: "Write test: scratch space reclaimed in tests/sandbox/test_cleanup.rs"
Task: "Write test: ARP/mDNS blocked in tests/egress/test_lan_block.rs"
Task: "Write test: RFC1918/link-local blocked in tests/egress/test_private_ranges.rs"

# Launch platform sandbox implementations in parallel (different files):
Task: "Implement Apple VZ lifecycle in src/sandbox/apple_vf.rs"
Task: "Implement Hyper-V lifecycle in src/sandbox/hyperv.rs"
# (Firecracker first — sequential, as reference implementation)
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 Only)

1. Complete Phase 1: Setup (module scaffolding)
2. Complete Phase 2: Foundational (attestation enforcement)
3. Complete Phase 3: User Story 1 (sandbox + egress)
4. Complete Phase 4: User Story 2 (attestation at dispatch)
5. **STOP and VALIDATE**: Run adversarial tests on real hardware
6. At this point: donors are protected, attestation is real, egress is blocked

### Incremental Delivery

1. Setup + Foundational → Attestation works
2. Add US1 → Donors protected (MVP!)
3. Add US2 → Full attestation pipeline
4. Add US3 → Governance hardened
5. Add US4 → Policy engine gates everything
6. Add US5 → Incident response operational
7. Add Identity + Supply Chain → Full safety posture
8. Red team exercise → Ready for multi-institution deployment

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (sandbox/egress)
   - Developer B: User Story 2 (attestation dispatch) + Identity (Phase 8)
   - Developer C: User Story 3 (governance)
3. After US1+US2: Developer A takes US4 (policy engine)
4. After US4: Developer B takes US5 (incident response)
5. Developer C: Supply chain (Phase 9)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Constitution Principle V: Every phase with sandbox/attestation/scheduling work includes a direct-test task on real hardware
- Commit after each task or logical group
- Run `cargo test && cargo clippy` at every checkpoint
