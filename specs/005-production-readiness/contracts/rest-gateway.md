# Contract: REST Gateway HTTP endpoints

**Scope**: spec 005 binds a real HTTP listener (FR-041) in the daemon when configured. Endpoints were designed in spec 004 but never served. This contract locks the v1 surface.

## Base URL

- `http://127.0.0.1:<port>/v1/` by default (port is `8443` for TLS, `8080` for plain HTTP, configurable)
- TLS via the agent's mTLS certificate from spec 004 (FR-047 in that spec)
- Auth: Ed25519-signed JWT in `Authorization: Bearer <token>` header

## Endpoints

| Method | Path | Request body | Response | Auth | Source FR |
|-|-|-|-|-|-|
| `GET` | `/v1/health` | — | `{"status": "ok", "version": "...", "peer_id": "..."}` | none | baseline |
| `GET` | `/v1/status` | — | `{connections: N, reservations: [...], load: {cpu, gpu, mem}}` | agent-token | FR-033 |
| `POST` | `/v1/jobs` | `{workload_cid, executor_peer_id?, ...}` | `{job_id}` | submitter-token | baseline |
| `GET` | `/v1/jobs/{job_id}` | — | `{job_id, status, receipt?}` | submitter-token | baseline |
| `POST` | `/v1/diffusion/infer` | `InferRequest` (JSON mirror of proto) | `{request_id}` + SSE stream at `/v1/diffusion/stream/{request_id}` | submitter-token | FR-027 |
| `GET` | `/v1/diffusion/stream/{request_id}` | — | SSE with `InferResponse` events | submitter-token | FR-027 |
| `POST` | `/v1/admin/firewall-diagnose` | `{duration_s}` | `{evidence_path}` | maintainer-token | US1 support |
| `POST` | `/v1/admin/drift-check` | — | `DriftCheckResult` JSON | maintainer-token | FR-011a |
| `GET` | `/v1/admin/placeholder-status` | — | `{allowlist_empty: bool, entries: [...]}` | anyone | FR-038, SC-006 |

## Error format

All 4xx/5xx responses follow RFC 7807 Problem Details:
```json
{
  "type": "https://worldcompute.org/errors/reservation-failed",
  "title": "Relay reservation could not be acquired",
  "status": 503,
  "detail": "All 5 bootstrap relays rejected reservation request; see logs for per-relay cause",
  "instance": "/v1/admin/firewall-diagnose"
}
```

## Rate limiting

Each endpoint is rate-limited per token (from spec 004's rate-limit subsystem). Defaults:
- `/v1/health`: 100/s
- `/v1/jobs` (POST): 10/s per token
- `/v1/diffusion/infer`: 1/s per token (inference is expensive)
- admin endpoints: 1/min

## Stability

All endpoints in v1 are stable once shipped. Breaking changes require a new `/v2/` prefix.
