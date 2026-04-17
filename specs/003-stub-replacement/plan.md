# Implementation Plan: Replace Implementation Stubs

**Branch**: `003-stub-replacement` | **Date**: 2026-04-16 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-stub-replacement/spec.md`

## Summary

Replace all 76 implementation stubs across 6 module categories (CLI, sandbox, attestation, identity, infrastructure, network) with real functionality. The work spans 19 GitHub issues (#8–#26) and touches every layer of the World Compute stack — from user-facing CLI dispatch to low-level VM hypervisor APIs and hardware attestation chains. The approach prioritizes CLI wiring first (unblocks all user interaction), then sandbox lifecycle (unblocks workload execution), followed by security (attestation, identity), infrastructure (transparency, telemetry, consensus), and network (NAT, DNS).

## Technical Context

**Language/Version**: Rust stable (tested on 1.95.0)
**Primary Dependencies**: libp2p 0.54, tonic 0.12, ed25519-dalek 2, wasmtime 27, openraft 0.9, opentelemetry 0.27, clap 4
**Storage**: CID-addressed content store (cid 0.11, multihash 0.19), erasure-coded (reed-solomon-erasure 6)
**Testing**: cargo test (422 existing tests: 319 lib + 103 integration)
**Target Platform**: Linux (primary — Firecracker KVM), macOS (Apple VF), cross-platform (WASM, CLI)
**Project Type**: CLI + daemon + P2P library
**Performance Goals**: Sub-second preemption yield (Principle III), real-time telemetry export
**Constraints**: Zero unsafe code, zero clippy warnings (-D warnings), no mock-only tests (Principle V)
**Scale/Scope**: 94 source files, 20 modules, ~76 TODO/stub references to replace

**New dependencies needed**:
- HTTP client (reqwest or ureq) for BrightID, OAuth2, Rekor, Apple DeviceCheck APIs
- STUN client crate for NAT detection
- Platform-specific: Swift interop for Apple VF (objc2 or subprocess helper)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Assessment |
|-|-|-|
| I. Safety First | PASS | Sandbox VM lifecycle replaces stubs with real hypervisor isolation. Ed25519 verification and full certificate-chain validation strengthen integrity. No weakening of default-deny egress or sandbox boundaries. |
| II. Robustness | PASS | Raft consensus (#24) adds control-plane replication. All stub replacements maintain safe-by-default behavior (reject/deny on failure). Graceful degradation preserved — missing Firecracker falls back to WASM-only. |
| III. Fairness & Donor Sovereignty | PASS | CLI wiring (#8–#12) gives donors full control over their participation. Identity verification (#19–#21) enables HP-weighted governance. No changes to preemption or credit accounting. |
| IV. Efficiency | PASS | OTLP telemetry (#23) enables observability for efficiency tracking. No new resource waste introduced. NAT detection (#25) improves connectivity efficiency. |
| V. Direct Testing | PASS with conditions | Each stub replacement MUST include integration tests on real resources. Firecracker tests require KVM. Apple VF tests require macOS. Attestation tests require test vectors from real hardware. BrightID/OAuth2/SMS tests require provider sandbox accounts. CI must run platform-specific tests on matching runners. |

**Conditions for Principle V compliance**:
1. Firecracker integration tests run on Linux CI runners with KVM access
2. Apple VF integration tests run on macOS CI runners
3. Attestation tests use real certificate chains from AMD/Intel/Apple (test vectors, not mocks)
4. Identity provider tests use provider sandbox/test modes (not mocked HTTP)
5. Rekor tests hit the public staging instance
6. STUN tests use a real STUN server

### Post-Design Re-evaluation

All gates re-confirmed after Phase 1 design:

| Principle | Post-Design Status | Design Impact |
|-|-|-|
| I. Safety First | PASS | Apple VF subprocess helper avoids unsafe code. reqwest uses rustls (no native OpenSSL). Firecracker API is local UDS only. Credentials from env vars, never hardcoded. |
| II. Robustness | PASS | Raft storage uses in-memory + WAL for restart survival. CertificateChainValidator trait enables graceful CA fallback. All HTTP calls map errors to existing safe-by-default result variants. |
| III. Fairness | PASS | CLI contract defines consistent error format. No changes to preemption or credit systems. |
| IV. Efficiency | PASS | OTLP enables efficiency monitoring. reqwest connection pooling avoids per-request overhead. |
| V. Direct Testing | PASS (conditions unchanged) | Research confirmed platform requirements. No new conditions beyond pre-design assessment. |

## Project Structure

### Documentation (this feature)

```text
specs/003-stub-replacement/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (CLI contract, gRPC updates)
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs                          # CLI dispatch (issues #8–#12)
├── cli/
│   ├── mod.rs                       # CLI module exports
│   ├── donor.rs                     # Donor subcommands → agent lifecycle
│   ├── submitter.rs                 # Job subcommands → scheduler
│   ├── governance.rs                # Governance subcommands → governance module
│   └── admin.rs                     # Admin subcommands → admin service
├── sandbox/
│   ├── firecracker.rs               # Firecracker API socket config (#13)
│   ├── apple_vf.rs                  # Apple VF Swift FFI bridge (#14)
│   └── wasm.rs                      # WASM CID loading + wasmtime (#15)
tools/
└── apple-vf-helper/                 # Swift helper binary for Apple VF lifecycle (subprocess, not FFI)
├── policy/
│   └── rules.rs                     # Ed25519 real verification (#16)
├── verification/
│   └── attestation.rs               # TPM2/SEV-SNP/TDX chain validation (#17), Apple SE (#18)
├── identity/
│   ├── personhood.rs                # BrightID HTTP client (#19)
│   ├── oauth2.rs                    # OAuth2 provider adapters (#20)
│   └── phone.rs                     # SMS/phone verification (#21)
├── registry/
│   └── transparency.rs              # Sigstore Rekor integration (#22)
├── ledger/
│   └── transparency.rs              # Ledger-side Rekor anchoring (#22)
├── telemetry/
│   └── mod.rs                       # OTLP exporter wiring (#23)
├── scheduler/
│   └── coordinator.rs               # Raft consensus via openraft (#24)
├── network/
│   ├── nat.rs                       # STUN-based NAT detection (#25)
│   └── discovery.rs                 # DNS seed nodes (#26)
tests/
├── cli/                             # New: CLI integration tests
├── sandbox/                         # Existing + new VM lifecycle tests
├── attestation/                     # New: certificate chain validation tests
├── identity/                        # Existing + new provider integration tests
├── infrastructure/                  # New: Rekor, OTLP, Raft tests
└── network/                         # New: NAT detection, DNS seed tests
```

**Structure Decision**: The existing module structure is maintained — each stub replacement modifies files in-place. No new modules are created; only new test files are added under `tests/`.

## Implementation Phases

### Phase A: CLI Wiring (Issues #8–#12) — Foundation

**Rationale**: Unblocks all user interaction. No external dependencies. Lowest risk, highest immediate usability.

1. Modify `src/main.rs` to change each `Commands::*` unit variant to carry the corresponding CLI struct (e.g., `Donor(cli::donor::DonorCli)`)
2. In each `cli/*.rs`, replace placeholder returns in `execute()` with calls to the real module functions
3. Add integration tests that invoke each subcommand and verify output

**Dependencies**: None — purely internal wiring.

### Phase B: Sandbox VM Lifecycle (Issues #13–#15) — Core Compute

**Rationale**: Enables the core value proposition (running workloads). Requires platform-specific work.

1. **Firecracker** (#13): Implement HTTP calls to the Firecracker API socket (PUT /machine-config, /boot-source, /drives, /network-interfaces, /actions)
2. **Apple VF** (#14): Build Swift helper binary or use objc2 crate for VZVirtualMachineConfiguration lifecycle
3. **WASM** (#15): Implement CID store fetch → wasmtime Module::new → Instance::new → function invocation

**Dependencies**: Firecracker binary + KVM (Linux only), Xcode + macOS 12+ (Apple VF), wasmtime already in Cargo.toml.

### Phase C: Attestation & Crypto (Issues #16–#18) — Security Hardening

**Rationale**: Strengthens trust model. Requires certificate chain knowledge but no external services for Ed25519.

1. **Ed25519** (#16): Replace length/non-zero check with `ed25519_dalek::VerifyingKey::verify()` against registered pubkey
2. **TPM2/SEV-SNP/TDX** (#17): Implement chain validation — parse endorsement certificates, verify signatures up to root CA
3. **Apple SE** (#18): HTTP client to Apple's DeviceCheck/App Attest verification endpoint

**Dependencies**: ed25519-dalek already in Cargo.toml. Platform CA certs (AMD ARK, Intel DCAP roots). Apple Developer account for DeviceCheck.

### Phase D: Identity & Verification (Issues #19–#21) — Enrollment

**Rationale**: Enables real user enrollment and governance participation. Requires HTTP client and provider accounts.

1. **BrightID** (#19): Add reqwest, implement GET to BrightID verification API, parse response
2. **OAuth2** (#20): Implement authorization code flow for each provider (GitHub, Google, Twitter, email)
3. **Phone** (#21): Integrate SMS provider API (Twilio or equivalent) for send + verify

**Dependencies**: New dependency: reqwest (HTTP client). Provider sandbox accounts for testing.

### Phase E: Infrastructure (Issues #22–#24) — Operational Maturity

**Rationale**: Adds transparency, observability, and fault tolerance. Can run after core compute works.

1. **Rekor** (#22): HTTP POST to Rekor REST API, parse and store log entry receipts
2. **OTLP** (#23): Wire opentelemetry-otlp exporter when `otel_endpoint` is set, connect to tracing subscriber
3. **Raft** (#24): Implement openraft RaftStorage trait for coordinator state, wire leader election and heartbeats

**Dependencies**: opentelemetry-otlp and openraft already in Cargo.toml. Rekor public instance for testing.

### Phase F: Network (Issues #25–#26) — Connectivity

**Rationale**: Improves peer discovery but mDNS already works for development. Lowest priority.

1. **NAT** (#25): Add STUN client crate, implement RFC 5389 binding request, classify NAT type
2. **DNS seeds** (#26): Replace placeholder addresses with real World Compute DNS seed hostnames

**Dependencies**: New dependency: STUN client crate. DNS seed domain registration.

## Complexity Tracking

No constitution violations requiring justification. All phases comply with Principles I–V.

| Consideration | Decision | Rationale |
|-|-|-|
| HTTP client choice | reqwest (async, tokio-native) | Already using tokio runtime; ureq is sync-only. Needed by 5+ stubs (#18–#22). |
| Apple VF approach | Swift helper binary via subprocess | Avoids unsafe FFI complexity. Helper is code-signed per Principle I. |
| Attestation CA certs | Bundled trust anchors + vendor API fallback | Offline validation preferred; online fetch as fallback for freshness. |
| Raft storage backend | In-memory with WAL | Sufficient for initial deployment; disk-backed upgrade is a future concern. |
| STUN server | Use public STUN servers (Google, Cloudflare) | Standard practice; no dependency on World Compute infrastructure. |
