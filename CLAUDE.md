# world-compute Development Guidelines

Last updated: 2026-04-17

## Project Overview

World Compute is a decentralized, volunteer-built compute federation. The codebase is a Rust workspace with 150+ source files, 784+ passing tests, and 20 library modules. All 5 CLI command groups are functional (donor, job, cluster, governance, admin). Core modules implemented: WASM sandbox with CID store integration, real Ed25519 signature verification, certificate chain validation (TPM2/SEV-SNP/TDX), BrightID/OAuth2/phone identity verification, Sigstore Rekor transparency logging, OTLP telemetry, STUN-based NAT detection, Raft coordinator consensus, and Firecracker/Apple VF sandbox drivers.

## Active Technologies
- Rust stable (tested on 1.95.0) + libp2p 0.54, tonic 0.12, ed25519-dalek 2, wasmtime 27, openraft 0.9, opentelemetry 0.27, clap 4 (003-stub-replacement)
- CID-addressed content store (cid 0.11, multihash 0.19), erasure-coded (reed-solomon-erasure 6) (003-stub-replacement)
- Rust stable (tested on 1.95.0) + libp2p 0.54, tonic 0.12, ed25519-dalek 2, wasmtime 27, openraft 0.9, opentelemetry 0.27, clap 4, reqwest 0.12, oauth2 4, x509-parser 0.16, reed-solomon-erasure 6, cid 0.11, multihash 0.19 (004-full-implementation)
- CID-addressed content store (SHA-256), erasure-coded RS(10,18) (004-full-implementation)

- **Language**: Rust (stable, tested on 1.95.0)
- **Networking**: rust-libp2p 0.54 (QUIC, TCP, mDNS, Kademlia, gossipsub)
- **Crypto**: ed25519-dalek 2, sha2 0.10
- **gRPC**: tonic 0.12, prost 0.13
- **Async**: tokio 1 (full features)
- **WASM**: wasmtime 27
- **Serialization**: serde, serde_json, serde_yaml, ciborium (CBOR)
- **Content addressing**: cid 0.11, multihash 0.19
- **Erasure coding**: reed-solomon-erasure 6
- **Consensus**: openraft 0.9
- **Observability**: opentelemetry 0.27, tracing 0.1
- **CLI**: clap 4 (derive)
- **GUI**: Tauri (gui/src-tauri)

## Project Structure

```text
src/                        # 94 Rust source files, 20 modules
  acceptable_use/           # Workload classification and filtering
  agent/                    # Donor agent lifecycle, identity, mesh LLM
  cli/                      # CLI subcommands (donor, job, cluster, governance, admin)
  credits/                  # NCU caliber classes
  data_plane/               # CID store, erasure coding, staging
  governance/               # Proposals, voting, roles, admin service, humanity points
  identity/                 # BrightID personhood, OAuth2, phone verification
  incident/                 # Containment actions, audit records
  ledger/                   # CRDT ledger, transparency anchoring
  network/                  # P2P discovery, gossip, NAT, TLS, rate limiting
  policy/                   # Deterministic policy engine (10-step pipeline)
  preemption/               # Sovereignty events, idle detection, supervisor
  registry/                 # Approved artifact registry, release channels
  sandbox/                  # VM drivers (Firecracker, AppleVF, HyperV, WASM), egress
  scheduler/                # Job/task state machines, broker, coordinator, manifest
  telemetry/                # OpenTelemetry, PII redaction
  verification/             # Attestation (TPM2/SEV-SNP/TDX), trust score, quorum
  error.rs                  # 20 error codes with gRPC + HTTP mapping
  types.rs                  # Core types (Cid, PeerId, NcuAmount, TrustScore, Timestamp)
tests/                      # 44 integration test files
  egress/                   # Default-deny, private ranges, LAN blocking, runtime fetch
  governance/               # Separation of duties, quorum, timelock, admin auth
  identity/                 # Personhood, OAuth2, revocation, uniqueness
  incident/                 # Freeze, quarantine, audit, auth, cascade timing
  policy/                   # Dispatch attestation, artifact check, quota, quarantine, LLM
  sandbox/                  # Isolation, cleanup
  red_team/                 # 5 adversarial scenarios (SC-S008)
proto/                      # 6 gRPC proto files (donor, submitter, cluster, governance, admin, mesh_llm)
specs/
  001-world-compute-core/   # Original spec, plan, research, data model, contracts
  002-safety-hardening/     # Red team response — 110 tasks, all complete
adapters/                   # Slurm, Kubernetes, cloud adapter crates
gui/src-tauri/              # Tauri GUI scaffold
```

