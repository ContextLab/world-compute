# DonorService — Protobuf-Style Contract Sketch

**Package**: `v1`
**Auth**: mTLS (per-account Ed25519 agent certificate required for all methods)
**Implementation**: Rust (FR-006), Apache 2.0 (FR-099)

This service is the interface between the enrolled donor agent and the World Compute
control plane. All calls originate from the agent process running on donor hardware;
no method in this service is intended for direct human invocation (the CLI wraps
these calls).

---

```protobuf
syntax = "proto3";
package v1;

// ── Enums ────────────────────────────────────────────────────────────────────

enum DonorStatus {
  DONOR_STATUS_UNSPECIFIED = 0;
  DONOR_STATUS_ACTIVE      = 1;  // agent enrolled, accepting work
  DONOR_STATUS_IDLE        = 2;  // enrolled, no jobs running
  DONOR_STATUS_PAUSED      = 3;  // donor explicitly paused
  DONOR_STATUS_PREEMPTED   = 4;  // sovereignty event; yielding resources
  DONOR_STATUS_WITHDRAWN   = 5;  // withdrawal initiated, cleaning up
}

enum TrustTier {
  TRUST_TIER_UNSPECIFIED = 0;
  TRUST_TIER_T0          = 1;  // browser/WASM, no VM
  TRUST_TIER_T1          = 2;  // VM only, no TPM attestation
  TRUST_TIER_T2          = 3;  // TPM 2.0 PCR attested VM
  TRUST_TIER_T3          = 4;  // SEV-SNP / TDX / H100 Confidential Compute
}

enum WorkloadClass {
  WORKLOAD_CLASS_UNSPECIFIED   = 0;
  WORKLOAD_CLASS_SCIENTIFIC    = 1;
  WORKLOAD_CLASS_PUBLIC_GOOD_ML= 2;
  WORKLOAD_CLASS_INDEXING      = 3;
  WORKLOAD_CLASS_RENDERING     = 4;
  WORKLOAD_CLASS_SELF_IMPROVE  = 5;
}

// ── Messages ─────────────────────────────────────────────────────────────────

message ResourceCapacity {
  uint32 cpu_vcores       = 1;   // logical cores offered to cluster
  uint64 memory_mib       = 2;
  uint64 storage_gib      = 3;
  bool   has_gpu          = 4;
  string gpu_model        = 5;   // e.g. "RTX 3080"
  uint32 gpu_vram_gib     = 6;
  string caliber_class    = 7;   // e.g. "class-2-consumer-gpu"
  uint32 cpu_cap_pct      = 8;   // donor-configured cap (FR-003)
}

message AttestationQuote {
  TrustTier tier              = 1;
  bytes     tpm_pcr_quote     = 2;   // TPM 2.0 PCR[0..7] quote, DER-encoded
  bytes     sev_measurement   = 3;   // SEV-SNP guest measurement, if tier=T3
  bytes     agent_sig         = 4;   // Ed25519 signature over agent binary hash
  string    agent_version     = 5;
  string    agent_git_commit  = 6;   // reproducible build ref
}

message EnrollRequest {
  string              peer_id              = 1;  // libp2p PeerId
  ResourceCapacity    capacity             = 2;
  AttestationQuote    attestation          = 3;
  repeated WorkloadClass opted_in_classes  = 4;  // granular consent (FR-003)
  repeated string     shard_categories     = 5;  // residency allowlist (FR-074)
  string              schedule_cron        = 6;  // e.g. "0 22 * * *" (active window)
  string              agent_version        = 7;
}

message EnrollResponse {
  string   account_id       = 1;   // assigned account UUID
  string   node_id          = 2;   // cluster-local node identifier
  string   cluster_id       = 3;
  bytes    broker_addrs     = 4;   // serialized multiaddrs for nearest brokers
  uint64   credit_balance   = 5;   // NCU * 1000 (fixed-point)
}

message HeartbeatRequest {
  string       node_id       = 1;
  DonorStatus  status        = 2;
  double       cpu_util_pct  = 3;
  double       mem_util_pct  = 4;
  double       gpu_util_pct  = 5;
  int64        uptime_secs   = 6;
  string       active_lease  = 7;  // current lease ID, empty if idle
}

message HeartbeatResponse {
  bool   should_pause        = 1;  // control-plane-initiated pause (e.g. P0 incident)
  string control_message     = 2;  // human-readable reason if should_pause=true
  uint64 credit_balance      = 3;  // NCU * 1000, refreshed each heartbeat
}

message GetDonorStatusRequest {
  string node_id = 1;
}

message GetDonorStatusResponse {
  string           account_id       = 1;
  string           node_id          = 2;
  DonorStatus      status           = 3;
  TrustTier        trust_tier       = 4;
  ResourceCapacity capacity         = 5;
  uint64           ncu_earned       = 6;   // NCU * 1000
  uint64           ncu_spent        = 7;
  uint64           jobs_run         = 8;
  uint64           jobs_verified    = 9;
  uint64           jobs_disputed    = 10;
  int64            uptime_secs      = 11;
  string           cluster_id       = 12;
}

message UpdateConsentRequest {
  string                 node_id          = 1;
  repeated WorkloadClass opted_in_classes = 2;  // full replacement list
  repeated string        shard_categories = 3;
  string                 schedule_cron    = 4;
  uint32                 cpu_cap_pct      = 5;
}

message UpdateConsentResponse {
  bool   accepted  = 1;
  string message   = 2;
}

message WithdrawRequest {
  string node_id       = 1;
  bool   force         = 2;  // if true, preempt running jobs immediately
}

message WithdrawResponse {
  bool   initiated     = 1;
  string cleanup_token = 2;  // opaque token; present to confirm cleanup complete
}

message ConfirmWithdrawRequest {
  string node_id       = 1;
  string cleanup_token = 2;
}

message ConfirmWithdrawResponse {
  bool   complete       = 1;
  uint64 final_ncu_balance = 2;
}

// ── Service ──────────────────────────────────────────────────────────────────

service DonorService {

  // Enroll a new donor node with the cluster. Validates attestation quote
  // (FR-013). Returns broker addresses and initial credit balance.
  // Telemetry: span v1.DonorService/Enroll; ledger entry: DONOR_ENROLLED.
  // Rate limit: JOB_READ class (enrollment is infrequent).
  rpc Enroll(EnrollRequest) returns (EnrollResponse);

  // Periodic liveness signal from an enrolled node. Must be called every ≤30s.
  // Returns optional control-plane directives (pause, halt). Credit balance
  // is refreshed in each response to avoid a separate round-trip.
  // Telemetry: span v1.DonorService/Heartbeat; no ledger entry (high-frequency).
  // Rate limit: DONOR_HEARTBEAT (120/min/node).
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);

  // Read current status for a node. Used by CLI `worldcompute donor status`.
  // Telemetry: span v1.DonorService/GetDonorStatus.
  // Rate limit: JOB_READ class.
  rpc GetDonorStatus(GetDonorStatusRequest) returns (GetDonorStatusResponse);

  // Update donor consent: workload classes, shard residency, schedule, CPU cap.
  // Changes take effect on the next lease cycle; running leases are unaffected.
  // Telemetry: span v1.DonorService/UpdateConsent; ledger entry: CONSENT_UPDATED.
  // Rate limit: JOB_READ class.
  rpc UpdateConsent(UpdateConsentRequest) returns (UpdateConsentResponse);

  // Initiate graceful withdrawal. The agent may continue running existing leases
  // through completion (or force=true to preempt immediately). Returns a cleanup
  // token that must be presented to ConfirmWithdraw.
  // Telemetry: span v1.DonorService/Withdraw; ledger entry: WITHDRAW_INITIATED.
  // Rate limit: JOB_READ class.
  rpc Withdraw(WithdrawRequest) returns (WithdrawResponse);

  // Confirm withdrawal is complete: all cluster state removed from host (FR-004).
  // Coordinator closes the account's node record.
  // Telemetry: span v1.DonorService/ConfirmWithdraw; ledger entry: DONOR_WITHDRAWN.
  // Rate limit: JOB_READ class.
  rpc ConfirmWithdraw(ConfirmWithdrawRequest) returns (ConfirmWithdrawResponse);
}
```

---

## Example Request / Response

**Enroll** — agent enrolling on a fresh Linux laptop:

```json
// Request
{
  "peer_id": "12D3KooWR7bHxkjFe2q...",
  "capacity": { "cpu_vcores": 8, "memory_mib": 16384, "storage_gib": 200,
                "has_gpu": false, "caliber_class": "class-1-cpu" },
  "attestation": { "tier": "TRUST_TIER_T2", "agent_version": "0.1.0" },
  "opted_in_classes": ["WORKLOAD_CLASS_SCIENTIFIC", "WORKLOAD_CLASS_SELF_IMPROVE"],
  "shard_categories": ["public"],
  "schedule_cron": "0 22 * * *"
}

// Response
{
  "account_id": "acct_a1b2c3d4",
  "node_id": "node_eu1_f5e6",
  "cluster_id": "wc-global",
  "credit_balance": 0
}
```
