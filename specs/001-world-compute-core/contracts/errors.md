# Canonical Error Model

All World Compute v1 APIs use a single error envelope:

```json
{
  "code": "ERROR_CODE_NAME",
  "message": "Human-readable detail. Safe to display to the caller.",
  "details": {}
}
```

gRPC responses carry the status code in the gRPC trailing metadata
(`grpc-status`, `grpc-message`) and a `google.rpc.Status` detail object.
REST gateway maps gRPC status to HTTP status as shown in each entry below.

---

## Error Code Reference

### INVALID_MANIFEST
**gRPC**: INVALID_ARGUMENT (3) | **HTTP**: 400
**When raised**: The submitted `JobManifest` fails validation — missing required
fields (`image_cid`, `command`), unrecognized `acceptable_use` category, replica
count outside allowed range (1–20), `max_wallclock_secs` exceeds policy cap, or
malformed CIDv1 URI.
**Client action**: Fix the manifest and resubmit. The `details` object includes
`field` (the offending field path) and `constraint` (description of the violation).

### INSUFFICIENT_CREDITS
**gRPC**: FAILED_PRECONDITION (9) | **HTTP**: 402
**When raised**: The submitter account has fewer NCU than the pre-flight estimate
requires for `PRIORITY_CLASS_DONOR_REDEMPTION` or `PRIORITY_CLASS_PAID_SPONSORED`
jobs. Not raised for `PUBLIC_GOOD` jobs (no credit required).
**Client action**: Donate more compute to earn NCU, or switch to `PUBLIC_GOOD`
priority (subject to excess-capacity availability).

### ACCEPTABLE_USE_VIOLATION
**gRPC**: PERMISSION_DENIED (7) | **HTTP**: 403
**When raised**: The job's `acceptable_use` category or inferred workload content
is rejected by the acceptable-use filter (e.g., unauthorized scanning, illegal
content, targeted surveillance). This is a hard rejection; re-submission of the
same manifest will produce the same error.
**Client action**: Do not retry. Contact governance if the rejection is believed to
be erroneous. The `details.filter_rule` field identifies the triggered rule.

### NO_ELIGIBLE_NODES
**gRPC**: FAILED_PRECONDITION (9) | **HTTP**: 503
**When raised**: The scheduler cannot find enough donor nodes that satisfy all
constraints simultaneously: required caliber class, trust tier, residency
allowlist, opt-in workload class, and disjoint-bucket placement rules.
**Client action**: Relax constraints (lower caliber class, remove residency
restriction, reduce replica count) or wait and retry with exponential backoff.
The `details` object includes `constraint_failed` identifying which constraint
eliminated all candidates.

### QUORUM_FAILURE
**gRPC**: INTERNAL (13) | **HTTP**: 500
**When raised**: The replicated execution completed but fewer than the required
number of replicas produced matching canonical hashes. This indicates divergent
outputs, potentially due to non-deterministic workloads or Byzantine donors.
**Client action**: Re-submit the job. If the failure recurs, ensure the workload
is deterministic (seeded RNG, fixed sorting, no wall-clock dependency). The
`details.replica_results` field summarizes the divergence without leaking content.

### TRUST_TIER_MISMATCH
**gRPC**: FAILED_PRECONDITION (9) | **HTTP**: 400
**When raised**: The job requests `CONFIDENTIALITY_CONFIDENTIAL` but no T3 nodes
(SEV-SNP/TDX/H100 Confidential Compute) matching the other constraints are
available, or the submitter's account is not authorized to submit confidential jobs.
**Client action**: Downgrade to `CONFIDENTIALITY_OPAQUE`, wait for T3 node
availability, or contact support to enable confidential job access on the account.

### SANDBOX_UNAVAILABLE
**gRPC**: UNAVAILABLE (14) | **HTTP**: 503
**When raised**: The assigned donor node's sandbox driver failed to start the
workload (e.g., Firecracker VMM failed to launch, KVM unavailable, macOS VF
authorization denied). The scheduler automatically reschedules; this error is
surfaced only if all replicas fail to start within the deadline.
**Client action**: Retry with backoff. If persistent, check whether the requested
hardware class is available in the current cluster capacity.

### PREEMPTION_TIMEOUT
**gRPC**: DEADLINE_EXCEEDED (4) | **HTTP**: 504
**When raised**: A replica's checkpoint was not committed within the expected
window after the donor's preemption supervisor issued SIGSTOP. The replica is
declared lost and rescheduled from the last committed checkpoint. Surfaced to
the submitter only if all replicas were preempted simultaneously and the
rescheduling deadline was exceeded.
**Client action**: No action required — the scheduler handles recovery. If the
job is in a terminal FAILED state, re-submit.

### LEDGER_VERIFICATION_FAILED
**gRPC**: INTERNAL (13) | **HTTP**: 500
**When raised**: The coordinator attempted to write a ledger entry (credit
accrual, job receipt, governance record) and the Merkle chain verification
step failed — indicating a potential integrity violation.
**Client action**: Do not retry immediately. Report to the on-call team. The
`details.ledger_sequence` field identifies the affected sequence number.
This error triggers an automatic P0 incident alert.

