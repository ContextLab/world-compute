# world-compute Development Guidelines

Last updated: 2026-04-19

## Project Overview

World Compute is a decentralized, volunteer-built compute federation. The codebase is a Rust workspace with 150+ source files, 802 passing tests, and 20 library modules. All 5 CLI command groups are functional (donor, job, cluster, governance, admin). Production P2P daemon with full libp2p NAT-traversal stack (TCP + QUIC, Noise, mDNS + Kademlia DHT, identify, ping, AutoNAT, Relay v2 server+client, DCUtR) and distributed job dispatch (TaskOffer + TaskDispatch request-response with CBOR + real WASM execution) — validated end-to-end in-process via `tests/nat_traversal.rs`. Core modules implemented: WASM sandbox with CID store integration, real Ed25519 signature verification, certificate chain validation (TPM2/SEV-SNP/TDX), BrightID/OAuth2/phone identity verification, Sigstore Rekor transparency logging, OTLP telemetry, STUN-based NAT detection, Raft coordinator consensus, and Firecracker/Apple VF sandbox drivers.

## Active Technologies
- Rust stable (tested on 1.95.0) + libp2p 0.54, tonic 0.12, ed25519-dalek 2, wasmtime 27, openraft 0.9, opentelemetry 0.27, clap 4 (003-stub-replacement)
- CID-addressed content store (cid 0.11, multihash 0.19), erasure-coded (reed-solomon-erasure 6) (003-stub-replacement)
- Rust stable (tested on 1.95.0) + libp2p 0.54, tonic 0.12, ed25519-dalek 2, wasmtime 27, openraft 0.9, opentelemetry 0.27, clap 4, reqwest 0.12, oauth2 4, x509-parser 0.16, reed-solomon-erasure 6, cid 0.11, multihash 0.19 (004-full-implementation)
- CID-addressed content store (SHA-256), erasure-coded RS(10,18) (004-full-implementation)
- Rust stable 1.95+ (current CI matrix is 1.95.0 on Linux/macOS/Windows + Sandbox KVM + swtpm). Secondary languages: Swift 5.9+ for Apple VF helper binary (macOS-only); TypeScript + React for Tauri GUI frontend; shell (bash) for operator scripts. + libp2p 0.54 (+ new: `libp2p-websocket`, `libp2p-tls`/`libp2p-websocket-websys` for WSS-over-443 transport; `hickory-resolver` with DoH for FR-005); wasmtime 27; candle 0.7+ OR `diffusers-rs` / custom PyTorch-via-FFI for the diffusion backbone (pending research); tonic 0.12 (gRPC); ed25519-dalek 2, ecdsa 0.16, rsa 0.9 (attestation); threshold_crypto 0.4 (BLS); reed-solomon-erasure 6; openraft 0.9; opentelemetry 0.27; clap 4; reqwest 0.12; rcgen 0.13; oci-spec 0.7 + tar 0.4 + `loopdev` or `fscommon`-style library for real Firecracker rootfs; `sysinfo` 0.33 + `nvml-wrapper` 0.10 (GPU metrics for `current_load`); `tss-esapi` 7 or `tpm2-tss` for TPM2-backed confidential compute sealing; Tauri 2 for GUI; `kube` 0.96 + `k8s-openapi` for K8s CRD operator. (005-production-readiness)
- CID-addressed content store (SHA-256) with RS(10,18) erasure coding (already in place); CRDT OR-Map ledger with BLS threshold signing (already in place); per-donor working directory (size-capped, wiped on agent exit) — implemented, no change. (005-production-readiness)

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
cargo test                  # 802 tests (500+ lib + 300+ integration)
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

## Remaining Stubs and Placeholders

Zero TODO comments in src/ and zero `#[ignore]` tests remain. However, several subsystems have scaffolding landed but placeholders in critical paths — these are not production-ready and are tracked in open issues:

