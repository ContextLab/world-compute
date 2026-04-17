# Data Model: Full Functional Implementation

**Date**: 2026-04-17 | **Spec**: [spec.md](spec.md)

## New and Modified Entities

### AttestationChain (modified — src/verification/attestation.rs)

Extends existing certificate chain structures with cryptographic verification fields.

- `platform_type`: Tpm2 | SevSnp | Tdx | AppleSe
- `leaf_cert`: DER-encoded leaf certificate bytes
- `intermediate_certs`: Vec of DER-encoded intermediate certificate bytes
- `root_cert_fingerprint`: [u8; 32] — SHA-256 of root CA DER encoding
- `signature_algorithm`: Rsa2048 | Rsa4096 | EcdsaP256 | EcdsaP384
- `verified`: bool — set after full cryptographic chain verification
- `verification_timestamp`: Timestamp

**Validation**: Root fingerprint must match pinned constant for platform type. All intermediate signatures must chain to root. No expired certificates. Leaf must contain expected OIDs.

### InclusionProof (new — src/ledger/transparency.rs)

- `leaf_hash`: [u8; 32] — SHA-256 of the log entry
- `tree_size`: u64 — size of the tree when proof was generated
- `proof_hashes`: Vec<[u8; 32]> — Merkle path from leaf to root
- `signed_tree_head`: SignedTreeHead { tree_size, root_hash, signature }
- `rekor_public_key`: [u8; 32] — pinned Ed25519 public key

**Validation**: Compute root from leaf_hash + proof_hashes. Compare to signed_tree_head.root_hash. Verify signature with rekor_public_key.

### Lease (modified — src/scheduler/broker.rs)

- `lease_id`: String
- `task_id`: String
- `node_id`: PeerId
- `issued_at`: Timestamp
- `ttl_ms`: u64
- `renewed_at`: Option<Timestamp>
- `status`: Active | Expired | Released

**State transitions**: Active → (heartbeat) → Active (renewed_at updated) | Active → (ttl exceeded) → Expired | Active → (task complete) → Released

### ContainmentAction (modified — src/incident/containment.rs)

- `action_type`: FreezeHost | QuarantineWorkloadClass | BlockSubmitter | RevokeArtifact | DrainHostPool
- `target`: String — host ID, workload class, submitter ID, artifact CID, or host pool ID
- `actor`: PeerId — who authorized the action
- `justification`: String
- `reversible`: bool
- `executed`: bool — NEW: whether enforcement effect was applied
- `execution_result`: Option<Result<(), String>> — NEW: outcome of enforcement

### ConfidentialBundle (new — src/data_plane/confidential.rs)

- `ciphertext_cid`: Cid — CID of encrypted data in store
- `cipher`: Aes256Gcm
- `nonce`: [u8; 12]
- `wrapped_key`: Vec<u8> — ephemeral key wrapped with submitter's public key
- `confidentiality_level`: Medium | High
- `attestation_requirement`: Option<GuestMeasurement> — for High level

### CreditDecayEvent (new — src/credits/decay.rs)

- `account_id`: PeerId
- `balance_before`: NcuAmount
- `balance_after`: NcuAmount
- `decay_rate`: f64 — effective rate (may be elevated for anti-hoarding)
- `floor`: NcuAmount — trailing_30d_earn_rate * 30
- `timestamp`: Timestamp

### MeshExpert (new — src/agent/mesh_llm/expert.rs)

- `expert_id`: PeerId
- `model_name`: String — e.g. "llama-3-8b-q4"
- `tokenizer`: String — must be "llama3" for compatibility
- `vram_mb`: u32
- `max_batch_size`: u32
- `health`: Healthy | Degraded | Offline
- `last_heartbeat`: Timestamp
- `latency_p50_ms`: u32

### ActionTier (new — src/agent/mesh_llm/safety.rs)

- `tier`: ReadOnly | Suggest | SandboxTest | DeployMinor | DeployMajor
- `approval_required`: None | HumanReview | AutomatedValidation | GovernanceQuorum(u32, u32) | FullVoteWithReview(Duration)

### EgressAllowlist (new — src/policy/rules.rs)

- `approved_endpoints`: Vec<EndpointPattern> — e.g. "*.example.com:443", "192.168.1.0/24:8080"
- `default_action`: Deny (always — per spec)

### StorageCap (new — src/data_plane/cid_store.rs)

- `node_id`: PeerId
- `cap_bytes`: u64
- `used_bytes`: u64
- `last_gc_at`: Timestamp

## Modified Existing Entities

### JobManifest (src/scheduler/manifest.rs)

Add fields:
- `allowed_endpoints`: Vec<String> — declared egress endpoints for policy validation
- `confidentiality_level`: Option<ConfidentialityLevel> — None | Medium | High

### AgentState (src/agent/mod.rs)

No structural change. Wire state transitions to real lifecycle operations (heartbeat loop, checkpoint on pause, cleanup on withdraw).

### PolicyDecision (src/policy/engine.rs)

Add field:
- `artifact_registry_result`: Option<ArtifactLookupResult> — result of CID lookup
- `egress_validation_result`: Option<EgressValidationResult> — result of endpoint check
