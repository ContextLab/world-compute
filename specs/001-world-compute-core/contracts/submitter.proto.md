# SubmitterService — Protobuf-Style Contract Sketch

**Package**: `v1`
**Auth**: mTLS (agent cert) or OAuth2 bearer with `submitter:read` / `submitter:write`
**Implementation**: Rust (FR-006), Apache 2.0 (FR-099)

This service is the primary interface for anyone submitting compute jobs to the
cluster — donors redeeming credits, institutional users, or public-good submitters.
It covers the full job lifecycle from submission through result retrieval.

---

```protobuf
syntax = "proto3";
package v1;

// ── Enums ────────────────────────────────────────────────────────────────────

enum PriorityClass {
  PRIORITY_CLASS_UNSPECIFIED      = 0;
  PRIORITY_CLASS_DONOR_REDEMPTION = 1;  // p95 queue time < 2h (FR-032)
  PRIORITY_CLASS_PAID_SPONSORED   = 2;
  PRIORITY_CLASS_PUBLIC_GOOD      = 3;  // excess capacity only
  PRIORITY_CLASS_SELF_IMPROVEMENT = 4;  // reserved cluster slice (FR-033)
}

enum ConfidentialityLevel {
  CONFIDENTIALITY_UNSPECIFIED = 0;
  CONFIDENTIALITY_PUBLIC      = 1;  // any eligible node
  CONFIDENTIALITY_OPAQUE      = 2;  // encrypted in transit, T2+ nodes
  CONFIDENTIALITY_CONFIDENTIAL= 3;  // T3 only: SEV-SNP/TDX/H100 CC (FR-024)
}

enum JobPhase {
  JOB_PHASE_UNSPECIFIED  = 0;
  JOB_PHASE_VALIDATING   = 1;
  JOB_PHASE_STAGING      = 2;  // inputs being content-addressed
  JOB_PHASE_QUEUED       = 3;
  JOB_PHASE_LEASED       = 4;  // replicas assigned to nodes
  JOB_PHASE_RUNNING      = 5;
  JOB_PHASE_CHECKPOINTING= 6;
  JOB_PHASE_VERIFYING    = 7;  // quorum comparison in progress
  JOB_PHASE_VERIFIED     = 8;  // accepted result, ledger entry written
  JOB_PHASE_FAILED       = 9;
  JOB_PHASE_CANCELLED    = 10;
}

enum AcceptableUseCategory {
  AUC_UNSPECIFIED      = 0;
  AUC_SCIENTIFIC       = 1;
  AUC_PUBLIC_GOOD_ML   = 2;
  AUC_INDEXING         = 3;
  AUC_RENDERING        = 4;
  AUC_SELF_IMPROVEMENT = 5;
}

// ── Messages ─────────────────────────────────────────────────────────────────

message InputSpec {
  string cid   = 1;   // CIDv1 SHA-256 content address (FR, T5)
  string mount = 2;   // guest path, e.g. "/input/data.bin"
}

message OutputSpec {
  string name = 1;    // logical name
  string path = 2;    // guest path, e.g. "/output/result.txt"
}

message ResourceRequirements {
  uint32 cpu_vcores    = 1;
  uint64 memory_mib    = 2;
  uint64 storage_gib   = 3;
  bool   requires_gpu  = 4;
  string caliber_class = 5;  // minimum hardware class required
}

message ResidencyConstraint {
  repeated string allowed_regions  = 1;   // ISO 3166-1 alpha-2 country codes
  repeated string allowed_shard_categories = 2;  // per FR-074
}

message JobManifest {
  string                 name                  = 1;
  string                 image_cid             = 2;  // oci+cid: or wasm+cid: URI
  repeated string        command               = 3;
  repeated InputSpec     inputs                = 4;
  repeated OutputSpec    outputs               = 5;
  ResourceRequirements   resources             = 6;
  PriorityClass          priority              = 7;
  uint32                 replica_count         = 8;  // default 3 (FR-024)
  uint32                 max_wallclock_secs    = 9;
  ConfidentialityLevel   confidentiality       = 10;
  AcceptableUseCategory  acceptable_use        = 11;
  ResidencyConstraint    residency             = 12;
  uint32                 checkpoint_interval_secs = 13;  // default 60 (FR-023)
}

message SubmitJobRequest {
  JobManifest manifest      = 1;
  bool        dry_run       = 2;  // validate only, do not enqueue
}

message SubmitJobResponse {
  string   job_id           = 1;
  string   manifest_cid     = 2;  // CIDv1 of the submitted manifest
  JobPhase phase            = 3;
  uint64   estimated_queue_secs = 4;
  uint64   ncu_estimated    = 5;  // NCU * 1000, pre-flight estimate
}

message GetJobRequest {
  string job_id = 1;
}

message ReplicaStatus {
  string   node_id       = 1;
  JobPhase phase         = 2;
  string   checkpoint_cid= 3;
  int64    started_at    = 4;   // Unix epoch ms
  int64    updated_at    = 5;
}

message GetJobResponse {
  string                 job_id          = 1;
  JobPhase               phase           = 2;
  string                 manifest_cid    = 3;
  repeated ReplicaStatus replicas        = 4;
  string                 result_cid      = 5;   // populated when VERIFIED
  string                 receipt_hash    = 6;   // SHA-256 of ledger receipt
  string                 rekor_entry_id  = 7;   // Sigstore Rekor anchor
  uint64                 ncu_charged     = 8;   // NCU * 1000
  int64                  submitted_at    = 9;
  int64                  verified_at     = 10;
  string                 error_code      = 11;  // canonical error code if FAILED
  string                 error_message   = 12;
}

message StreamJobLogsRequest {
  string job_id    = 1;
  string replica_id= 2;  // optional; omit for merged stream from all replicas
  int64  since_ms  = 3;  // start from this timestamp; 0 = from beginning
}

message LogLine {
  int64  timestamp_ms  = 1;
  string replica_id    = 2;
  string stream        = 3;   // "stdout" or "stderr"
  string text          = 4;
  // NOTE: text MUST be redacted of submitter secrets before relay;
  // the sandbox stdout is piped through the acceptable-use filter.
}

message CancelJobRequest {
  string job_id  = 1;
  string reason  = 2;
}

message CancelJobResponse {
  bool   accepted         = 1;
  JobPhase terminal_phase = 2;  // phase at time of cancellation
}

message ListJobsRequest {
  string   account_id   = 1;
  JobPhase phase_filter = 2;   // 0 = all phases
  int32    page_size    = 3;
  string   page_token   = 4;
}

message ListJobsResponse {
  repeated GetJobResponse jobs           = 1;
  string                  next_page_token= 2;
}

message FetchResultRequest {
  string job_id     = 1;
  string output_name= 2;   // from JobManifest.outputs[].name
}

message FetchResultResponse {
  string result_cid   = 1;
  bytes  data         = 2;   // inline for results ≤ 4 MiB; empty otherwise
  string download_url = 3;   // pre-signed URL for larger results
  string receipt_hash = 4;
  string rekor_entry_id = 5;
}

// ── Service ──────────────────────────────────────────────────────────────────

service SubmitterService {

  // Validate and enqueue a job manifest. On dry_run=true, returns estimate only.
  // Runs acceptable-use filter (INVALID_MANIFEST, ACCEPTABLE_USE_VIOLATION errors).
  // Telemetry: span v1.SubmitterService/SubmitJob; ledger entry: JOB_SUBMITTED.
  // Rate limit: JOB_SUBMIT (10/min/account).
  // Auth: submitter:write scope.
  rpc SubmitJob(SubmitJobRequest) returns (SubmitJobResponse);

  // Poll current job state. Clients should poll at ≤1 req/5s while waiting;
  // prefer StreamJobLogs for real-time progress.
  // Telemetry: span v1.SubmitterService/GetJob.
  // Rate limit: JOB_READ.
  // Auth: submitter:read scope.
  rpc GetJob(GetJobRequest) returns (GetJobResponse);

  // Server-streaming log relay from sandbox stdout/stderr. Stream closes when
  // job reaches a terminal phase (VERIFIED, FAILED, CANCELLED).
  // Telemetry: span v1.SubmitterService/StreamJobLogs.
  // Rate limit: STREAM (5 concurrent/account).
  // Auth: submitter:read scope.
  rpc StreamJobLogs(StreamJobLogsRequest) returns (stream LogLine);

  // Request cancellation of a queued or running job. If replicas are running,
  // coordinator issues preempt; checkpoints are discarded. NCU charged pro-rata.
  // Telemetry: span v1.SubmitterService/CancelJob; ledger entry: JOB_CANCELLED.
  // Rate limit: JOB_READ.
  // Auth: submitter:write scope.
  rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);

  // List jobs for an account, with optional phase filter and pagination.
  // Telemetry: span v1.SubmitterService/ListJobs.
  // Rate limit: JOB_READ.
  // Auth: submitter:read scope.
  rpc ListJobs(ListJobsRequest) returns (ListJobsResponse);

  // Retrieve a verified job output by name. Returns inline bytes for small
  // results; pre-signed URL for results > 4 MiB.
  // Telemetry: span v1.SubmitterService/FetchResult.
  // Rate limit: JOB_READ.
  // Auth: submitter:read scope.
  rpc FetchResult(FetchResultRequest) returns (FetchResultResponse);
}
```

---

## Example Request / Response

**SubmitJob** — submitting `hello.yaml`:

```json
// Request
{
  "manifest": {
    "name": "hello-sha256",
    "image_cid": "oci+cid:bafybeihashofalpinewithsha256utils",
    "command": ["sha256sum", "/input/data.bin"],
    "inputs": [{ "cid": "bafybeig3k7inputdatacid", "mount": "/input/data.bin" }],
    "outputs": [{ "name": "result", "path": "/output/result.txt" }],
    "resources": { "cpu_vcores": 1, "memory_mib": 512 },
    "priority": "PRIORITY_CLASS_PUBLIC_GOOD",
    "replica_count": 3,
    "acceptable_use": "AUC_SCIENTIFIC"
  }
}

// Response
{
  "job_id": "job_8f9c2a4b1e",
  "manifest_cid": "bafybeimanifestcid",
  "phase": "JOB_PHASE_QUEUED",
  "estimated_queue_secs": 45,
  "ncu_estimated": 420
}
```
