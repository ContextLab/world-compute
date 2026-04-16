# AdminService — Protobuf-Style Contract Sketch

**Package**: `v1`
**Auth**: mTLS with role claim `wc-role: admin` in certificate Subject. No OAuth2
token ever carries admin scope. Admin certificates are hardware-backed (HSM or
YubiKey), rotated quarterly. Port 7444, firewall-restricted.
**Implementation**: Rust (FR-006), Apache 2.0 (FR-099)

This service provides emergency and operational control actions for designated
on-call responders. Every method is write-audited: each call produces a
tamper-evident ledger entry (ADMIN_ACTION) with the caller identity, timestamp,
parameters, and rationale. This audit trail is the primary accountability mechanism
for emergency powers exercised under the constitution's governance rules.

Constitution reference: "In response to an active Principle I (safety) or Principle II
(data-loss) incident, designated on-call responders MAY halt cluster operations
without prior governance approval; such actions MUST be reviewed retroactively
within 7 days."

---

```protobuf
syntax = "proto3";
package v1;

// ── Enums ────────────────────────────────────────────────────────────────────

enum BanReason {
  BAN_REASON_UNSPECIFIED          = 0;
  BAN_REASON_SANDBOX_ESCAPE       = 1;  // confirmed P0 incident
  BAN_REASON_ATTESTATION_FAILED   = 2;
  BAN_REASON_BYZANTINE_BEHAVIOR   = 3;
  BAN_REASON_ACCEPTABLE_USE_VIOLATION = 4;
  BAN_REASON_OFAC_COMPLIANCE      = 5;
}

enum AdminActionKind {
  ADMIN_ACTION_UNSPECIFIED          = 0;
  ADMIN_ACTION_HALT_DISPATCH        = 1;
  ADMIN_ACTION_RESUME_DISPATCH      = 2;
  ADMIN_ACTION_BAN_NODE             = 3;
  ADMIN_ACTION_ROTATE_COORDINATOR_KEY = 4;
}

// ── Messages ─────────────────────────────────────────────────────────────────

message AuditEntry {
  string          audit_id         = 1;
  AdminActionKind action           = 2;
  string          admin_account_id = 3;   // from mTLS certificate
  int64           timestamp_ms     = 4;
  string          rationale        = 5;   // required; min 20 characters
  string          incident_ref     = 6;   // optional link to incident record
  bytes           params_hash      = 7;   // SHA-256 of the request parameters
  string          ledger_entry_cid = 8;   // CIDv1 of the ledger record
  int64           review_deadline_ms = 9; // timestamp by which governance review is due
}

message HaltDispatchRequest {
  string rationale        = 1;   // required
  string affected_versions= 2;   // semver range of affected agent versions; empty = all
  string incident_ref     = 3;
}

message HaltDispatchResponse {
  bool       accepted         = 1;
  string     halt_token       = 2;   // opaque; required for ResumeDispatch
  AuditEntry audit            = 3;
  uint64     jobs_preempted   = 4;   // count of in-flight jobs halted
}

message ResumeDispatchRequest {
  string halt_token   = 1;   // from HaltDispatchResponse
  string rationale    = 2;   // required; documents what was fixed
}

message ResumeDispatchResponse {
  bool       accepted = 1;
  AuditEntry audit    = 2;
}

message BanNodeRequest {
  string     node_id    = 1;   // node to ban
  string     peer_id    = 2;   // libp2p PeerId (used for ban propagation via gossip)
  BanReason  reason     = 3;
  string     rationale  = 4;   // required
  string     incident_ref = 5;
  bool       propagate  = 6;   // if true, broadcast ban to all brokers via gossip
}

message BanNodeResponse {
  bool       accepted     = 1;
  AuditEntry audit        = 2;
  uint64     jobs_evicted = 3;   // jobs running on the banned node that were cancelled
}

message RotateCoordinatorKeyRequest {
  string shard_id   = 1;   // coordinator shard whose key to rotate; empty = all shards
  string rationale  = 2;
  bytes  new_public_key = 3;   // Ed25519 public key, DER-encoded
}

message RotateCoordinatorKeyResponse {
  bool       accepted        = 1;
  AuditEntry audit           = 2;
  int64      effective_at_ms = 3;   // key becomes active after overlap period (30 min)
}

// ── Service ──────────────────────────────────────────────────────────────────

service AdminService {

  // Halt all new job dispatches cluster-wide (or for specified agent versions).
  // Used for P0 sandbox-escape incidents (constitution Principle I), active
  // Principle II data-loss events, or OFAC compliance actions.
  // Takes effect within one gossip round-trip (≤5 s under normal conditions).
  // Produces ADMIN_ACTION ledger entry; governance review required within 7 days.
  // Telemetry: span v1.AdminService/HaltDispatch; ledger entry: ADMIN_ACTION.
  // Rate limit: ADMIN (60/min/admin).
  // Auth: mTLS + wc-role:admin.
  rpc HaltDispatch(HaltDispatchRequest) returns (HaltDispatchResponse);

  // Resume job dispatches after a halt. Requires the halt_token from HaltDispatch.
  // The rationale must document what remediation was applied.
  // Produces ADMIN_ACTION ledger entry.
  // Telemetry: span v1.AdminService/ResumeDispatch; ledger entry: ADMIN_ACTION.
  // Rate limit: ADMIN.
  // Auth: mTLS + wc-role:admin.
  rpc ResumeDispatch(ResumeDispatchRequest) returns (ResumeDispatchResponse);

  // Permanently ban a node from receiving job dispatches. Ban is recorded in the
  // ledger and propagated to all brokers via libp2p gossip if propagate=true.
  // All jobs running on the node are cancelled; submitters are notified.
  // Produces ADMIN_ACTION ledger entry.
  // Telemetry: span v1.AdminService/BanNode; ledger entry: ADMIN_ACTION.
  // Rate limit: ADMIN.
  // Auth: mTLS + wc-role:admin.
  rpc BanNode(BanNodeRequest) returns (BanNodeResponse);

  // Rotate the threshold-signing key for one or all coordinator shards.
  // Used after a coordinator compromise or as a scheduled quarterly rotation.
  // New key becomes active after a 30-minute overlap period to allow in-flight
  // quorum operations to complete with the old key.
  // Produces ADMIN_ACTION ledger entry.
  // Telemetry: span v1.AdminService/RotateCoordinatorKey; ledger entry: ADMIN_ACTION.
  // Rate limit: ADMIN.
  // Auth: mTLS + wc-role:admin.
  rpc RotateCoordinatorKey(RotateCoordinatorKeyRequest) returns (RotateCoordinatorKeyResponse);
}
```

