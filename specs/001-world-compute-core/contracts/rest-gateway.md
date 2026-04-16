# REST/JSON Gateway Mapping

The REST gateway is generated from the same protobuf schema as the gRPC services
(grpc-gateway pattern). Every RPC has a canonical HTTP method + path + body mapping.
The gateway endpoint is `https://<coordinator>:7080/v1/...`.

All requests and responses are JSON. Proto3 field names map to camelCase JSON keys
(standard proto3 JSON encoding). Enum values serialize as their string names.

**Authentication**: Pass `Authorization: Bearer <token>` for OAuth2, or present a
client certificate for mTLS. Public endpoints (marked below) require no auth header.

**Errors**: All errors follow the canonical error envelope in `errors.md`:
```json
{ "code": "ERROR_CODE_NAME", "message": "human-readable detail", "details": {} }
```

---

## DonorService

### POST /v1/donor/enroll

Maps to `DonorService.Enroll`. Auth: mTLS agent cert.

```bash
curl -X POST https://coordinator.worldcompute.org:7080/v1/donor/enroll \
  --cert /etc/worldcompute/agent.crt \
  --key  /etc/worldcompute/agent.key \
  --cacert /etc/worldcompute/wc-ca.crt \
  -H 'Content-Type: application/json' \
  -d '{
    "peerId": "12D3KooWR7bHxkjFe2q...",
    "capacity": {
      "cpuVcores": 8, "memoryMib": 16384, "storageGib": 200,
      "hasGpu": false, "caliberClass": "class-1-cpu"
    },
    "attestation": { "tier": "TRUST_TIER_T2", "agentVersion": "0.1.0" },
    "optedInClasses": ["WORKLOAD_CLASS_SCIENTIFIC"],
    "shardCategories": ["public"],
    "scheduleCron": "0 22 * * *"
  }'
```

Response `200 OK`:
```json
{
  "accountId": "acct_a1b2c3d4",
  "nodeId": "node_eu1_f5e6",
  "clusterId": "wc-global",
  "creditBalance": 0
}
```

---

### POST /v1/donor/heartbeat

Maps to `DonorService.Heartbeat`. Auth: mTLS. Called every ≤30 s by agent.

```bash
curl -X POST https://coordinator.worldcompute.org:7080/v1/donor/heartbeat \
  --cert /etc/worldcompute/agent.crt --key /etc/worldcompute/agent.key \
  --cacert /etc/worldcompute/wc-ca.crt \
  -H 'Content-Type: application/json' \
  -d '{
    "nodeId": "node_eu1_f5e6",
    "status": "DONOR_STATUS_IDLE",
    "cpuUtilPct": 2.1,
    "memUtilPct": 45.3,
    "gpuUtilPct": 0.0,
    "uptimeSecs": 86400,
    "activeLease": ""
  }'
```

Response `200 OK`:
```json
{ "shouldPause": false, "controlMessage": "", "creditBalance": 12847000 }
```

---

### GET /v1/donor/status/{nodeId}

Maps to `DonorService.GetDonorStatus`. Auth: mTLS. **Public** for own node only.

```bash
curl https://coordinator.worldcompute.org:7080/v1/donor/status/node_eu1_f5e6 \
  --cert /etc/worldcompute/agent.crt --key /etc/worldcompute/agent.key \
  --cacert /etc/worldcompute/wc-ca.crt
```

Response `200 OK`:
```json
{
  "accountId": "acct_a1b2c3d4",
  "nodeId": "node_eu1_f5e6",
  "status": "DONOR_STATUS_IDLE",
  "trustTier": "TRUST_TIER_T2",
  "ncuEarned": 12847000,
  "ncuSpent": 420000,
  "jobsRun": 847,
  "jobsVerified": 841,
  "jobsDisputed": 6,
  "uptimeSecs": 86400
}
```

---

## SubmitterService

### POST /v1/jobs

Maps to `SubmitterService.SubmitJob`. Auth: OAuth2 `submitter:write`.

```bash
curl -X POST https://coordinator.worldcompute.org:7080/v1/jobs \
  -H 'Authorization: Bearer eyJ...' \
  -H 'Content-Type: application/json' \
  -d '{
    "manifest": {
      "name": "hello-sha256",
      "imageCid": "oci+cid:bafybeihashofalpinewithsha256utils",
      "command": ["sha256sum", "/input/data.bin"],
      "inputs": [{ "cid": "bafybeig3k7inputdatacid", "mount": "/input/data.bin" }],
      "outputs": [{ "name": "result", "path": "/output/result.txt" }],
      "resources": { "cpuVcores": 1, "memoryMib": 512 },
      "priority": "PRIORITY_CLASS_PUBLIC_GOOD",
      "replicaCount": 3,
      "acceptableUse": "AUC_SCIENTIFIC"
    }
  }'
```

Response `202 Accepted`:
```json
{
  "jobId": "job_8f9c2a4b1e",
  "manifestCid": "bafybeimanifestcid",
  "phase": "JOB_PHASE_QUEUED",
  "estimatedQueueSecs": 45,
  "ncuEstimated": 420
}
```

---

### GET /v1/jobs/{jobId}

Maps to `SubmitterService.GetJob`. Auth: OAuth2 `submitter:read`.

```bash
curl https://coordinator.worldcompute.org:7080/v1/jobs/job_8f9c2a4b1e \
  -H 'Authorization: Bearer eyJ...'
```

