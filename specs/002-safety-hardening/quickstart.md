# Quickstart: Safety Hardening

## What this feature does

Closes enforcement gaps in World Compute's safety infrastructure:
attestation verification, sandbox isolation, network egress, governance
separation, incident response, and identity verification. All changes
preserve the project's constitutional model as an open volunteer compute
federation.

## Key concepts

- **Policy engine**: Deterministic gate wrapping `validate_manifest()`.
  Every job passes through it before scheduling. LLM review is advisory.
- **Attestation enforcement**: TPM2/SEV-SNP/TDX verification replaces
  stubs. Nodes are classified T0–T4 based on real hardware evidence.
- **Default-deny egress**: Sandbox-level firewall blocks all outbound
  traffic unless the manifest declares approved endpoints.
- **Separation of duties**: No single identity can approve a workload
  class, sign the artifact, AND deploy policy changes.
- **Incident containment**: Freeze, quarantine, block, revoke, and drain
  actions with full audit trails.

## Development workflow

1. **Start with Phase 1** (attestation) — everything else depends on it
2. **Phase 2** (sandbox) requires Phase 1 for artifact verification
3. **Phase 3** (policy engine) requires Phases 1+2
4. **Phase 6** (identity) can run in parallel with Phases 3-5
5. Run `cargo test && cargo clippy` after each phase

## Testing requirements

Per Constitution Principle V, every component must be directly tested on
real hardware. Key test scenarios:

- Forged TPM2 quotes must be rejected (Phase 1)
- Outbound connections from sandboxed jobs must be blocked (Phase 2)
- All-zero manifest signatures must be rejected (Phase 1)
- Single-actor governance violations must be detected (Phase 4)
- Containment actions must complete within 60 seconds (Phase 5)

## Files to know

| File | Purpose |
|-|-|
| src/verification/attestation.rs | Attestation stubs to replace |
| src/sandbox/*.rs | VM drivers to implement |
| src/scheduler/manifest.rs | Existing validate_manifest() — preserved, wrapped |
| src/governance/voting.rs | Quorum thresholds to differentiate |
| src/governance/admin_service.rs | halt() auth to add |

## New modules to create

| Module | Purpose |
|-|-|
| src/policy/ | Deterministic policy engine |
| src/incident/ | Incident response containment |
| src/identity/ | Humanity Points verification flows |
| src/registry/ | Approved artifact registry |
| src/sandbox/egress.rs | Network egress enforcement |
| src/governance/roles.rs | GovernanceRole entity |
