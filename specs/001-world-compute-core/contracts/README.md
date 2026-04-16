# World Compute v1 API Contracts — Index

**Package prefix**: `v1`
**Proto source of truth**: All service definitions live in `contracts/*.proto.md` as
protobuf-style pseudocode; generated clients (Rust, Python, TypeScript) are derived
from canonical `.proto` files that must match these sketches exactly.
**License**: Apache 2.0 (FR-099)
**Implementation language**: Rust (FR-006)

---

## Services

| File | Service | Method count | Auth scope |
|-|-|-|-|
| `donor.proto.md` | `DonorService` | 6 | mTLS (agent cert) |
| `submitter.proto.md` | `SubmitterService` | 6 | mTLS or OAuth2 bearer |
| `cluster.proto.md` | `ClusterService` | 4 | mTLS or OAuth2 bearer (read-only public) |
| `governance.proto.md` | `GovernanceService` | 4 | mTLS or OAuth2 bearer |
| `admin.proto.md` | `AdminService` | 4 | mTLS + admin role claim |

REST/JSON gateway mappings: `rest-gateway.md`
Canonical error model: `errors.md`

---

## Transport

All services are exposed over **gRPC** (HTTP/2, proto3 encoding) as the primary
transport. A **REST/HTTP+JSON gateway** (grpc-gateway style) is generated from the
same protobuf schema so the CLI, web dashboard, and third-party integrations share
one contract (FR-093). The gateway translates HTTP methods + paths to gRPC calls and
handles JSON ↔ proto transcoding transparently.

**Ports** (all TLS-only; plaintext is rejected):

| Port | Protocol | Purpose |
|-|-|-|
| 7443 | gRPC/TLS | All services, mTLS required |
| 7080 | HTTPS | REST gateway + web dashboard API |
| 7444 | gRPC/TLS | Admin service (firewall-restricted) |

**TLS version**: TLS 1.3 minimum. TLS 1.2 is rejected.

---

## Authentication

### mTLS — Agent and CLI

The primary authentication mechanism for agents and CLI. Every enrolled donor node
receives a per-account **Ed25519 certificate** issued by the World Compute PKI (an
intermediate CA rooted in a hardware-backed offline root). The certificate's Subject
contains the account ID (`wc-account:<uuid>`) and the node peer ID
(`wc-peer:<peer-id>`). The coordinator validates the client certificate on every
connection and rejects any cert not issued by a recognized intermediate CA.

Agents authenticate exclusively via mTLS. There is no username/password fallback for
the agent protocol. Certificate rotation is handled automatically by the agent every
90 days via ACME-like protocol against the coordinator's PKI endpoint.

### OAuth2 Bearer Token — Web Dashboard and Third-Party

The web dashboard (React SPA, FR-092) and API integrators authenticate via **OAuth2**
with the World Compute authorization server. Supported grant types: Authorization Code
+ PKCE (web dashboard), Client Credentials (machine-to-machine integrations). Tokens
are short-lived JWTs (15-minute expiry) with a refresh token (7-day sliding window).

Scopes:
- `donor:read` — read own donor status, credits, job history
- `donor:write` — enroll, configure, pause, withdraw
- `submitter:read` — read jobs and results
- `submitter:write` — submit and cancel jobs
- `cluster:read` — cluster status, ledger head (public, no auth required for read)
- `governance:read` — proposals, reports
- `governance:write` — create proposals, cast votes
- `admin` — AdminService (requires explicit role grant; not grantable via OAuth2 flow)

### Admin Authentication

AdminService methods additionally require a role claim `wc-role: admin` in the mTLS
certificate Subject. Admin certificates are issued separately from account certificates,
stored in hardware (HSM or YubiKey), and rotated quarterly. No OAuth2 token ever
carries admin scope.

---

## Versioning

All services live under the `v1` package prefix (proto `package v1;`). Field numbers
and RPC names are **additive-only** once the v1 API is declared stable: no field may
be removed or renumbered; no RPC may be removed. Backward-incompatible changes require
a new major version (`v2`) with a defined migration window (minimum 12 months overlap).

New optional fields added to any message receive a new field number and default to the
zero value for their type. Clients MUST ignore unknown fields (proto3 default behavior).

The REST gateway path prefix includes the version: `/v1/...`

---

## Telemetry

Every RPC produces an **OpenTelemetry trace span** (FR-105). Span names follow the
convention `v1.<ServiceName>/<MethodName>`. Spans carry:
- `wc.account_id` — hashed (not raw) account identifier
- `wc.job_id` — job ID where applicable
- `wc.rpc.status` — gRPC status code
- `wc.rpc.rate_limit_class` — rate limit bucket applied

Telemetry MUST NOT include donor PII, submitter job contents, or host-identifying
information (hostnames, local IPs, usernames, MAC addresses) per FR-106. Redaction is
enforced at the emit layer and is a release gate (FR-106).

---

## Rate Limit Classes

| Class | Limit | Applies to |
|-|-|-|
| `DONOR_HEARTBEAT` | 120 req/min/node | DonorService.Heartbeat |
| `JOB_SUBMIT` | 10 req/min/account | SubmitterService.SubmitJob |
| `JOB_READ` | 300 req/min/account | GetJob, ListJobs, FetchResult |
| `STREAM` | 5 concurrent/account | StreamJobLogs |
| `GOVERNANCE` | 30 req/min/account | GovernanceService all methods |
| `ADMIN` | 60 req/min/admin | AdminService all methods |
| `CLUSTER_READ` | 600 req/min (global) | ClusterService read methods |
