# Data Model: Safety Hardening

**Date**: 2026-04-16
**Source**: [spec.md](spec.md) Key Entities section + functional requirements

## New Entities

### PolicyDecision

An auditable record of a deterministic policy engine evaluation.

| Field | Type | Description |
|-|-|-|
| decision_id | UUID | Unique identifier for this evaluation |
| manifest_cid | Cid | CID of the evaluated job manifest |
| submitter_peer_id | PeerId | Identity of the submitter |
| policy_version | String | Version of the policy ruleset applied |
| checks | Vec\<PolicyCheck\> | Individual check results (see below) |
| verdict | Verdict | Accept or Reject |
| reject_reason | Option\<String\> | Human-readable reason if rejected |
| llm_advisory_flag | Option\<String\> | LLM advisory opinion if provided |
| llm_disagrees | bool | True if LLM flagged but policy approved (or vice versa) |
| timestamp | Timestamp | When the evaluation occurred |

**Verdict** enum: `Accept`, `Reject`

**PolicyCheck** struct:

| Field | Type | Description |
|-|-|-|
| check_name | String | e.g., "submitter_identity", "workload_class", "artifact_registry" |
| passed | bool | Whether this check passed |
| detail | String | Explanation of the result |

**Relationships**:
- References a `JobManifest` by `manifest_cid`
- References a submitter by `submitter_peer_id`
- Created by the policy engine (FR-S040)
- Consumed by audit logging and transparency reporting

**State transitions**: None — PolicyDecisions are immutable records.

---

### IncidentRecord

A record of a containment action taken during incident response.

| Field | Type | Description |
|-|-|-|
| record_id | UUID | Unique identifier |
| incident_id | UUID | Groups related actions into one incident |
| action_type | ContainmentAction | Type of action taken |
| target | String | What the action targets (host ID, workload class, submitter ID, artifact CID) |
| actor_peer_id | PeerId | Identity of the responder who took the action |
| actor_role | GovernanceRole | Role under which the action was authorized |
| justification | String | Why the action was taken |
| reversible | bool | Whether the action can be undone |
| reversed_by | Option\<UUID\> | If reversed, the record_id of the reversal action |
| timestamp | Timestamp | When the action was taken |

**ContainmentAction** enum: `FreezeHost`, `QuarantineWorkloadClass`,
`BlockSubmitter`, `RevokeArtifact`, `DrainHostPool`, `LiftFreeze`,
`LiftQuarantine`, `UnblockSubmitter`

**Relationships**:
- Groups into incidents by `incident_id`
- References the acting responder by `actor_peer_id` + `actor_role`
- Quarantine actions are enforced by the policy engine (FR-S062)

**State transitions**: Containment actions create immutable records.
Reversal actions reference the original via `reversed_by`.

---

### ApprovedArtifact

A workload artifact that has passed review and is registered for dispatch.

| Field | Type | Description |
|-|-|-|
| artifact_cid | Cid | Content-addressed identifier (primary key) |
| workload_class | String | Category of workload (e.g., "scientific-batch", "model-inference") |
| provenance | ProvenanceAttestation | Build pipeline provenance |
| signer_peer_id | PeerId | Identity of the artifact signer |
| approved_at | Timestamp | When the artifact was approved |
| approved_by | PeerId | Identity of the approver (must differ from signer per FR-S032) |
| revoked | bool | Whether the artifact has been revoked |
| revoked_at | Option\<Timestamp\> | When revoked, if applicable |
| transparency_log_entry | Option\<String\> | Sigstore/Rekor log index |

**ProvenanceAttestation** struct:

| Field | Type | Description |
|-|-|-|
| build_source | String | Source repository and commit |
| build_pipeline | String | CI pipeline identifier |
| build_timestamp | Timestamp | When the build ran |
| reproducible | bool | Whether the build is verified reproducible |
| sbom_cid | Option\<Cid\> | CID of the SBOM if generated |

**Relationships**:
- Referenced by `JobManifest.workload_cid` (existing field)
- Checked by policy engine artifact registry lookup (FR-S013)
- Signer and approver must be different identities (FR-S032)

**State transitions**: `approved` → `revoked` (one-way, via IncidentRecord)

---

### GovernanceRole

A separation-of-duties role assignment binding an identity to a capability.

| Field | Type | Description |
|-|-|-|
| assignment_id | UUID | Unique identifier |
| peer_id | PeerId | Identity the role is assigned to |
| role | RoleType | The role granted |
| granted_by | PeerId | Identity that granted this role |
| granted_at | Timestamp | When the role was assigned |
| expires_at | Option\<Timestamp\> | Expiration if time-limited |
| revoked | bool | Whether the assignment has been revoked |

**RoleType** enum: `WorkloadApprover`, `ArtifactSigner`,
`PolicyDeployer`, `OnCallResponder`, `GovernanceVoter`

**Separation-of-duties constraints** (FR-S032):
- No single `peer_id` may hold both `WorkloadApprover` AND `ArtifactSigner`
  for the same approval flow
- No single `peer_id` may hold `ArtifactSigner` AND `PolicyDeployer`
  simultaneously

**Relationships**:
- Binds to a peer identity by `peer_id`
- Checked by governance actions (FR-S031, FR-S032)
- `OnCallResponder` role required for `AdminServiceHandler.halt()`

**State transitions**: `active` → `expired` (automatic) or `revoked` (explicit)

## Modified Entities

### JobManifest (existing — `src/scheduler/manifest.rs`)

**Changes**:
- `submitter_signature`: Now cryptographically verified by policy engine
  (FR-S012). All-zero signatures rejected.
- `workload_cid`: Now checked against ApprovedArtifact registry (FR-S013)
- Endpoint allowlist: New optional field for jobs requesting network access

### QuadraticVoteBudget (existing — `src/governance/voting.rs`)

**Changes**:
- Safety-critical proposals (`EmergencyHalt`, `ConstitutionAmendment`)
  use elevated quorum thresholds (FR-S030)
- Minimum HP score enforced for safety-critical voters
- `ConstitutionAmendment` has mandatory 7-day review period

### HumanityPoints (existing — `src/governance/humanity_points.rs`)

**Changes**:
- Boolean fields connected to real verification flows (FR-S070, FR-S073)
- Verification occurs at enrollment, re-verified at trust score
  recalculation intervals
