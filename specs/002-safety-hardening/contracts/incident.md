# Contract: Incident Response

**Date**: 2026-04-16
**Scope**: FR-S060, FR-S061, FR-S062

## Interface

Incident response provides containment action primitives that can be
triggered by authorized responders (OnCallResponder role) or by
automated anomaly detection.

### Execute Containment Action

**Input**: `ContainmentRequest` — action type, target, justification,
actor identity + role proof

**Output**: `IncidentRecord` — immutable audit record of the action taken

**Authorization**: Caller must present cryptographic proof of
`OnCallResponder` role assignment (GovernanceRole). Unauthorized callers
are rejected.

### Containment Actions

| Action | Target | Effect | Reversible |
|-|-|-|-|
| FreezeHost | Host PeerId | Remove host from scheduling pool; no new jobs dispatched | Yes (LiftFreeze) |
| QuarantineWorkloadClass | Workload class name | Policy engine rejects all jobs of this class | Yes (LiftQuarantine) |
| BlockSubmitter | Submitter PeerId | Policy engine rejects all jobs from this submitter | Yes (UnblockSubmitter) |
| RevokeArtifact | Artifact CID | Artifact removed from approved registry; policy engine rejects jobs using it | No (re-approval required) |
| DrainHostPool | Pool identifier | Checkpoint all running jobs on pool, migrate to other pools, remove pool from scheduling | Yes (re-add pool) |

### Cascade behavior

- `FreezeHost`: Running jobs on the host are allowed to complete their
  current checkpoint interval, then evicted. No new jobs are dispatched.
- `QuarantineWorkloadClass`: Running jobs of the quarantined class are
  NOT terminated mid-execution (to avoid data loss) but are flagged for
  review. No new jobs of the class are accepted.
- `DrainHostPool`: All running jobs are checkpointed and rescheduled.
  The pool is removed from the scheduler. This is the response for
  suspected pool-wide compromise.

### Audit requirements

Every `IncidentRecord` MUST contain:
- `actor_peer_id`: Who took the action
- `actor_role`: Under what authority
- `justification`: Free-text explanation
- `reversible`: Whether the action can be undone
- `reversed_by`: If later reversed, link to the reversal record
- `timestamp`: When the action was taken

### Automated triggers

The following anomalies MAY trigger automated containment (subject to
policy configuration):

- Repeated denied syscalls from a sandbox (threshold: configurable)
- Unexpected outbound connection attempts (any, if egress is deny-all)
- Crash loops from a workload class (threshold: 3 failures in 10 minutes)
- Attestation verification failure during re-verification

Automated containment actions use a system identity with
`OnCallResponder` role and justification set to the anomaly description.
All automated actions are flagged for human review within 24 hours.

### Emergency halt

`AdminServiceHandler.halt()` is a special case that freezes ALL new job
dispatch cluster-wide. It requires `OnCallResponder` role proof
(FR-S031). Per constitution, emergency halts must be reviewed
retroactively within 7 days.