### COORDINATOR_UNREACHABLE
**gRPC**: UNAVAILABLE (14) | **HTTP**: 503
**When raised**: The gRPC connection to the coordinator shard responsible for
the requested account or job could not be established within the deadline,
or the shard has no Raft leader.
**Client action**: Retry with exponential backoff (start at 1 s, max 60 s).
The REST gateway will attempt a failover to a replica shard before surfacing
this error. Check `GET /v1/cluster/status` for a cluster health indicator.

### RESIDENCY_CONSTRAINT_VIOLATION
**gRPC**: FAILED_PRECONDITION (9) | **HTTP**: 400
**When raised**: The job's `residency.allowed_regions` or
`residency.allowed_shard_categories` cannot be satisfied — either no nodes
in the allowed regions are available, or the requested shard category does
not have sufficient erasure-coded replicas to meet durability guarantees.
**Client action**: Broaden the residency constraint or wait for nodes in the
required region to become available.

### ATTESTATION_FAILED
**gRPC**: PERMISSION_DENIED (7) | **HTTP**: 403
**When raised**: A donor node's attestation quote was rejected by the
coordinator — TPM PCR mismatch, expired or revoked agent version, invalid
Ed25519 signature over the agent binary hash, or SEV-SNP guest measurement
mismatch. The node is quarantined; no jobs will be dispatched to it until
re-attestation succeeds.
**Client action** (for agent operators): Re-install or update the agent to
a version with a valid code signature and re-enroll. Check
`GET /v1/donor/status/{nodeId}` for the specific attestation failure reason.

### RATE_LIMITED
**gRPC**: RESOURCE_EXHAUSTED (8) | **HTTP**: 429
**When raised**: The caller has exceeded the rate limit for the current
rate-limit class (see `contracts/README.md`). The response includes
`Retry-After` (HTTP) or `x-retry-after-ms` (gRPC trailer) indicating
the earliest retry time.
**Client action**: Back off for at least the indicated duration. Implement
token-bucket or leaky-bucket client-side rate control to avoid repeated
429s.

### UNAUTHORIZED
**gRPC**: UNAUTHENTICATED (16) | **HTTP**: 401
**When raised**: No valid credential was presented (missing `Authorization`
header, expired bearer token, missing client certificate for mTLS-required
endpoints).
**Client action**: Refresh the bearer token (OAuth2 token refresh flow) or
ensure the mTLS certificate is valid and not expired. For agents, trigger
automatic certificate rotation.

### INTERNAL
**gRPC**: INTERNAL (13) | **HTTP**: 500
**When raised**: An unexpected error occurred in the coordinator or gateway.
This is a catch-all for bugs, panics, and unhandled conditions.
**Client action**: Retry once with backoff. If persistent, report to the
on-call team with the `x-request-id` header value from the response.

### UNAVAILABLE
**gRPC**: UNAVAILABLE (14) | **HTTP**: 503
**When raised**: The service is temporarily unable to handle the request
(rolling deployment, broker overload, coordinator election in progress).
**Client action**: Retry with exponential backoff. This is always a transient
condition; do not alert on a single occurrence.

### DEADLINE_EXCEEDED
**gRPC**: DEADLINE_EXCEEDED (4) | **HTTP**: 504
**When raised**: The operation did not complete within the caller's deadline
or the service's internal deadline. For job operations, the job itself is not
cancelled — it continues running; only the RPC timed out.
**Client action**: For job operations, poll `GetJob` to check current state.
For non-job operations, retry with a longer deadline.

### NOT_FOUND
**gRPC**: NOT_FOUND (5) | **HTTP**: 404
**When raised**: The referenced resource (job ID, node ID, proposal ID,
report ID, output name) does not exist or is not accessible to the caller.
**Client action**: Verify the identifier. If the resource was recently created,
allow up to 5 s for replication lag before retrying.

### ALREADY_EXISTS
**gRPC**: ALREADY_EXISTS (6) | **HTTP**: 409
**When raised**: A creation request conflicts with an existing resource — most
commonly a duplicate vote on the same proposal, or a duplicate enrollment
request for a node ID that is already registered.
**Client action**: For votes, this is a terminal rejection (one vote per
account per proposal). For enrollment, call `GetDonorStatus` to verify the
existing enrollment and update consent if needed.

### PERMISSION_DENIED
**gRPC**: PERMISSION_DENIED (7) | **HTTP**: 403
**When raised**: The caller is authenticated but lacks the required scope or
role for the operation (e.g., OAuth2 token has `submitter:read` but the
endpoint requires `submitter:write`; non-admin certificate used on
AdminService; governance vote cast by an account outside the eligible group).
**Client action**: Re-authenticate with the correct scope or contact an
administrator to grant the required role.
