# GovernanceService — Protobuf-Style Contract Sketch

**Package**: `v1`
**Auth**: `governance:read` (list/get) or `governance:write` (create/vote) OAuth2
scope, or mTLS account certificate. Votes are signed by the caller's account key and
recorded in the ledger (FR-104).
**Implementation**: Rust (FR-006), Apache 2.0 (FR-099)

This service exposes the on-chain governance proposal and voting system. Proposal
records and vote records are written to the same Merkle-chained ledger as compute
provenance, ensuring governance history is tamper-evident and publicly auditable
(FR-104, constitution §Governance).

---

```protobuf
syntax = "proto3";
package v1;

// ── Enums ────────────────────────────────────────────────────────────────────

enum ProposalStatus {
  PROPOSAL_STATUS_UNSPECIFIED = 0;
  PROPOSAL_STATUS_OPEN        = 1;
  PROPOSAL_STATUS_PASSED      = 2;
  PROPOSAL_STATUS_REJECTED    = 3;
  PROPOSAL_STATUS_SUPERSEDED  = 4;  // replaced by a later proposal
  PROPOSAL_STATUS_WITHDRAWN   = 5;
}

enum ProposalKind {
  PROPOSAL_KIND_UNSPECIFIED          = 0;
  PROPOSAL_KIND_CONSTITUTION_PATCH   = 1;  // amend constitution (MAJOR/MINOR/PATCH)
  PROPOSAL_KIND_POLICY_CHANGE        = 2;  // AUP, scheduling policy, credit formula
  PROPOSAL_KIND_PARAMETER_CHANGE     = 3;  // tunable threshold (e.g. replica count)
  PROPOSAL_KIND_MEMBERSHIP           = 4;  // TSC / board membership change
  PROPOSAL_KIND_SECURITY_DISCLOSURE  = 5;  // public P0 incident disclosure
  PROPOSAL_KIND_BUDGET               = 6;  // fund allocation decision
}

enum VoteChoice {
  VOTE_CHOICE_UNSPECIFIED = 0;
  VOTE_CHOICE_YES         = 1;
  VOTE_CHOICE_NO          = 2;
  VOTE_CHOICE_ABSTAIN     = 3;
}

enum EligibilityGroup {
  ELIGIBILITY_GROUP_UNSPECIFIED   = 0;
  ELIGIBILITY_GROUP_TSC           = 1;   // Technical Steering Committee
  ELIGIBILITY_GROUP_BOARD         = 2;   // Board of Directors
  ELIGIBILITY_GROUP_CONTRIBUTORS  = 3;   // active contributors (merit-based)
  ELIGIBILITY_GROUP_DONORS        = 4;   // donors above $100 cumulative
}

// ── Messages ─────────────────────────────────────────────────────────────────

message VoteTally {
  uint64 yes_votes      = 1;
  uint64 no_votes       = 2;
  uint64 abstain_votes  = 3;
  uint64 eligible_count = 4;   // total eligible voters at proposal creation
  double quorum_pct     = 5;   // participation as a fraction of eligible_count
  double threshold_pct  = 6;   // required for passage (e.g. 0.667 for supermajority)
}

message Proposal {
  string            proposal_id       = 1;
  ProposalKind      kind              = 2;
  ProposalStatus    status            = 3;
  string            title             = 4;
  string            description       = 5;   // markdown; max 8 KiB
  string            author_account_id = 6;
  int64             created_at_ms     = 7;
  int64             voting_opens_ms   = 8;
  int64             voting_closes_ms  = 9;
  EligibilityGroup  eligible_group    = 10;
  VoteTally         tally             = 11;
  string            ledger_entry_cid  = 12;  // CIDv1 of ledger record
  string            supersedes_id     = 13;  // if kind=CONSTITUTION_PATCH
  bytes             diff_cid          = 14;  // CIDv1 of proposed diff artifact
}

message Vote {
  string       vote_id         = 1;
  string       proposal_id     = 2;
  string       account_id      = 3;
  VoteChoice   choice          = 4;
  string       rationale       = 5;   // optional; max 1 KiB markdown
  bytes        signature       = 6;   // Ed25519 over (proposal_id || choice || account_id)
  int64        cast_at_ms      = 7;
  string       ledger_entry_cid= 8;
}

message Report {
  string   report_id          = 1;
  string   title              = 2;
  string   period             = 3;   // e.g. "2026-Q1"
  string   kind               = 4;   // "financial" | "compliance" | "incident"
  string   body_cid           = 5;   // CIDv1 of the full report document
  string   summary            = 6;   // max 2 KiB plain text
  int64    published_at_ms    = 7;
  string   ledger_entry_cid   = 8;
}

message ListProposalsRequest {
  ProposalStatus status_filter = 1;   // 0 = all statuses
  ProposalKind   kind_filter   = 2;   // 0 = all kinds
  int32          page_size     = 3;
  string         page_token    = 4;
}

message ListProposalsResponse {
  repeated Proposal proposals       = 1;
  string            next_page_token = 2;
}

message CreateProposalRequest {
  ProposalKind     kind             = 1;
  string           title            = 2;
  string           description      = 3;
  EligibilityGroup eligible_group   = 4;
  int64            voting_opens_ms  = 5;   // 0 = immediately
  int64            voting_closes_ms = 6;   // must be >= opens + 72h
  bytes            diff_cid         = 7;   // required for CONSTITUTION_PATCH
  string           supersedes_id    = 8;   // optional
}

message CreateProposalResponse {
  Proposal proposal = 1;
}

message CastVoteRequest {
  string     proposal_id = 1;
  VoteChoice choice      = 2;
  string     rationale   = 3;
  bytes      signature   = 4;   // Ed25519; coordinator verifies before recording
}

message CastVoteResponse {
  Vote   vote            = 1;
  bool   quorum_reached  = 2;
  bool   threshold_met   = 3;
}

message GetReportRequest {
  string report_id = 1;
}

// ── Service ──────────────────────────────────────────────────────────────────

service GovernanceService {

  // List governance proposals, with optional status and kind filters.
  // No auth required for reading open/passed/rejected proposals.
  // Telemetry: span v1.GovernanceService/ListProposals.
  // Rate limit: GOVERNANCE (30/min/account).
  rpc ListProposals(ListProposalsRequest) returns (ListProposalsResponse);

  // Submit a new governance proposal. Requires governance:write scope.
  // Proposal is recorded in the Merkle ledger upon creation (FR-104).
  // Constitutional amendments (kind=CONSTITUTION_PATCH) require a diff_cid
  // and are subject to a Sync Impact Report review process.
  // Telemetry: span v1.GovernanceService/CreateProposal;
  //            ledger entry: PROPOSAL_CREATED.
  // Rate limit: GOVERNANCE.
  // Auth: governance:write scope.
  rpc CreateProposal(CreateProposalRequest) returns (CreateProposalResponse);

  // Cast a vote on an open proposal. The caller's Ed25519 signature over
  // (proposal_id || choice || account_id) is recorded in the ledger.
  // Duplicate votes are rejected (ALREADY_EXISTS). Voting outside the
  // open window is rejected (PERMISSION_DENIED).
  // Telemetry: span v1.GovernanceService/CastVote;
  //            ledger entry: VOTE_CAST.
  // Rate limit: GOVERNANCE.
  // Auth: governance:write scope; caller must be in the eligible_group.
  rpc CastVote(CastVoteRequest) returns (CastVoteResponse);

  // Retrieve a governance report (financial, compliance, or incident disclosure).
  // Reports are referenced by report_id; the full body is fetched via body_cid
  // from the content-addressed data plane.
  // Telemetry: span v1.GovernanceService/GetReport.
  // Rate limit: GOVERNANCE.
  // Auth: governance:read scope (public for compliance/incident reports).
  rpc GetReport(GetReportRequest) returns (Report);
}
```

---

## Example Request / Response

**CastVote** — TSC member voting on a parameter change:

```json
// Request
{
  "proposal_id": "prop_a1b2c3",
  "choice": "VOTE_CHOICE_YES",
  "rationale": "Increasing default replica count from 3 to 5 improves quorum robustness.",
  "signature": "<base64 Ed25519 sig>"
}

// Response
{
  "vote": {
    "vote_id": "vote_x9y8z7",
    "proposal_id": "prop_a1b2c3",
    "account_id": "acct_tsc_member",
    "choice": "VOTE_CHOICE_YES",
    "cast_at_ms": 1776499200000,
    "ledger_entry_cid": "bafybeivoteledgercid"
  },
  "quorum_reached": false,
  "threshold_met": false
}
```