---

## Audit Trail Semantics

Every AdminService call writes an `ADMIN_ACTION` ledger entry before the action takes
effect. The entry includes:

- `admin_account_id` — from the mTLS certificate CN; not redactable
- `action` — enum value
- `timestamp_ms` — coordinator wall clock (NTP-synchronized)
- `rationale` — human-readable justification (minimum 20 characters; empty string is
  rejected with INVALID_ARGUMENT)
- `params_hash` — SHA-256 of the serialized request, so the full parameters can be
  reconstructed from the audit log if needed
- `review_deadline_ms` — 7 days from timestamp; governance review is required by this
  deadline per constitution emergency powers rules

Ledger entries for admin actions are included in the same Merkle chain as compute
provenance entries and anchored to Sigstore Rekor, making them tamper-evident and
publicly auditable. Admin actions cannot be retroactively deleted or modified.

## Example

**HaltDispatch** — P0 sandbox escape response:

```json
// Request
{
  "rationale": "Confirmed Firecracker escape via CVE-2026-XXXXX in agent v0.1.0-v0.1.3. Halting all dispatches to affected versions pending patch.",
  "affected_versions": ">=0.1.0 <0.1.4",
  "incident_ref": "INC-2026-001"
}

// Response
{
  "accepted": true,
  "halt_token": "halt_tok_a1b2c3",
  "audit": {
    "audit_id": "audit_d4e5f6",
    "action": "ADMIN_ACTION_HALT_DISPATCH",
    "admin_account_id": "acct_oncall_eng",
    "timestamp_ms": 1776499200000,
    "ledger_entry_cid": "bafybeiauditledgercid",
    "review_deadline_ms": 1777104000000
  },
  "jobs_preempted": 4712
}
```
