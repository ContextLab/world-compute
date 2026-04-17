# Feature Specification: Safety Hardening — Red Team Response

**Feature Branch**: `002-safety-hardening`
**Created**: 2026-04-16
**Status**: Draft
**Input**: Address findings from red team review (issue #4) — minimize risk as non-negotiable first priority, maximize utility as second priority. Evaluate claims independently before implementing; take safety issues seriously but preserve the project's constitutional mission as a volunteer compute federation.

---

## Clarifications

### Session 2026-04-16

- Q: Should the new deterministic policy engine replace, wrap, or run
  separately from the existing `validate_manifest()` function? → A: The
  policy engine wraps `validate_manifest()` as one step in a larger
  pipeline — preserving existing structural validation while adding
  identity, quota, allowlist, and ban checks around it.
- Q: Should proof-of-personhood and Humanity Points verification happen
  at donor enrollment, on each job submission, or periodically? → A:
  Verify at enrollment time, re-verify periodically at trust score
  recalculation intervals. This minimizes per-submission latency while
  catching expired or revoked credentials.

---

## Overview

This specification addresses the findings from the red team security review
(GitHub issue #4) filed against the World Compute v1 design. The review
identifies genuine safety gaps in the current implementation while also
recommending architectural changes that would conflict with the project's
constitution.

This spec takes a **constitution-compatible safety hardening** approach:
adopt every safety recommendation that strengthens the system within its
stated mission, explicitly reject recommendations that would require
constitutional amendment, and document the reasoning for both.

### Guiding principle

> **Risk minimization is the first priority (non-negotiable). Utility
> maximization is second (can be accomplished at the cost of speed and
> convenience). The project's constitutional identity as an open volunteer
> compute federation is preserved.**

### Red team review summary

The review recommends converting World Compute into a "federated,
institution-backed, tightly constrained compute network." Independent
analysis of the codebase against these claims found:

| Domain | Review accuracy | Key finding |
|-|-|-|
| Identity | Mischaracterized | No ".edu email" gate exists; identity is Ed25519 + Humanity Points + hardware attestation tiers T0–T4 |
| Sandboxing | Redundant | VM-level isolation already required (FR-010); personal laptops are constitutional first-class targets |
| Egress/network | Partially valid | `network_egress_bytes` field exists but enforcement is absent; signature verification is stub-only |
| Governance | Partially valid | Mesh LLM is already advisory-only in current phases; but no separation of duties or differentiated quorum exists |
| Supply chain | Partially redundant | Constitution already mandates reproducible builds and code signing; attestation backends are non-functional stubs |

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Donor Machine Protected From Malicious Workloads (Priority: P1)

A volunteer donates their personal laptop to the cluster. A malicious actor
submits a workload that attempts to: (a) open outbound network connections
to exfiltrate data, (b) escape the VM sandbox to access host files,
(c) persist state after job completion, (d) probe the donor's LAN. The
donor's machine remains completely unaffected — all attack vectors are
blocked at the sandbox and network layers before the workload can act on
them.

**Why this priority**: This is Constitutional Principle I — the single
highest priority. A single breach would destroy public trust permanently.
The red team review correctly identifies that while VM isolation is
*specified*, the enforcement code is entirely stub-based. Closing this gap
is existential.

**Independent Test**: Deploy a sandboxed workload on a real donor machine
that attempts each attack vector. Verify: (a) all outbound connections
are blocked by default-deny egress, (b) no host filesystem access is
possible from within the VM, (c) after job termination, zero artifacts
remain on the host, (d) LAN scan attempts produce zero responses. Run
with both Firecracker (Linux) and Apple Virtualization.framework (macOS).

**Acceptance Scenarios**:

1. **Given** a donor running the agent on macOS, **When** a workload
   attempts `connect()` to any external IP, **Then** the connection is
   refused and the attempt is logged as a security event.
2. **Given** a donor running the agent on Linux with Firecracker, **When**
   a workload attempts to read `/etc/passwd` on the host, **Then** it sees
   only the guest filesystem — no host paths are visible.
3. **Given** a workload that writes 1 GB to its scratch space, **When** the
   job completes or is terminated, **Then** the scratch space is fully
   reclaimed and no files remain on the donor's disk.
4. **Given** a workload attempting ARP/mDNS discovery, **When** packets are
   sent, **Then** they never leave the VM's virtual network interface.

---

### User Story 2 - Attestation Prevents Compromised Agents and Workloads (Priority: P1)

A coordinator receives a job dispatch request targeting a donor node. The
coordinator verifies: (a) the donor's agent is a signed, reproducible build
with valid hardware attestation, (b) the workload artifact has a valid
cryptographic signature and approved provenance, (c) the job manifest
passes deterministic policy checks. If any verification fails, the job is
rejected before it reaches the donor.

**Why this priority**: The red team correctly identifies that attestation
backends (`verify_tpm2`, `verify_sev_snp`, `verify_tdx`) unconditionally
accept any non-empty quote. This means the trust tier system (T0–T4) is
structurally present but not enforced — a node claiming T3 (SEV-SNP)
status without actual hardware would be accepted.

**Independent Test**: Submit a job with: (a) a forged TPM2 quote — verify
rejection, (b) a valid TPM2 quote — verify acceptance, (c) an unsigned
workload artifact — verify rejection, (d) a properly signed artifact —
verify acceptance. Run against a real TPM2-equipped machine and a software
TPM for comparison.

**Acceptance Scenarios**:

1. **Given** a donor node presenting an empty attestation quote, **When**
   the coordinator evaluates trust tier, **Then** the node is classified
   as T0 (lowest tier) regardless of claimed hardware.
2. **Given** a donor node with a valid TPM2 quote and PCR measurements
   matching the current signed agent build, **When** the coordinator
   evaluates, **Then** the node is classified as T1 or T2 as appropriate.
3. **Given** a job manifest with `submitter_signature` of all zeros,
   **When** `validate_manifest` runs, **Then** the manifest is rejected.
4. **Given** a workload artifact CID that does not match any approved
   artifact in the registry, **When** admission policy runs, **Then**
   the job is rejected with a clear error.

---

### User Story 3 - Governance Separation Prevents Single-Actor Compromise (Priority: P2)

An operator attempts to simultaneously approve a new workload class, sign
the artifact, and deploy a policy change relaxing egress rules. The system
enforces separation of duties: no single identity can perform all three
actions. Safety-critical governance actions (emergency halt, constitution
amendments, admission policy relaxation) require elevated quorum thresholds
and cannot be decided by standard quadratic voting alone.

**Why this priority**: The red team correctly identifies that the current
governance code applies identical voting rules to `EmergencyHalt` and
`Compute` proposals. The `AdminServiceHandler.halt()` has no authorization
check. These are real gaps.

**Independent Test**: Attempt each prohibited combination with a single
operator identity. Verify all are rejected. Then verify that a properly
constituted multi-party approval flow succeeds.

**Acceptance Scenarios**:

1. **Given** a single operator, **When** they attempt to approve a workload
   class AND sign the artifact, **Then** the second action is rejected
   with "separation of duties violation."
2. **Given** an `EmergencyHalt` proposal, **When** it is submitted for
   voting, **Then** the system requires a higher quorum threshold than
   standard proposals AND a minimum Humanity Points score for voters.
3. **Given** a `ConstitutionAmendment` proposal, **When** voting opens,
   **Then** a mandatory 7-day review period is enforced before votes are
   tallied.
4. **Given** `AdminServiceHandler.halt()` is called, **When** the caller
   lacks the designated on-call responder role, **Then** the call is
   rejected.

---

### User Story 4 - Deterministic Policy Engine Gates All Admissions (Priority: P2)

A job is submitted to the cluster. Before any scheduling occurs, a
deterministic policy engine evaluates: submitter identity validity,
workload class approval, artifact digest approval, resource limits,
endpoint allowlists, data classification compatibility, quota compliance,
and policy ban status. The engine produces an auditable accept/reject
decision with full reasoning. LLM-based review is available as an advisory
layer but never overrides or substitutes for the deterministic gate.

**Why this priority**: The red team correctly notes that LLM-based review
should be advisory-only. The current design already specifies this for
phases 0–2, but no deterministic policy engine exists in code.

**Independent Test**: Submit jobs that violate each policy dimension
individually. Verify each produces a specific, auditable rejection. Then
submit a job that passes all checks and verify it is admitted.

**Acceptance Scenarios**:

1. **Given** a submitter with expired or revoked identity, **When** they
   submit a job, **Then** the policy engine rejects it before any LLM
   review occurs.
2. **Given** a valid job that the LLM advisory layer flags as suspicious,
   **When** the deterministic policy engine approves it, **Then** the job
   is admitted (with the LLM flag logged for human review).
3. **Given** a job requesting `network_egress_bytes > 0` without an
   approved endpoint allowlist, **When** the policy engine evaluates,
   **Then** the job is rejected.
4. **Given** a submitter who has exceeded their per-epoch submission quota,
   **When** they submit another job, **Then** it is rejected with quota
   information.

---

### User Story 5 - Incident Response Halts and Quarantines Effectively (Priority: P3)

A potential sandbox escape is detected on a donor node. The incident
response system: (a) immediately freezes new job dispatch to the affected
host pool, (b) quarantines the workload class across all nodes,
(c) notifies the security contact, (d) logs all actions with full
attribution and reversibility tracking. The response completes within
minutes, not hours.

**Why this priority**: The red team's incident response recommendations
are sound. The current codebase has no incident response machinery beyond
the `AdminServiceHandler.halt()` stub.

**Independent Test**: Simulate a sandbox anomaly signal. Verify the
cascade: freeze, quarantine, notify, log. Verify that quarantined
workloads cannot restart. Verify that the freeze can be lifted by
authorized responders after investigation.

**Acceptance Scenarios**:

1. **Given** a reported anomaly on a donor node, **When** the incident
   handler triggers, **Then** the affected host is removed from the
   scheduling pool within 30 seconds.
2. **Given** a quarantined workload class, **When** a new job of that
   class is submitted, **Then** it is rejected with "workload class
   quarantined" until the quarantine is lifted.
3. **Given** all containment actions taken during an incident, **When** an
   auditor reviews the log, **Then** every action has: actor identity,
   timestamp, justification, and reversibility status.

---

### Edge Cases

- What happens when a donor's attestation expires mid-job? The job must
  complete or checkpoint within a grace period of 5 minutes or one
  checkpoint interval (whichever is shorter), then the node is
  re-evaluated before receiving new work.
- What happens when the policy engine and LLM advisory disagree? The
  deterministic policy engine is authoritative. Disagreements are logged
  for human review.
- What happens when an emergency halt is triggered but the governance
  quorum cannot be reached for the retroactive review within 7 days?
  The halt remains in effect. An escalation path to the founding
  governance group is activated. Note: the governance group must be
  formally named and its escalation procedures defined before any
  multi-institution deployment (Phase 1+).
- What happens when a donor is on a network that blocks the attestation
  verification endpoint? The donor cannot receive jobs above T0 tier
  until connectivity is restored. T0 jobs (WASM-only, public data,
  R>=5 replicas) may still run.

## Requirements *(mandatory)*

### Functional Requirements

**Sandbox enforcement (closing stub gaps)**

- **FR-S001**: All sandbox drivers (Firecracker, AppleVF, HyperV) MUST
  implement real VM lifecycle management — `create()`, `start()`,
  `freeze()`, `checkpoint()`, `terminate()`, and `cleanup()` MUST
  execute actual hypervisor operations, not stub state transitions.
- **FR-S002**: Every sandbox MUST enforce default-deny network egress at
  the hypervisor/namespace level. No guest traffic may leave the VM's
  virtual network interface unless the job manifest declares approved
  endpoints AND the policy engine has approved those endpoints.
- **FR-S003**: Every sandbox MUST enforce filesystem isolation. The guest
  MUST have zero visibility into the host filesystem. Scratch space MUST
  be size-capped per the resource envelope and fully reclaimed on job
  completion or termination.
- **FR-S004**: Linux idle detection (`linux_idle_ms()`) MUST return real
  values, not `None`. The preemption supervisor MUST work on all
  supported platforms, not only macOS.

**Attestation enforcement (closing verification gaps)**

- **FR-S010**: `verify_tpm2()` MUST validate PCR measurements against
  known-good values for the current signed agent build. It MUST NOT
  accept arbitrary non-empty quotes.
- **FR-S011**: `verify_sev_snp()` and `verify_tdx()` MUST validate
  hardware attestation reports against AMD/Intel root-of-trust
  certificates. Stub acceptance MUST be removed.
- **FR-S012**: `validate_manifest()` MUST cryptographically verify
  `submitter_signature` against the submitter's registered public key.
  All-zero signatures MUST be rejected.
- **FR-S013**: Workload artifact CIDs MUST be checked against an approved
  artifact registry before dispatch. Unsigned or unregistered artifacts
  MUST be rejected.

**Network and egress policy**

- **FR-S020**: The `network_egress_bytes` field in `ResourceEnvelope`
  MUST be enforced at the sandbox level, not just declared. Jobs with
  `network_egress_bytes: 0` MUST have all outbound connections blocked.
- **FR-S021**: Jobs requesting network access MUST declare specific
  endpoint allowlists in the manifest. The policy engine MUST validate
  these against an approved endpoint list.
- **FR-S022**: Jobs MUST NOT be able to reach donor LAN resources,
  RFC1918 private ranges, link-local addresses, cloud metadata endpoints,
  or management interfaces from within the sandbox.
- **FR-S023**: No runtime code fetch from the internet. Jobs MUST NOT
  be able to `pip install`, `curl`, or download secondary payloads.
  Everything needed MUST be in the reviewed artifact.

**Governance hardening**

- **FR-S030**: Safety-critical proposal types (`EmergencyHalt`,
  `ConstitutionAmendment`, admission policy relaxation) MUST require
  elevated quorum thresholds, separate from standard quadratic voting.
- **FR-S031**: `AdminServiceHandler.halt()` MUST require cryptographic
  authentication of the caller's designated on-call responder role.
- **FR-S032**: Separation of duties MUST be enforced: no single identity
  may both approve a workload class AND sign its artifacts AND deploy
  policy changes in the same approval flow.
- **FR-S033**: The mesh LLM MUST remain advisory-only for all phases
  through Phase 2. It MUST NOT autonomously change policy, approve jobs,
  or deploy updates without human governance approval per FR-125.

**Deterministic policy engine**

- **FR-S040**: A deterministic, testable policy engine MUST evaluate
  every job submission before scheduling. The engine MUST wrap the
  existing `validate_manifest()` as one step in a larger pipeline,
  adding checks for: submitter identity validity, workload class
  approval, artifact digest, resource limits, endpoint allowlists,
  data classification, quotas, and ban status.
- **FR-S041**: The policy engine MUST produce auditable accept/reject
  decisions with full reasoning logged.
- **FR-S042**: LLM-based review MUST be advisory-only. The deterministic
  engine is authoritative. Disagreements between LLM advisory and policy
  engine MUST be logged for human review.

**Supply chain and release security**

- **FR-S050**: Agent builds MUST be reproducible and produce identical
  artifacts from the same source. Code signing MUST use hardware-backed
  keys or equivalent protection.
- **FR-S051**: All workload artifacts MUST carry provenance attestations
  linking them to their build pipeline.
- **FR-S052**: A transparency log (Sigstore Rekor or equivalent) MUST
  record all artifact signatures and policy decisions.
- **FR-S053**: Release pipeline MUST support at minimum: development,
  staging, and production channels. Direct promotion from development
  to production MUST be blocked.

**Incident response**

- **FR-S060**: The system MUST support automated containment actions:
  freeze new dispatch to a host, quarantine a workload class, block a
  submitter, revoke an artifact, drain a host pool.
- **FR-S061**: All containment actions MUST be logged with actor
  identity, timestamp, justification, and reversibility status.
- **FR-S062**: Quarantined workload classes MUST be rejected at the
  policy engine level. Quarantine MUST persist until explicitly lifted
  by an authorized responder.

**Identity hardening (addressing real gaps, not the review's mischaracterizations)**

- **FR-S070**: Proof-of-personhood verification MUST be implemented
  with at least one concrete mechanism (e.g., BrightID, government ID
  verification, or equivalent). The current `proof_of_personhood: bool`
  field MUST connect to a real verification flow. Verification MUST
  occur at donor enrollment time and MUST be re-verified periodically
  at trust score recalculation intervals.
- **FR-S071**: Ed25519 key revocation MUST be supported. A compromised
  key MUST be revocable such that the associated PeerId is rejected by
  all coordinators.
- **FR-S072**: `donor_id` MUST have an enforced format and uniqueness
  constraint, not be an opaque String.
- **FR-S073**: OAuth2 verification flows for Humanity Points (email,
  phone, social accounts) MUST be implemented, not just tracked as
  booleans. Like proof-of-personhood, these MUST be verified at
  enrollment and re-verified at trust score recalculation intervals.

### Key Entities

- **PolicyDecision**: An auditable record of a deterministic policy
  evaluation — submitter, manifest CID, each check result, final
  verdict, timestamp, policy version used.
- **IncidentRecord**: A containment action taken during incident
  response — type, target, actor, justification, reversibility,
  timestamp.
- **ApprovedArtifact**: A workload artifact that has passed review —
  CID, provenance attestation, signer identity, approval timestamp,
  workload class.
- **GovernanceRole**: A separation-of-duties role assignment — identity,
  role (workload approver, artifact signer, policy deployer, on-call
  responder), granting authority, expiration. Roles MUST have a default
  expiration of 90 days (renewable) to prevent stale privilege accumulation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-S001**: Zero sandbox escape vulnerabilities survive to production
  deployment. All known escape vectors from the red team review are
  tested and blocked.
- **SC-S002**: 100% of jobs on production donors pass through the
  deterministic policy engine before scheduling. No bypass path exists.
- **SC-S003**: 100% of attestation verifications use real cryptographic
  validation. Zero stub-acceptance paths remain in production builds.
- **SC-S004**: Default-deny network egress is enforced for 100% of jobs
  that do not declare approved endpoints. Verified by automated
  adversarial testing on each supported platform.
- **SC-S005**: Separation of duties is enforced for all safety-critical
  governance actions. No single identity can complete a prohibited
  action combination.
- **SC-S006**: Incident containment actions (freeze, quarantine, block)
  complete within 60 seconds of trigger.
- **SC-S007**: All containment actions produce complete audit trails
  that satisfy the logging requirements in FR-S061.
- **SC-S008**: The system passes a formal red team exercise covering:
  malicious workload, compromised account, policy bypass attempt,
  sandbox escape attempt, and supply-chain injection — before any
  multi-institution deployment.
- **SC-S009**: Proof-of-personhood and OAuth2 verification flows are
  operational and tested with real verification providers.
- **SC-S010**: Agent builds are reproducible. Two independent builds
  from the same source produce bit-identical artifacts.

## Assumptions

- The project's constitutional identity as a volunteer compute federation
  ("anyone on Earth who opts in") is preserved. This spec does NOT adopt
  the red team's recommendation to convert to an institution-only model.
- Institutional SSO (InCommon/eduGAIN) is NOT adopted as the primary
  identity gate because it would exclude the majority of the intended
  global participant base. The existing Humanity Points + hardware
  attestation model is strengthened instead.
- Personal laptops, phones, and home servers remain first-class
  execution targets per Constitutional Principle II/III. Safety is
  achieved through VM isolation and sandboxing, not through exclusion
  of hardware classes.
- The mesh LLM's advisory-only status in Phases 0–2 is already specified.
  This spec reinforces it with explicit deterministic policy gates.
- Hardware TPM2 availability is assumed for T1+ trust tiers on x86
  platforms. Donors without TPM2 operate at T0 (WASM-only, high
  replication) — this is safe-by-default, not exclusionary.
- Sigstore/Rekor or an equivalent transparency log service is available
  as external infrastructure. The project does not need to operate its
  own transparency log in Phase 0.
- The phased rollout (starting small, expanding with evidence) is
  consistent with the existing spec's approach. This spec does not
  require single-institution Phase 0 — it requires passing go/no-go
  criteria before expansion, regardless of institutional backing.
