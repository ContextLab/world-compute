# Research: Safety Hardening — Red Team Response

**Date**: 2026-04-16
**Method**: 5 parallel research agents independently evaluated red team
review claims against the actual codebase. Each agent read source files,
spec text, and constitutional requirements before reporting findings.

## Research Question 1: Identity and Authentication

**Decision**: Strengthen existing Humanity Points + Ed25519 + hardware
attestation model. Do NOT adopt institutional SSO (InCommon/eduGAIN).

**Rationale**: The red team claims the system relies on ".edu email" as
the main identity gate. This is false. The codebase uses Ed25519 keypair
generation (`src/agent/identity.rs`) → libp2p PeerId. Email appears only
in `src/governance/humanity_points.rs` as the lowest-weight signal (+1 HP
out of 24). The multi-tier Humanity Points system (email, phone, OAuth2
social, web-of-trust, proof-of-personhood, active donor status) is
architecturally appropriate for a globally inclusive volunteer network.
Institutional SSO would exclude non-academics, students without active
accounts, and participants in countries without InCommon members.

**Alternatives considered**:
- InCommon/eduGAIN federation: Rejected — incompatible with "anyone on
  Earth who opts in" constitutional mandate
- Decentralized identity (DID): Considered — may be useful future
  addition but adds complexity without clear immediate benefit
- Government ID verification: Included as one proof-of-personhood option

**Real gaps to close**:
- `proof_of_personhood: bool` has no verification backend
- OAuth2 flows are tracked as booleans, not implemented
- Ed25519 key revocation not supported
- `donor_id` has no enforced format or uniqueness

## Research Question 2: Sandboxing and Runtime Isolation

**Decision**: Implement real VM lifecycle in all sandbox drivers. Keep
personal laptops as first-class targets. Add network egress enforcement.

**Rationale**: The codebase already requires VM-level isolation per FR-010:
"all workloads MUST execute inside a hypervisor- or VM-level sandbox.
Process-only sandboxes are NOT sufficient." The `Sandbox` trait in
`src/sandbox/mod.rs` mandates create/start/freeze/checkpoint/terminate/cleanup.
However, ALL lifecycle methods across ALL drivers are stubs — no real VM
is ever launched. The red team's concern about isolation is valid for the
implementation gap, not the architecture gap.

The recommendation that personal laptops "should not be first-class
execution targets" directly contradicts Constitutional Principles II and III,
which explicitly name "the general public's laptops, phones, and home
servers" as the expected environment.

**Alternatives considered**:
- Exclude personal hardware: Rejected — constitutional conflict
- Container-only isolation: Rejected — FR-010 requires hypervisor/VM level
- WASM-only for all platforms: Rejected — insufficient isolation for T1+ tiers

**Real gaps to close**:
- All 5 critical methods in each VM driver are stubs (Firecracker,
  AppleVF, HyperV)
- Linux idle detection returns `None` unconditionally
- `resume_all()` in preemption supervisor is a stub
- No cryptographic attestation of workload images at dispatch time
- Network egress enforcement absent despite `network_egress_bytes` field

## Research Question 3: Network Egress and Workload Submission

**Decision**: Enforce default-deny egress at sandbox level. Keep CID-based
declarative manifest system. Add approved endpoint allowlisting.

**Rationale**: The manifest system is already declarative — jobs reference
a CID of an OCI image or WASM module, not inline code. The
`ResourceEnvelope` has `network_egress_bytes: u64` defaulting to 0 in all
test fixtures. However, this is a declarative budget with no runtime
enforcement. No firewall/namespace rules are configured by any sandbox
driver. The `submitter_signature` field exists but `validate_manifest()`
never verifies it — tests pass with all-zero signatures.

**Alternatives considered**:
- Curated-only catalog (no custom artifacts): Deferred — CID-based system
  already enables catalog enforcement; adding it is straightforward but
  not required for initial safety hardening
- Network proxy/gateway for all egress: Considered — useful for Phase 2+
  but adds infrastructure complexity; default-deny at sandbox level is
  sufficient for v1

**Real gaps to close**:
- `network_egress_bytes` has no sandbox-level enforcement
- `submitter_signature` is never cryptographically verified
- No endpoint allowlist mechanism in manifest or policy
- No blocking of RFC1918, link-local, cloud metadata endpoints

## Research Question 4: Governance and Mesh LLM Authority

**Decision**: Add differentiated quorum thresholds for safety-critical
proposals. Add separation of duties. Keep mesh LLM advisory-only (already
specified for Phases 0–2).

**Rationale**: The mesh LLM is NOT currently authoritative — FR-125
requires human governance approval for all modification tiers, and
Phases 0–2 restrict it to "read-only + suggest mode." The red team
overstates this risk. However, the governance code applies identical
voting rules to `EmergencyHalt` and `Compute` proposals — no elevated
quorum, no time-lock, no minimum HP requirement for safety-critical
votes. `AdminServiceHandler.halt()` has no authorization check. No
separation of duties exists anywhere in the codebase.

**Alternatives considered**:
- Remove all public voting from safety paths: Partially adopted — elevated
  quorum for safety-critical proposals, but standard voting preserved for
  non-safety governance (consistent with volunteer model)
- Formal review board only: Rejected for v1 — "governance group to be
  formally named at project start" is still provisional; implement
  technical controls (quorum thresholds, role separation) that work
  regardless of governance structure

**Real gaps to close**:
- `validate_vote()` applies identical rules to all proposal types
- `AdminServiceHandler.halt()` has no signature/role check
- No separation-of-duties enforcement anywhere
- No time-lock for `ConstitutionAmendment` proposals
- `ProposalBoard.cast_vote()` doesn't enforce minimum HP for safety votes

## Research Question 5: Supply Chain and Rollout

**Decision**: Implement attestation verification (priority), CI signing
pipeline, and Sigstore integration. Adopt phased rollout with go/no-go
criteria (not institution-specific gates).

**Rationale**: The constitution already mandates reproducibly built,
code-signed agents with cryptographic attestation. The spec names Sigstore
Rekor as the transparency log target. However, 3 of 5 attestation backends
(`verify_tpm2`, `verify_sev_snp`, `verify_tdx`) are stubs that accept any
non-empty quote. No CI pipeline for reproducible builds exists. No SBOM
generation. The red team's supply-chain concerns are real but largely
redundant with existing constitutional mandates — the gap is implementation,
not design.

The review's "single-institution Phase 0" framing subtly reorients trust
from volunteer-first to institution-first. The existing spec already has
phased rollout (centralized Phase 0 → full autonomous Phase 4). Go/no-go
criteria based on security evidence (SC-S008: red team exercise) are more
appropriate than institution-specific gates.

**Alternatives considered**:
- Full SLSA certification: Deferred — useful framework but formal
  certification is overhead for current stage; adopt practices without
  formal level tracking
- Institution-only Phase 0: Rejected — the constitution's trust anchors
  are cryptographic (attestation, quorum, signed ledger), not institutional
- Multiple release channels (dev/staging/canary/production): Adopted
  partially — dev/staging/production required; canary deferred until
  sufficient node count

**Real gaps to close**:
- `verify_tpm2`, `verify_sev_snp`, `verify_tdx` unconditionally accept
  any non-empty quote
- No CI pipeline for reproducible builds or code signing
- No Sigstore Rekor integration
- No SBOM generation
- No release channel enforcement