Response `200 OK`:
```json
{
  "jobId": "job_8f9c2a4b1e",
  "phase": "JOB_PHASE_VERIFIED",
  "resultCid": "bafybeig3k7resultcid",
  "receiptHash": "sha256:e3b0c44298...",
  "rekorEntryId": "3f8c9d2a1b4e",
  "ncuCharged": 420,
  "submittedAt": 1776499200000,
  "verifiedAt": 1776499560000
}
```

---

### GET /v1/jobs/{jobId}/logs

Maps to `SubmitterService.StreamJobLogs`. Auth: OAuth2 `submitter:read`.
Uses HTTP chunked transfer encoding (not WebSocket) for the streaming response.
Each chunk is a newline-delimited JSON object (`LogLine`).

```bash
curl -N https://coordinator.worldcompute.org:7080/v1/jobs/job_8f9c2a4b1e/logs \
  -H 'Authorization: Bearer eyJ...'
```

Response `200 OK` (chunked stream, one JSON object per line):
```
{"timestampMs":1776499210000,"replicaId":"replica_0","stream":"stdout","text":"e3b0c44298fc1c149a...  /input/data.bin\n"}
{"timestampMs":1776499210100,"replicaId":"replica_1","stream":"stdout","text":"e3b0c44298fc1c149a...  /input/data.bin\n"}
```

---

### DELETE /v1/jobs/{jobId}

Maps to `SubmitterService.CancelJob`. Auth: OAuth2 `submitter:write`.

```bash
curl -X DELETE "https://coordinator.worldcompute.org:7080/v1/jobs/job_8f9c2a4b1e?reason=user+cancelled" \
  -H 'Authorization: Bearer eyJ...'
```

Response `200 OK`:
```json
{ "accepted": true, "terminalPhase": "JOB_PHASE_CANCELLED" }
```

---

### GET /v1/jobs/{jobId}/result/{outputName}

Maps to `SubmitterService.FetchResult`. Auth: OAuth2 `submitter:read`.

```bash
curl "https://coordinator.worldcompute.org:7080/v1/jobs/job_8f9c2a4b1e/result/result" \
  -H 'Authorization: Bearer eyJ...'
```

Response `200 OK` (inline for small results):
```json
{
  "resultCid": "bafybeig3k7resultcid",
  "data": "ZTNiMGM0NDI5OGZjMWMxNDlhZmJmNGM4...",
  "receiptHash": "sha256:e3b0c44298...",
  "rekorEntryId": "3f8c9d2a1b4e"
}
```

For results > 4 MiB, `data` is empty and `downloadUrl` contains a pre-signed URL.

---

## ClusterService

### GET /v1/cluster/status

Maps to `ClusterService.GetClusterStatus`. **No auth required** — public endpoint.

```bash
curl https://coordinator.worldcompute.org:7080/v1/cluster/status
```

Response `200 OK`:
```json
{
  "health": "CLUSTER_HEALTH_HEALTHY",
  "clusterId": "wc-global",
  "version": "0.1.0",
  "capacity": {
    "totalNodesEnrolled": 14823,
    "nodesActive": 9241,
    "totalCpuVcores": 147856,
    "jobsRunning": 3042,
    "jobsQueued": 187
  },
  "ledgerHeadCid": "bafybeimerklerootcid",
  "rekorEntryId": "3f8c9d2a1b4e"
}
```

---

### GET /v1/cluster/ledger/head

Maps to `ClusterService.GetLedgerHead`. **No auth required** — public endpoint.

```bash
curl https://coordinator.worldcompute.org:7080/v1/cluster/ledger/head
```

Response `200 OK`:
```json
{
  "cid": "bafybeimerklerootcid",
  "sequence": 1048576,
  "rekorEntryId": "3f8c9d2a1b4e",
  "anchoredAtMs": 1776499200000,
  "sigThreshold": 4,
  "sigParticipants": 7
}
```

---

## GovernanceService

### GET /v1/governance/proposals

Maps to `GovernanceService.ListProposals`. **No auth required** for open proposals.

```bash
curl "https://coordinator.worldcompute.org:7080/v1/governance/proposals?statusFilter=PROPOSAL_STATUS_OPEN"
```

Response `200 OK`:
```json
{
  "proposals": [{
    "proposalId": "prop_a1b2c3",
    "kind": "PROPOSAL_KIND_PARAMETER_CHANGE",
    "status": "PROPOSAL_STATUS_OPEN",
    "title": "Increase default replica count from 3 to 5",
    "votingClosesMs": 1776758400000,
    "tally": { "yesVotes": 3, "noVotes": 1, "abstainVotes": 0, "eligibleCount": 7 }
  }]
}
```

---

## Error Response Example

Any endpoint returns a structured error on failure:

```bash
curl -X POST https://coordinator.worldcompute.org:7080/v1/jobs \
  -H 'Authorization: Bearer eyJ...' \
  -d '{ "manifest": { "imageCid": "" } }'
```

Response `400 Bad Request`:
```json
{
  "code": "INVALID_MANIFEST",
  "message": "manifest.image_cid is required and must be a valid CIDv1 URI",
  "details": { "field": "manifest.image_cid" }
}
```

---

## HTTP Status Code Mapping

| gRPC status | HTTP status | Notes |
|-|-|-|
| OK | 200 / 202 | 202 for async operations (SubmitJob) |
| INVALID_ARGUMENT | 400 | |
| UNAUTHENTICATED | 401 | |
| PERMISSION_DENIED | 403 | |
| NOT_FOUND | 404 | |
| ALREADY_EXISTS | 409 | |
| RESOURCE_EXHAUSTED | 429 | Rate limited |
| INTERNAL | 500 | |
| UNAVAILABLE | 503 | |
| DEADLINE_EXCEEDED | 504 | |
