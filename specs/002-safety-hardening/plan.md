# Implementation Plan: Safety Hardening — Red Team Response

**Branch**: `002-safety-hardening` | **Date**: 2026-04-16 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-safety-hardening/spec.md`

## Summary

Harden World Compute's safety posture by closing enforcement gaps identified
through independent evaluation of the red team review (issue #4). The core
work is: (1) replace attestation stubs with real cryptographic verification,
(2) enforce default-deny network egress at the sandbox level,
(3) implement a deterministic policy engine wrapping `validate_manifest()`,
(4) add governance separation of duties and differentiated quorum thresholds,
(5) build incident response containment machinery, and (6) implement
identity verification flows for Humanity Points. All work preserves the
project's constitutional identity as an open volunteer compute federation.

## Technical Context

**Language/Version**: Rust (latest stable, currently 1.82+), per FR-006
**Primary Dependencies**: rust-libp2p (P2P networking), ed25519-dalek
(cryptography), wasmtime (WASM sandbox), serde (serialization),
tonic/prost (gRPC), tokio (async runtime)
**Storage**: Content-addressed CID store (in-tree), CRDT-based ledger
**Testing**: `cargo test` + `cargo clippy`; direct testing on real
hardware per Constitution Principle V
**Target Platform**: Linux (Firecracker/KVM), macOS (Virtualization.framework),
Windows (Hyper-V), Browser/Mobile (WASM fallback)
**Project Type**: Distributed system daemon + CLI + GUI (Tauri)
**Performance Goals**: Policy engine decision latency < 100ms p95;
containment action completion < 60 seconds (SC-S006)
**Constraints**: Zero stub-acceptance paths in production attestation
(SC-S003); zero sandbox escape vulnerabilities survive to production
(SC-S001); default-deny egress enforced for 100% of non-allowlisted
jobs (SC-S004)
**Scale/Scope**: Initial deployment targeting tens of nodes (Phase 0),
scaling to thousands (Phase 2+). Policy engine must handle burst
submission rates proportional to node count.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Safety First (Sandboxing & Host Integrity)

| Requirement | Plan compliance | Notes |
|-|-|-|
| All workloads in hypervisor/VM sandbox | Compliant | FR-S001 closes stub gaps in all VM drivers |
| No path to host kernel/filesystem/network | Compliant | FR-S003 enforces filesystem isolation; FR-S002/S020-S022 enforce network isolation |
| Cryptographic attestation of agent/workload images | Compliant | FR-S010-S013 replace stub verification with real crypto |
| Agent reproducibly built and code-signed | Compliant | FR-S050 mandates reproducible builds with hardware-backed signing |
| Sandbox escape = P0 incident | Compliant | FR-S060-S062 add incident response machinery |

### Principle II: Robustness & Graceful Degradation

| Requirement | Plan compliance | Notes |
|-|-|-|
| Declarative workload specs | Compliant | FR-S040 wraps existing manifest validation in policy pipeline |
| Self-healing scheduling | Not in scope | Existing scheduler handles this; not modified by this spec |
| Failure modes explicit | Compliant | Edge cases document attestation expiry, governance quorum failure |

### Principle III: Fairness & Donor Sovereignty

| Requirement | Plan compliance | Notes |
|-|-|-|
| Donor machine safety absolute priority | Compliant | FR-S001-S004 enforce real VM isolation and egress blocking |
| Sub-second yield to local user | Not in scope | Preemption supervisor architecture exists; not modified by this spec |
| Donors as first-class citizens | Compliant | Spec explicitly rejects review's recommendation to exclude personal hardware |

### Principle IV: Efficiency, Performance & Self-Improvement

| Requirement | Plan compliance | Notes |
|-|-|-|
| Efficient use of resources | Compliant | Policy engine adds < 100ms overhead per submission |
| Performance regressions block release | Compliant | Policy engine latency is a measurable success criterion |

### Principle V: Direct Testing (NON-NEGOTIABLE)

| Requirement | Plan compliance | Notes |
|-|-|-|
| End-to-end on real hardware | Compliant | All user stories specify real-hardware testing (real TPM2, real VM, real network) |
| Adversarial test cases for safety paths | Compliant | US1 tests malicious workloads; US2 tests forged attestation; SC-S008 requires formal red team exercise |
| Direct-test evidence artifact per release | Compliant | SC-S008 blocks multi-institution deployment until red team passes |

**GATE RESULT: PASS** — No violations. All five principles are satisfied.

## Project Structure

### Documentation (this feature)

```text
specs/002-safety-hardening/
├── plan.md              # This file
├── research.md          # Phase 0: research findings
├── data-model.md        # Phase 1: entity definitions
├── quickstart.md        # Phase 1: developer getting-started
├── contracts/           # Phase 1: interface contracts
│   ├── policy-engine.md # Policy engine evaluation contract
│   ├── attestation.md   # Attestation verification contract
│   └── incident.md      # Incident response action contract
└── checklists/
    └── requirements.md  # Spec quality checklist
