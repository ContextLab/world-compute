# ClusterService — Protobuf-Style Contract Sketch

**Package**: `v1`
**Auth**: Public read (no auth required for status/ledger methods); mTLS or OAuth2
bearer `cluster:read` scope for peer enumeration.
**Implementation**: Rust (FR-006), Apache 2.0 (FR-099)

This service exposes cluster health, topology, and ledger verification. It is the
interface for operators, monitoring dashboards, and third-party auditors. Most methods
are intentionally read-only and publicly accessible to support open auditability
(constitution: "open and auditable").

---

```protobuf
syntax = "proto3";
package v1;

// ── Enums ────────────────────────────────────────────────────────────────────

enum ClusterHealthStatus {
  CLUSTER_HEALTH_UNSPECIFIED = 0;
  CLUSTER_HEALTH_HEALTHY     = 1;  // all systems nominal
  CLUSTER_HEALTH_DEGRADED    = 2;  // some coordinators/brokers unhealthy
  CLUSTER_HEALTH_INCIDENT    = 3;  // active P0 incident; new dispatches halted
  CLUSTER_HEALTH_PARTITIONED = 4;  // network partition detected
}

enum PeerRole {
  PEER_ROLE_UNSPECIFIED  = 0;
  PEER_ROLE_DONOR        = 1;
  PEER_ROLE_BROKER       = 2;
  PEER_ROLE_COORDINATOR  = 3;
  PEER_ROLE_ADAPTER      = 4;   // Slurm/K8s gateway adapter
}

// ── Messages ─────────────────────────────────────────────────────────────────

message ClusterCapacitySummary {
  uint64 total_nodes_enrolled    = 1;
  uint64 nodes_active            = 2;
  uint64 nodes_idle              = 3;
  uint64 nodes_paused            = 4;
  uint64 total_cpu_vcores        = 5;
  uint64 total_memory_gib        = 6;
  uint64 total_storage_gib       = 7;
  uint64 gpu_nodes               = 8;
  uint64 jobs_running            = 9;
  uint64 jobs_queued             = 10;
  double ncu_earned_last_24h     = 11;
  double ncu_spent_last_24h      = 12;
}

message ClusterStatus {
  ClusterHealthStatus    health             = 1;
  string                 cluster_id         = 2;
  string                 version            = 3;   // coordinator software version
  int64                  status_at          = 4;   // Unix epoch ms
  ClusterCapacitySummary capacity           = 5;
  uint32                 coordinator_shards = 6;
  uint32                 active_brokers     = 7;
  string                 ledger_head_cid    = 8;   // latest Merkle root CID
  string                 rekor_entry_id     = 9;   // most recent Sigstore anchor
  string                 incident_summary   = 10;  // non-empty if health=INCIDENT
}

message GetClusterStatusRequest {
  // No parameters — returns global aggregate status.
}

message PeerInfo {
  string   peer_id         = 1;
  PeerRole role            = 2;
  string   region          = 3;   // ISO 3166-1 alpha-2
  string   caliber_class   = 4;
  string   trust_tier      = 5;
  bool     is_healthy      = 6;
  int64    last_seen_ms    = 7;
  uint32   jobs_running    = 8;
}

message ListPeersRequest {
  PeerRole  role_filter     = 1;  // 0 = all roles
  string    region_filter   = 2;  // ISO code; empty = all regions
  bool      healthy_only    = 3;
  int32     page_size       = 4;
  string    page_token      = 5;
}

message ListPeersResponse {
  repeated PeerInfo peers          = 1;
  string            next_page_token= 2;
  uint64            total_count    = 3;
}

message GetLedgerHeadRequest {
  // No parameters — returns the current Merkle chain head.
}

message LedgerHead {
  string cid              = 1;   // CIDv1 SHA-256 of the current Merkle root
  uint64 sequence         = 2;   // monotonically increasing ledger sequence number
  string rekor_entry_id   = 3;   // Sigstore Rekor entry anchoring this root
  int64  anchored_at_ms   = 4;   // Unix epoch ms of Rekor anchor
  bytes  threshold_sig    = 5;   // m-of-n threshold signature from coordinator quorum
  uint32 sig_threshold    = 6;   // m
  uint32 sig_participants = 7;   // n
}

message VerifyReceiptRequest {
  string job_id         = 1;
  string receipt_hash   = 2;   // SHA-256 hex string from GetJobResponse
}

message VerificationResult {
  bool   valid              = 1;
  string job_id             = 2;
  string receipt_hash       = 3;
  string ledger_root_cid    = 4;   // root under which this receipt is included
  string rekor_entry_id     = 5;
  uint32 merkle_proof_depth = 6;
  bytes  merkle_proof       = 7;   // serialized inclusion proof
  string failure_reason     = 8;   // non-empty when valid=false
}

// ── Service ──────────────────────────────────────────────────────────────────

service ClusterService {

  // Return aggregate cluster health and capacity. No auth required.
  // Suitable for public monitoring dashboards and status pages.
  // Telemetry: span v1.ClusterService/GetClusterStatus.
  // Rate limit: CLUSTER_READ (600/min global).
  rpc GetClusterStatus(GetClusterStatusRequest) returns (ClusterStatus);

  // Enumerate known peers (donors, brokers, coordinators, adapters).
  // Supports filtering by role, region, and health.
  // Telemetry: span v1.ClusterService/ListPeers.
  // Rate limit: CLUSTER_READ.
  // Auth: cluster:read scope (peer list may reveal node geography).
  rpc ListPeers(ListPeersRequest) returns (ListPeersResponse);

  // Return the current ledger Merkle head with Sigstore anchor proof.
  // Anchored every 10 minutes; this call returns the most recently anchored head.
  // No auth required — supports public auditability.
  // Telemetry: span v1.ClusterService/GetLedgerHead.
  // Rate limit: CLUSTER_READ.
  rpc GetLedgerHead(GetLedgerHeadRequest) returns (LedgerHead);

  // Verify that a job receipt is included in the Merkle ledger and anchored
  // to Sigstore Rekor. Returns the inclusion proof so callers can verify
  // locally without trusting the coordinator.
  // No auth required — supports independent third-party auditing.
  // Telemetry: span v1.ClusterService/VerifyReceipt.
  // Rate limit: CLUSTER_READ.
  rpc VerifyReceipt(VerifyReceiptRequest) returns (VerificationResult);
}
```

---

## Example Request / Response

**VerifyReceipt** — third-party auditor checking a job result:

```json
// Request
{
  "job_id": "job_8f9c2a4b1e",
  "receipt_hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}

// Response
{
  "valid": true,
  "job_id": "job_8f9c2a4b1e",
  "receipt_hash": "sha256:e3b0c44298...",
  "ledger_root_cid": "bafybeimerklerootcid",
  "rekor_entry_id": "3f8c9d2a1b4e",
  "merkle_proof_depth": 14,
  "merkle_proof": "<base64-encoded inclusion proof>"
}
```