## Commands

```sh
# Build and test
cargo test                  # 784+ tests (500+ lib + 284+ integration)
cargo clippy --lib -- -D warnings  # Zero warnings enforced

# Build only
cargo build                 # Builds the worldcompute binary
cargo build --lib           # Library only (faster)

# Run (all 5 CLI command groups functional)
./target/debug/worldcompute --help
```

## Code Style

- Rust stable, standard conventions
- All public items have doc comments (//!)
- Module-level doc comments explain the FR/SC requirements
- Tests are inline (#[cfg(test)]) and in tests/ directory
- Clippy with -D warnings (zero warnings policy)
- No unsafe code

## Architecture Decisions

- **Policy engine wraps validate_manifest()** as one step in a 10-step pipeline (not replaces)
- **Identity verification at enrollment**, re-verified at trust score recalculation intervals
- **BrightID** is the primary proof-of-personhood provider (decentralized, free, no biometrics)
- **Invalid attestation quotes are rejected**, not silently downgraded to T0 (empty quotes downgrade)
- **GovernanceRole default expiration**: 90 days, renewable
- **ConstitutionAmendment time-lock**: 7-day mandatory review period
- **Safety-critical votes**: require HP >= 5 (EmergencyHalt, ConstitutionAmendment)
- **Default-deny network egress** at sandbox level for all platforms
- **Separation of duties**: WorkloadApprover + ArtifactSigner prohibited on same identity
- **Release channels**: dev → staging → production only (no dev → production skip)

## Constitution

The project is governed by a ratified constitution at `.specify/memory/constitution.md` with 5 binding principles:
1. **Safety First** — VM-level isolation, no host access, code-signed agents
2. **Robustness** — erasure coding, checkpoint/resume, self-healing
3. **Fairness & Donor Sovereignty** — sub-second preemption, credit reciprocity
4. **Efficiency & Self-Improvement** — energy-aware scheduling, mesh LLM
5. **Direct Testing** — real hardware tests required, no mocks for production

## Remaining Stubs

**None** — all implementation stubs have been replaced as of spec 004-full-implementation. Zero TODO comments remain in src/. Zero `#[ignore]` tests remain.

## CI

Two GitHub Actions workflows:
- `ci.yml` — basic build + test
- `safety-hardening-ci.yml` — multi-platform (Linux/macOS/Windows) with Principle V evidence artifacts

## Recent Changes

- **004-full-implementation** (2026-04-17): Complete functional implementation (#57, #28–#56). 211 tasks, 784+ tests. Deep cryptographic attestation, agent lifecycle, preemption supervisor, policy engine completion, GPU passthrough, Firecracker rootfs, incident containment, adversarial tests, confidential compute, mTLS, threshold signing, CRDT ledger, scheduler matchmaking, credit decay, storage GC, platform adapters (Slurm/K8s/Cloud/Apple VF), Tauri GUI, REST gateway, mesh LLM, Docker/Helm deployment, energy metering.
- **003-stub-replacement** (2026-04-16): Replaced all implementation stubs (#7, #8–#26). 77 tasks, 489+ tests. Added reqwest, oauth2, x509-parser, rcgen dependencies. Wired CLI, sandboxes, attestation, identity, transparency, telemetry, consensus, network.
- **002-safety-hardening** (2026-04-16): Red team review (#4). Policy engine, attestation, governance, incident response, egress, identity hardening. 110 tasks, PR #6.
