# Contract: Deterministic Policy Engine

**Date**: 2026-04-16
**Scope**: FR-S040, FR-S041, FR-S042

## Interface

The policy engine exposes a single evaluation entry point that wraps
`validate_manifest()` in a larger pipeline.

### Evaluate Job Submission

**Input**: `JobManifest` + submitter identity context

**Output**: `PolicyDecision` (accept/reject with full reasoning)

**Pipeline steps** (sequential, short-circuit on first rejection):

1. **Manifest structural validation** — delegates to existing
   `validate_manifest()`. Checks: non-empty workload CID, non-empty
   command, wallclock within range, confidentiality/verification
   compatibility.

2. **Submitter identity check** — verifies submitter PeerId is registered,
   not revoked, and meets minimum HP threshold for the requested workload
   class.

3. **Signature verification** — cryptographically verifies
   `submitter_signature` against the submitter's registered public key.
   Rejects all-zero and invalid signatures.

4. **Artifact registry lookup** — checks `workload_cid` against the
   ApprovedArtifact registry. Rejects unsigned or unregistered artifacts.

5. **Workload class approval** — verifies the artifact's workload class
   is approved and not quarantined.

6. **Resource limit validation** — checks requested resources against
   per-user and per-institution quotas.

7. **Endpoint allowlist validation** — if `network_egress_bytes > 0`,
   validates declared endpoints against approved endpoint list.

8. **Data classification check** — verifies data sensitivity level is
   compatible with available host pools at the required trust tier.

9. **Quota enforcement** — checks per-epoch submission quotas for the
   submitter.

10. **Ban status check** — verifies submitter is not banned or
    cooldown-restricted.

### Error semantics

Each step produces a `PolicyCheck` with `check_name`, `passed`, and
`detail`. On rejection, the pipeline returns the first failing check's
detail as the `reject_reason`. The full set of checks run up to the
rejection point is included in the `PolicyDecision` for audit.

### LLM advisory layer

After the deterministic pipeline completes (regardless of verdict), the
LLM advisory layer MAY flag the submission. If the LLM disagrees with
the deterministic verdict, `llm_disagrees: true` is set and the
disagreement is logged. The LLM never overrides the deterministic verdict.

### Idempotency

Evaluating the same manifest twice with the same policy version produces
the same verdict (deterministic). Policy version changes are explicit
and logged.
