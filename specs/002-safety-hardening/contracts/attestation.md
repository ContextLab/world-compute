# Contract: Attestation Verification

**Date**: 2026-04-16
**Scope**: FR-S010, FR-S011, FR-S012, FR-S013

## Interface

Attestation verification is called during donor node enrollment and
periodically at trust score recalculation. It determines the trust tier
(T0–T4) based on hardware attestation evidence.

### Verify Node Attestation

**Input**: Attestation quote (platform-specific binary blob) + claimed
hardware capabilities

**Output**: `AttestationResult` — verified trust tier + expiration

**Verification by platform**:

1. **TPM2 (T1/T2)**: Validate PCR measurements against known-good values
   for the current signed agent build. Verify the TPM endorsement key
   chain. Reject if PCR values don't match or endorsement chain is invalid.

2. **SEV-SNP (T3)**: Validate the attestation report against AMD's
   root-of-trust certificate (ARK → ASK → VCEK chain). Verify the
   measurement matches the expected guest image. Reject if chain is
   invalid or measurement mismatches.

3. **TDX (T3)**: Validate the TDX quote against Intel's root-of-trust
   certificates. Verify MRTD and RTMR measurements. Reject if chain is
   invalid or measurements mismatch.

4. **H100 CC (T4)**: Validate NVIDIA confidential compute attestation.
   Verify GPU firmware measurements.

5. **Soft attestation / no quote (T0)**: No hardware verification. Node
   is classified as T0 — restricted to WASM-only, public data, R>=5
   replicas.

### Error semantics

- Empty quote → T0 classification (safe default, not an error)
- Invalid quote → rejection with specific error (invalid chain, PCR
  mismatch, expired certificate)
- Valid quote for claimed tier → verified classification

### Rejection behavior

A node presenting an invalid quote for a claimed tier is NOT silently
downgraded to T0. The attestation fails with an error, and the node
must re-enroll with correct attestation or accept T0 classification
explicitly. This prevents a compromised node from operating at T0
while claiming higher capabilities.

### Re-verification

Attestation is verified at enrollment and re-verified at trust score
recalculation intervals (per clarification session 2026-04-16). If
re-verification fails, the node's trust tier is downgraded and in-flight
jobs are checkpointed and rescheduled to appropriate-tier nodes.

### Agent build verification

The known-good PCR values are published alongside each signed agent
release. The coordinator maintains a mapping of agent version → expected
PCR measurements. Only the current release and one prior release are
accepted (rolling window for upgrade transitions).