```

### Source Code (repository root)

```text
src/
├── policy/                    # NEW: Deterministic policy engine
│   ├── mod.rs                 # Policy pipeline orchestration
│   ├── engine.rs              # Core evaluation logic wrapping validate_manifest()
│   ├── rules.rs               # Individual policy rules (identity, quota, allowlist, ban)
│   └── decision.rs            # PolicyDecision audit record
├── verification/
│   ├── attestation.rs         # MODIFY: Replace stubs with real TPM2/SEV-SNP/TDX verification
│   ├── quorum.rs              # Existing (no change)
│   └── trust_score.rs         # Existing (no change)
├── sandbox/
│   ├── mod.rs                 # Existing trait (no change)
│   ├── firecracker.rs         # MODIFY: Implement real Firecracker VM lifecycle
│   ├── apple_vf.rs            # MODIFY: Implement real VZ framework lifecycle
│   ├── hyperv.rs              # MODIFY: Implement real Hyper-V lifecycle
│   ├── wasm.rs                # Existing (no change)
│   ├── gpu.rs                 # Existing (no change)
│   └── egress.rs              # NEW: Network egress enforcement (firewall rules per sandbox)
├── governance/
│   ├── mod.rs                 # Existing
│   ├── voting.rs              # MODIFY: Add differentiated quorum thresholds
│   ├── board.rs               # MODIFY: Add separation-of-duties enforcement
│   ├── roles.rs               # NEW: GovernanceRole entity and role assignment
│   ├── proposal.rs            # MODIFY: Add time-lock for ConstitutionAmendment
│   ├── admin_service.rs       # MODIFY: Add auth check to halt()
│   └── ...                    # Other existing files unchanged
├── incident/                  # NEW: Incident response machinery
│   ├── mod.rs                 # Incident handler orchestration
│   ├── containment.rs         # Freeze, quarantine, block, revoke actions
│   └── audit.rs               # IncidentRecord logging
├── agent/
│   ├── identity.rs            # MODIFY: Add key revocation support
│   ├── donor.rs               # MODIFY: Enforce donor_id format/uniqueness
│   └── ...                    # Other existing files unchanged
├── identity/                  # NEW: Humanity Points verification flows
│   ├── mod.rs                 # Verification orchestration
│   ├── oauth2.rs              # OAuth2 flows (email, social)
│   ├── phone.rs               # Phone verification
│   └── personhood.rs          # Proof-of-personhood integration
├── registry/                  # NEW: Approved artifact registry
│   ├── mod.rs                 # Registry API
│   └── transparency.rs        # Sigstore/Rekor integration
├── scheduler/
│   ├── manifest.rs            # Existing validate_manifest() — preserved, wrapped by policy engine
│   └── ...                    # Other existing files unchanged
└── ...