- **Mesh LLM** (#27, #54): `src/agent/mesh_llm/expert.rs::load_model()` is a placeholder — no real LLaMA inference. Orchestration (router, aggregator, safety tiers, kill switch) is complete.
- **AMD / Intel root CA fingerprints** (#28): pinned as `[0u8; 32]` in `src/verification/attestation.rs`. Validators enter permissive bypass mode when fingerprints are zero.
- **Rekor public key** (#29): pinned as `[0u8; 32]` in `src/ledger/transparency.rs`. Signed tree head verification is skipped when the key is zero.
- **Agent lifecycle → gossip wiring** (#30): heartbeat/pause/withdraw return payloads but don't broadcast over gossipsub (the daemon event loop does broadcast separately).
- **Firecracker rootfs** (#33): concatenates layer bytes; does NOT run mkfs.ext4 + OCI tar extraction. A real boot would fail.
- **Admin `ban()`** (#34): `src/governance/admin_service.rs::ban()` returns `Ok(())` without updating the trust registry.
- **Platform adapters** (#37, #38, #39): Slurm/K8s/Cloud scaffolds exist but have not been exercised against live systems.
- **GUI** (#40): never built or run.
- **Deployment** (#41): Dockerfile and Helm chart exist but have never been built or deployed.
- **REST gateway** (#43): routing + auth + rate-limit logic exist but no HTTP listener is bound in the daemon.
- **Churn simulator** (#51): statistical model, not a real kill-rejoin harness.
- **Apple VF Swift helper** (#52): never built on macOS.
- **Receipt verification** (`src/verification/receipt.rs`): structural check only; coordinator public key not yet wired.
- **Daemon `current_load()`** (`src/agent/daemon.rs:500`): stub returning 0.1.
- **Cross-machine firewall traversal** (#60): production NAT stack validated in-process only. Real WAN operation behind institutional firewalls is unverified.

## CI

Two GitHub Actions workflows:
- `ci.yml` — basic build + test
- `safety-hardening-ci.yml` — multi-platform (Linux/macOS/Windows) with Principle V evidence artifacts

## Recent Changes
- 005-production-readiness: Added Rust stable 1.95+ (current CI matrix is 1.95.0 on Linux/macOS/Windows + Sandbox KVM + swtpm). Secondary languages: Swift 5.9+ for Apple VF helper binary (macOS-only); TypeScript + React for Tauri GUI frontend; shell (bash) for operator scripts. + libp2p 0.54 (+ new: `libp2p-websocket`, `libp2p-tls`/`libp2p-websocket-websys` for WSS-over-443 transport; `hickory-resolver` with DoH for FR-005); wasmtime 27; candle 0.7+ OR `diffusers-rs` / custom PyTorch-via-FFI for the diffusion backbone (pending research); tonic 0.12 (gRPC); ed25519-dalek 2, ecdsa 0.16, rsa 0.9 (attestation); threshold_crypto 0.4 (BLS); reed-solomon-erasure 6; openraft 0.9; opentelemetry 0.27; clap 4; reqwest 0.12; rcgen 0.13; oci-spec 0.7 + tar 0.4 + `loopdev` or `fscommon`-style library for real Firecracker rootfs; `sysinfo` 0.33 + `nvml-wrapper` 0.10 (GPU metrics for `current_load`); `tss-esapi` 7 or `tpm2-tss` for TPM2-backed confidential compute sealing; Tauri 2 for GUI; `kube` 0.96 + `k8s-openapi` for K8s CRD operator.

- **004-full-implementation** (2026-04-18): Merged scaffolding + significant implementation for #57 and its sub-issues (#28–#56, and a first pass on #27/#54 mesh LLM). 802 tests passing across Linux/macOS/Windows + Sandbox KVM + swtpm CI. Landed: full production P2P daemon with libp2p NAT-traversal stack (TCP + QUIC + Noise + mDNS + Kademlia + identify + ping + AutoNAT + Relay v2 server/client + DCUtR), AutoRelay reservations, public libp2p bootstrap relays as default rendezvous, TaskOffer + TaskDispatch request-response protocols over CBOR, real WASM execution of dispatched jobs, `worldcompute job submit --executor <multiaddr> --workload <wasm>` CLI command, end-to-end 3-node relay-circuit integration test. Also landed: ~12 sub-issues fully completed (policy engine, GPU passthrough, adversarial tests, test coverage, credit decay, preemption, confidential compute, mTLS, energy metering, storage GC, documentation, scheduler matchmaking); ~16 sub-issues partially addressed with scaffolding (see Remaining Stubs above); #27/#54 mesh LLM orchestration shell complete but real LLaMA inference deferred. Critical open issue #60 tracks cross-machine WAN mesh formation behind firewalls.
- **003-stub-replacement** (2026-04-16): Replaced all implementation stubs (#7, #8–#26). 77 tasks, 489+ tests. Added reqwest, oauth2, x509-parser, rcgen dependencies. Wired CLI, sandboxes, attestation, identity, transparency, telemetry, consensus, network.
