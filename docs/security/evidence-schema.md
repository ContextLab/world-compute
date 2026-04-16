# Direct-Test Evidence Artifact Schema

Version: 1.0.0  
Status: Draft  
Reference: FR-082

## Overview

Every security direct-test produces a structured evidence artifact stored as
a CIDv1-addressed JSON document. This document defines the canonical schema
for those artifacts.

## JSON Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12",
  "$id": "https://world-compute.org/schemas/evidence/v1.json",
  "title": "SecurityEvidenceArtifact",
  "type": "object",
  "required": [
    "schema_version",
    "evidence_cid",
    "test_id",
    "test_name",
    "test_class",
    "outcome",
    "timestamp_utc",
    "runner_node_id",
    "environment"
  ],
  "properties": {
    "schema_version": {
      "type": "string",
      "const": "1.0.0",
      "description": "Schema version. Increment minor for additive changes, major for breaking."
    },
    "evidence_cid": {
      "type": "string",
      "description": "CIDv1 (SHA-256, raw codec) of this document's canonical bytes."
    },
    "test_id": {
      "type": "string",
      "description": "Unique identifier for the test case (e.g. T137)."
    },
    "test_name": {
      "type": "string",
      "description": "Human-readable name matching the #[test] fn name."
    },
    "test_class": {
      "type": "string",
      "enum": ["sandbox_escape", "network_isolation", "byzantine_donor", "flood_resilience", "other"],
      "description": "Category of security property being tested."
    },
    "outcome": {
      "type": "string",
      "enum": ["pass", "fail", "skip", "error"],
      "description": "Result of the test execution."
    },
    "failure_detail": {
      "type": "string",
      "description": "Human-readable description of failure (present when outcome = fail | error)."
    },
    "timestamp_utc": {
      "type": "string",
      "format": "date-time",
      "description": "ISO-8601 UTC timestamp when the test completed."
    },
    "runner_node_id": {
      "type": "string",
      "description": "PeerId of the node that executed the test."
    },
    "build_info": {
      "type": "object",
      "description": "Build metadata from BuildInfo struct.",
      "properties": {
        "version":         { "type": "string" },
        "git_sha":         { "type": "string" },
        "build_timestamp": { "type": "string" },
        "is_signed":       { "type": "boolean" }
      }
    },
    "environment": {
      "type": "object",
      "description": "Runtime environment details.",
      "required": ["os", "arch", "kernel"],
      "properties": {
        "os":     { "type": "string" },
        "arch":   { "type": "string" },
        "kernel": { "type": "string" },
        "tee":    { "type": "string", "description": "TEE type if present (sev-snp, tdx, tpm2, none)." }
      }
    },
    "attestation": {
      "type": "object",
      "description": "Optional TEE attestation quote covering this evidence document.",
      "properties": {
        "quote_type":  { "type": "string" },
        "quote_bytes": { "type": "string", "contentEncoding": "base64" }
      }
    },
    "linked_incident": {
      "type": "string",
      "description": "Optional reference to an incident report if this test was triggered by a live event."
    }
  },
  "additionalProperties": false
}
```

## Example Artifact

```json
{
  "schema_version": "1.0.0",
  "evidence_cid": "bafkreihdwdcef...",
  "test_id": "T137",
  "test_name": "sandbox_read_etc_passwd",
  "test_class": "sandbox_escape",
  "outcome": "pass",
  "timestamp_utc": "2026-04-15T12:00:00Z",
  "runner_node_id": "12D3KooW...",
  "build_info": {
    "version": "0.1.0",
    "git_sha": "abc1234",
    "build_timestamp": "2026-04-15T10:00:00Z",
    "is_signed": true
  },
  "environment": {
    "os": "linux",
    "arch": "x86_64",
    "kernel": "6.8.0",
    "tee": "sev-snp"
  }
}
```

## Storage and Verification

1. The artifact is serialized as canonical JSON (keys sorted, no trailing whitespace).
2. Its CIDv1 is computed and stored in `evidence_cid`.
3. The artifact is pinned to the World Compute CID store with a 7-year retention tag.
4. The `evidence_cid` is submitted to the governance ledger for auditability.