tests/
├── policy/                    # Policy engine tests (unit + integration)
├── attestation/               # Real TPM2 + software TPM tests
├── egress/                    # Network egress enforcement tests
├── incident/                  # Incident response tests
├── governance/                # Separation of duties + quorum tests
└── identity/                  # HP verification flow tests
```

**Structure Decision**: Extends the existing `src/` module structure with
new modules (`policy/`, `incident/`, `identity/`, `registry/`) and
modifications to existing modules (`verification/`, `sandbox/`,
`governance/`, `agent/`). No new top-level projects or workspaces needed.

## Complexity Tracking

No constitution violations to justify. All work fits within the existing
single-crate structure.

## Implementation Phases

### Phase 1: Attestation Enforcement (P1 — blocks everything)

**Rationale**: Without real attestation, the entire trust tier system is
theater. A node claiming T3 (SEV-SNP) without the hardware would be
accepted. This must be fixed first because the policy engine, governance,
and incident response all depend on trustworthy identity and attestation.

**Scope**: FR-S010, FR-S011, FR-S012, FR-S013

**Work**:
1. Implement real TPM2 PCR verification in `verify_tpm2()` — validate
   measurements against known-good values for the current signed agent build
2. Implement real SEV-SNP and TDX attestation report verification against
   AMD/Intel root-of-trust certificates
3. Add cryptographic signature verification to `validate_manifest()` —
   reject all-zero and invalid signatures
4. Create approved artifact registry with CID-based lookup
5. Write adversarial tests: forged quotes, expired certificates, invalid
   signatures, unregistered CIDs

**Dependencies**: None (foundational)
**Risk**: TPM2/SEV-SNP/TDX verification requires platform-specific
testing hardware. Mitigation: use software TPM (swtpm) for CI,
real hardware for Principle V direct tests.

### Phase 2: Sandbox Enforcement (P1 — blocks safe execution)

**Rationale**: The sandbox drivers are structurally correct but all
lifecycle methods are stubs. No real VM is ever launched. This must be
implemented before any workload runs on donor hardware.

**Scope**: FR-S001, FR-S002, FR-S003, FR-S004, FR-S020, FR-S021,
FR-S022, FR-S023

**Work**:
1. Implement real Firecracker microVM lifecycle (create rootfs from
   CID, launch VM, freeze via SIGSTOP, checkpoint via snapshot API,
   terminate, cleanup)
2. Implement real Apple Virtualization.framework lifecycle (VZVirtualMachine
   configuration, start, pause, stop)
3. Implement real Hyper-V lifecycle (via PowerShell/WMI or Hyper-V API)
4. Implement network egress enforcement module — configure per-sandbox
   firewall rules: default-deny all outbound, allowlist only declared
   endpoints from approved manifest
5. Block RFC1918, link-local, cloud metadata, donor LAN from all sandboxes
6. Implement Linux idle detection (`linux_idle_ms()`)
7. Write adversarial tests: outbound connection attempts, host filesystem
   probes, LAN scanning, runtime code fetch attempts

**Dependencies**: Phase 1 (attestation — needed to verify workload artifacts)
**Risk**: Platform-specific VM APIs have complex error surfaces. Mitigation:
implement Linux/Firecracker first (best documented), then macOS, then Windows.

### Phase 3: Deterministic Policy Engine (P2)

**Rationale**: The authoritative gate for all job admissions. Must exist
before any workload reaches a donor.

**Scope**: FR-S040, FR-S041, FR-S042

**Work**:
1. Create `src/policy/` module with pipeline architecture wrapping
   `validate_manifest()` as one step
2. Implement policy rules: submitter identity check, workload class
   approval, artifact registry lookup, resource limit validation,
   endpoint allowlist validation, data classification check, quota
   enforcement, ban status check
3. Implement `PolicyDecision` audit record with full reasoning
4. Wire LLM advisory layer as non-authoritative input — log
   disagreements when LLM flags a policy-approved job
5. Write tests: each policy dimension fails independently, full
   pipeline integration, LLM advisory disagreement logging

**Dependencies**: Phase 1 (attestation for identity verification),
Phase 2 (egress rules referenced by endpoint allowlist)

### Phase 4: Governance Hardening (P2)

**Rationale**: Prevents single-actor compromise of safety-critical
governance paths.

**Scope**: FR-S030, FR-S031, FR-S032, FR-S033

**Work**:
1. Add `GovernanceRole` entity and role assignment in `src/governance/roles.rs`
2. Implement differentiated quorum thresholds: `EmergencyHalt` and
   `ConstitutionAmendment` require elevated threshold + minimum HP score
3. Add mandatory 7-day review period for `ConstitutionAmendment` proposals
4. Add cryptographic auth check to `AdminServiceHandler.halt()` —
   require on-call responder role
5. Implement separation of duties: validate that no single identity
   appears in multiple prohibited role combinations within an approval flow
6. Write tests: single-actor violation detection, quorum threshold
   enforcement, time-lock bypass attempts, unauthorized halt attempts

**Dependencies**: Phase 1 (identity for role binding)

### Phase 5: Incident Response (P3)

**Rationale**: The system must be able to contain and respond to security
incidents before any multi-institution deployment.

**Scope**: FR-S060, FR-S061, FR-S062

**Work**:
1. Create `src/incident/` module with containment action primitives:
   freeze dispatch to host, quarantine workload class, block submitter,
   revoke artifact, drain host pool
2. Implement `IncidentRecord` audit logging with actor, timestamp,
   justification, reversibility status
3. Wire quarantine status into policy engine — quarantined classes
   rejected at FR-S040 evaluation
4. Write tests: containment action execution, audit trail completeness,
   quarantine persistence, authorized vs. unauthorized lift attempts

**Dependencies**: Phase 3 (policy engine for quarantine enforcement),
Phase 4 (governance roles for authorization)

### Phase 6: Identity Verification Flows (P2)

**Rationale**: Humanity Points are the Sybil-resistance mechanism but
currently tracked as booleans with no verification backend.

**Scope**: FR-S070, FR-S071, FR-S072, FR-S073

**Work**:
1. Create `src/identity/` module with verification orchestration
2. Implement OAuth2 flows for email, phone, social account verification
3. Integrate at least one proof-of-personhood provider (BrightID or
   equivalent)
4. Implement Ed25519 key revocation — revoked keys rejected by all
   coordinators
5. Enforce `donor_id` format and uniqueness constraint
6. Wire verification to enrollment flow — verify at enrollment, schedule
   re-verification at trust score recalculation intervals
7. Write tests: real OAuth2 flow (against test provider), key revocation
   propagation, duplicate donor_id rejection

**Dependencies**: None (can run in parallel with Phases 3-5)

### Phase 7: Supply Chain & Release Pipeline (P2)

**Rationale**: Constitutional requirement for reproducible, signed builds.
Required before any external deployment.

**Scope**: FR-S050, FR-S051, FR-S052, FR-S053

**Work**:
1. Set up reproducible build pipeline (Cargo + Nix or equivalent)
2. Implement code signing with hardware-backed keys
3. Add provenance attestation to build artifacts
4. Integrate Sigstore Rekor (or equivalent) transparency log
5. Configure release channels: development → staging → production
6. Write tests: build reproducibility verification, signature validation,
   provenance chain verification

**Dependencies**: Phase 1 (attestation infrastructure for signing)
**Risk**: Sigstore integration depends on external service availability.
Mitigation: implement with pluggable transparency log backend.

## Phase Dependencies

```text
Phase 1 (Attestation) ─────┬──── Phase 2 (Sandbox)
                            │
                            ├──── Phase 3 (Policy Engine) ──── Phase 5 (Incident Response)
                            │
                            ├──── Phase 4 (Governance)
                            │
                            └──── Phase 7 (Supply Chain)

Phase 6 (Identity) ──── runs in parallel, no blocking dependencies
```

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|-|-|-|-|
| TPM2/SEV-SNP hardware unavailable for testing | High | Medium | Use software TPM (swtpm) for CI; reserve real hardware for Principle V direct tests |
| Firecracker API changes between versions | Medium | Low | Pin Firecracker version; use integration tests to detect breaking changes |
| Sigstore/Rekor service unavailability | Medium | Low | Pluggable transparency log backend; local fallback for development |
| OAuth2 provider API changes | Low | Medium | Abstract behind verification interface; provider-specific adapters |
| Scope creep from deferred red team items | Medium | Medium | Explicit out-of-scope boundary in spec Assumptions section |
