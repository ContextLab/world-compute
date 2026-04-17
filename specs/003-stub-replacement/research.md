# Research: Replace Implementation Stubs

**Branch**: `003-stub-replacement` | **Date**: 2026-04-16

## 1. CLI Wiring (#8–#12)

### Current State
- `src/main.rs`: All 5 `Commands::*` variants are unit variants (no payload). Match arms print "not yet implemented."
- `src/cli/donor.rs`: Full `DonorCli` struct with `DonorCommand` enum (Join, Status, Pause, Resume, Leave, Credits, Logs). `execute()` exists but returns placeholder strings.
- `src/cli/submitter.rs`: `SubmitterCli` struct with job commands.
- `src/cli/governance.rs`: `GovernanceCli` struct with propose/list/vote/report.
- `src/cli/admin.rs`: `AdminCli` struct with halt/resume/ban/audit.
- Cluster CLI: No dedicated struct exists yet — needs creation.

### Decision
Wire each CLI struct into `main.rs` by changing unit variants to tuple variants carrying the CLI struct. Each `execute()` function dispatches to the corresponding library module.

### Rationale
Purely mechanical wiring — no design alternatives. The subcommand structs already exist with correct argument definitions.

### Alternatives Considered
None — the architecture is already defined by the existing CLI structs.

---

## 2. Firecracker API Socket (#13)

### Current State
- `src/sandbox/firecracker.rs`:
  - **Line 227–228**: Process spawning works (launches `firecracker --api-sock`), captures PID. But no HTTP calls to the API socket.
  - **Line 274**: Snapshot creation writes file placeholders instead of calling `PUT /snapshot/create`.
  - **Lines 101–120**: Rootfs preparation is stubbed (writes placeholder file).
  - **Lines 123–152**: Network config is stubbed (logs only).
  - API socket path defined at line 58 (`work_dir/firecracker.sock`).

### Decision
Use hyper (already available via reqwest dependency) or a lightweight Unix socket HTTP client to issue PUT requests to the Firecracker API socket. Sequence: /machine-config → /boot-source → /drives/rootfs → /network-interfaces/eth0 → /actions InstanceStart.

### Rationale
Firecracker uses a REST API over Unix domain socket. The protocol is simple HTTP PUT with JSON bodies. hyper with unix socket support is the standard Rust approach.

### Alternatives Considered
- **Raw TCP over UDS**: Too low-level; would need to implement HTTP framing manually.
- **reqwest with unix socket**: reqwest doesn't natively support UDS; would need a custom connector. hyper is simpler for this use case.

---

## 3. Apple Virtualization.framework FFI (#14)

### Current State
- `src/sandbox/apple_vf.rs`:
  - **Line 138**: `start()` — logs but TODOs Swift FFI bridge for VZVirtualMachineConfiguration.
  - **Line 154**: `freeze()` — TODOs VZVirtualMachine.pause() FFI.
  - **Line 173**: `checkpoint()` — writes "vm-state-placeholder" instead of calling saveMachineStateTo().
  - **Line 191**: `terminate()` — TODOs VZVirtualMachine.stop() FFI.

### Decision
Build a Swift helper binary (`wc-apple-vf-helper`) invoked via subprocess. The helper accepts JSON commands on stdin and returns JSON results on stdout. Commands: create, start, pause, resume, stop, checkpoint.

### Rationale
Direct Objective-C FFI from Rust (via objc2 crate) is fragile, requires unsafe code (violating project conventions), and is hard to test. A subprocess helper is code-signed independently (Principle I), testable in isolation, and avoids unsafe blocks entirely.

### Alternatives Considered
- **objc2 crate**: Requires unsafe code, complex lifecycle management. Rejected per zero-unsafe-code policy.
- **C bridging header**: Same unsafe issues as objc2 with extra build complexity.

---

## 4. WASM Module Loading (#15)

### Current State
- `src/sandbox/wasm.rs`:
  - **Line 35**: `create()` — logs CID but skips module fetch and compilation.
  - **Line 43**: `start()` — sets running flag but TODOs instantiation and stdout capture.
  - **Line 106**: `run_module()` — returns empty `Vec::new()`, no actual WASM execution.

### Decision
Implement: (1) fetch WASM bytes from CID store via existing `data_plane::cid_store`, (2) compile with `wasmtime::Module::new()`, (3) instantiate with `wasmtime::Instance::new()`, (4) call exported `_start` or specified entry function, (5) capture stdout via WASI preview1.

### Rationale
wasmtime 27 is already in Cargo.toml. The WASI preview1 API provides standard stdout/stderr capture. CID store integration follows the existing data_plane patterns.

### Alternatives Considered
- **wasmer**: Not already in dependencies; wasmtime is the project's chosen WASM runtime.

---

## 5. Ed25519 Signature Verification (#16)

### Current State
- `src/policy/rules.rs` **line 60–61**: `check_signature()` rejects only empty or all-zero 64-byte signatures. Any other 64-byte value passes. Comment references T018 Phase 2.

### Decision
Replace with `ed25519_dalek::VerifyingKey::from_bytes(&ctx.submitter_public_key)` → `verifying_key.verify(&message, &signature)`. The message is the manifest hash. The public key comes from the submitter's registered identity.

### Rationale
ed25519-dalek 2 is already in Cargo.toml. The API is straightforward: construct VerifyingKey, call verify(). No new dependencies needed.

### Alternatives Considered
- **ring**: Also viable but ed25519-dalek is already a dependency and provides a cleaner API for Ed25519 specifically.

---

## 6. TPM2/SEV-SNP/TDX Certificate Chain Validation (#17)

### Current State
- `src/verification/attestation.rs`:
  - **Lines 392–398**: `verify_quote_signature()` uses crude binding check — first 4 bytes of signature must match first 4 bytes of SHA-256(signed_data). Not real crypto.
  - **Lines 401–424**: `verify_tpm2()`, `verify_sev_snp()`, `verify_tdx()` perform structural parsing (magic bytes, length, field presence) then delegate to the stubbed signature check.
  - **Implemented**: Measurement registry (known-good PCR/measurement values), quote structure parsing, empty/zero rejection, agent version rollover.
  - **Missing**: Actual cryptographic signature verification, CA certificate chain validation.

### Decision
Implement a `CertificateChainValidator` trait with platform-specific implementations:
- **TPM2**: Parse EK certificate, verify AIK signature against EK, verify quote signature against AIK.
- **SEV-SNP**: Fetch/bundle AMD ARK → ASK → VCEK chain. Verify VCEK signs the attestation report.
- **TDX**: Use Intel DCAP verification library or implement ECDSA chain validation against Intel's root CA.

Bundle known root CA certificates as compile-time constants. Provide runtime refresh via vendor API as fallback.

### Rationale
Certificate chain validation is the core of hardware attestation. Bundled CAs enable offline verification (faster, no network dependency). Runtime fetch handles CA rotation.

### Alternatives Considered
- **Vendor-hosted verification services**: Adds network dependency to the critical trust path. Rejected for offline-first approach.
- **openssl crate**: Heavy dependency. Prefer pure-Rust x509 parsing (x509-parser crate or webpki).

---

## 7. Apple Secure Enclave DeviceCheck (#18)

### Current State
- `src/verification/attestation.rs` **lines 426–442**: `verify_apple_se()` checks only payload length ≥ 64 bytes and last 64 bytes non-trivial. No Apple API call.

### Decision
Implement HTTP POST to Apple's App Attest validation endpoint. The attestation object (CBOR-encoded) is sent to Apple's server which returns a verified assertion or error.

### Rationale
Apple Secure Enclave attestation cannot be verified locally — it requires Apple's server to validate the device identity. This is by design (Apple controls the root of trust).

### Alternatives Considered
- **Local verification**: Not possible for Apple SE. Apple's server is the only authority.

---

## 8. BrightID HTTP Client (#19)

### Current State
- `src/identity/personhood.rs` **line 103**: `ureq_get_brightid()` returns `Err("HTTP client not yet integrated")`.
- `BrightIdVerification` struct exists with `verified`, `unique`, `context_id`, `error` fields (deserializable from JSON).
- `PersonhoodResult` enum: Verified, Pending{connections_needed}, Failed(String), ProviderUnavailable(String).

### Decision
Add reqwest as an async HTTP client. Implement `GET /node/v6/verifications/WorldCompute/{contextId}` call. Parse JSON response into existing `BrightIdVerification` struct. Map to `PersonhoodResult`.

### Rationale
reqwest is async and tokio-native (project already uses tokio). The BrightID API is a simple GET endpoint. The data model already exists.

### Alternatives Considered
- **ureq**: Sync-only; would block the tokio runtime. The stub even mentions ureq but async is better for a P2P daemon.

### New Dependency
`reqwest = { version = "0.12", features = ["json", "rustls-tls"] }` — needed by 5+ stubs (#18–#22). Use rustls-tls to avoid native openssl dependency.

---

## 9. OAuth2 Provider Adapters (#20)

### Current State
- `src/identity/oauth2.rs` **line 27**: `verify_oauth2()` returns `ProviderUnavailable("OAuth2 verification flows not yet implemented")`.
- `OAuth2Result` enum: Verified{provider, account_id}, Failed(String), ProviderUnavailable(String).

### Decision
Implement authorization code flow for each provider:
1. Generate authorization URL with provider-specific scopes
2. Handle callback with authorization code
3. Exchange code for access token via provider's token endpoint
4. Fetch user profile to get account_id
5. Return OAuth2Result::Verified

Provider config (client_id, client_secret, redirect_uri) loaded from environment variables.

### Rationale
Standard OAuth2 authorization code flow. Each provider follows the same pattern with different endpoint URLs and scopes.

### Alternatives Considered
- **oauth2 crate**: Provides OAuth2 flow abstractions. Worth adding to avoid reimplementing token exchange. Decision: use `oauth2 = "4"` crate.

### New Dependency
`oauth2 = "4"` — standard Rust OAuth2 client library.

---

## 10. Phone/SMS Verification (#21)

### Current State
- `src/identity/phone.rs`:
  - **Line 18**: `send_verification_code()` returns error.
  - **Line 25**: `verify_code()` returns error.
- `PhoneResult` enum: Verified{phone_hash}, CodeExpired, InvalidCode, ProviderUnavailable(String).

### Decision
Implement Twilio Verify API integration:
1. `send_verification_code()`: POST to Twilio Verify to send SMS/voice code.
2. `verify_code()`: POST to Twilio Verify to check code.
Provider abstracted behind a trait for future provider swap.

### Rationale
Twilio Verify is the most widely used SMS verification API. Trait abstraction allows swapping to Vonage or another provider later.

### Alternatives Considered
- **Vonage**: Viable alternative. Trait abstraction makes this swappable.
- **AWS SNS**: More complex setup. Twilio is simpler for SMS verification.

---

## 11. Sigstore Rekor Integration (#22)

### Current State
- `src/registry/transparency.rs` **line 60**: Returns `TransparencyLogResult::Unavailable` with placeholder message.
- `src/ledger/transparency.rs` **lines 28–44**: Creates fake entry ID from hash prefix (`stub-rekor-{hex_prefix}`). Verify function always returns `Ok(true)`.

### Decision
Implement HTTP POST to `https://rekor.sigstore.dev/api/v1/log/entries` with hashedrekord entry type. Parse response for log index, entry UUID, and inclusion proof. Verify function checks inclusion proof against Rekor's signed tree head.

### Rationale
Rekor's REST API is well-documented. The hashedrekord type is the simplest entry format — just a hash and signature. Public instance available for development; private instance for production.

### Alternatives Considered
- **sigstore-rs crate**: Provides Rust bindings. Decision: evaluate maturity; if insufficient, use raw reqwest calls.

---

## 12. OpenTelemetry OTLP Exporter (#23)

### Current State
- `src/telemetry/mod.rs` **line 20**: Ignores `otel_endpoint` parameter (`let _ = otel_endpoint`). Only initializes JSON logging via tracing-subscriber.

### Decision
When `otel_endpoint` is Some:
1. Create OTLP trace exporter via `opentelemetry_otlp::new_exporter().tonic()`
2. Create trace pipeline with batch span processor
3. Add tracing-opentelemetry layer to the subscriber
4. Register metrics exporter for runtime metrics

All OTLP dependencies are already in Cargo.toml.

### Rationale
The dependencies (opentelemetry 0.27, opentelemetry-otlp 0.27, tracing-opentelemetry 0.28) are already declared. This is purely wiring code.

### Alternatives Considered
None — the technology choice was already made when dependencies were added to Cargo.toml.

---

## 13. Raft Consensus (#24)

### Current State
- `src/scheduler/coordinator.rs`:
  - **Line 55**: `start_election()` increments term, sets Candidate role. No RPC broadcasting.
  - **Line 64**: `become_leader()` sets Leader role. No heartbeat sending or log replication.

### Decision
Implement openraft's `RaftStorage` and `RaftNetworkFactory` traits for coordinator state:
1. `RaftStorage`: In-memory log with optional write-ahead log (WAL) file.
2. `RaftNetworkFactory`: Use existing libp2p gossipsub for Raft RPC transport.
3. Wire `Raft::new()` into coordinator startup.
4. Replace stub election/leader with openraft's built-in leader election.

### Rationale
openraft 0.9 is already in Cargo.toml. It provides a complete Raft implementation — just needs storage and network adapters.

### Alternatives Considered
- **Custom Raft**: Reimplementing Raft is error-prone. openraft is well-tested.
- **etcd**: External dependency; too heavy for an embedded coordinator.

---

## 14. NAT Detection (#25)

### Current State
- `src/network/nat.rs` **line 35**: Returns hardcoded `NatStatus::Direct`. Comment notes real detection needs AutoNAT and STUN.

### Decision
Implement STUN binding request (RFC 5389) to determine external address and NAT type. Use libp2p's built-in AutoNAT behavior for ongoing NAT status updates.

### Rationale
libp2p already includes AutoNAT support. For initial bootstrap (before peers are available), a STUN binding request to public servers (Google, Cloudflare) provides the external address.

### New Dependency
`stun_client` or use raw UDP with STUN message parsing. Evaluate `stun-rs` crate.

### Alternatives Considered
- **libp2p AutoNAT only**: Requires existing peers; doesn't work at bootstrap. STUN supplements it.

---

## 15. DNS Seed Nodes (#26)

### Current State
- `src/network/discovery.rs` **line 63**: `DiscoveryConfig::default()` populates bootstrap seeds with placeholder `/dnsaddr/bootstrap1.worldcompute.org`.

### Decision
Replace placeholder addresses with real World Compute DNS seed hostnames once domain is registered. For now, make the seed list configurable via config file or environment variable, with the placeholder as fallback.

### Rationale
DNS seeds cannot be implemented until the domain and seed infrastructure exist. Making the list configurable allows deployment without code changes.

### Alternatives Considered
- **Hardcoded IPs**: Fragile; DNS names are preferred for infrastructure flexibility.
- **DHT-only bootstrap**: Requires at least one known peer. DNS seeds solve the initial bootstrap problem.

---

## Summary of New Dependencies

| Dependency | Version | Purpose | Used By |
|-|-|-|-|
| reqwest | 0.12 (rustls-tls, json) | Async HTTP client | BrightID, OAuth2, Rekor, Apple DeviceCheck, Twilio |
| oauth2 | 4 | OAuth2 authorization code flow | OAuth2 adapters (#20) |
| x509-parser | 0.16 | Certificate chain parsing | TPM2/SEV-SNP/TDX validation (#17) |
| stun-rs or equivalent | latest | STUN binding requests | NAT detection (#25) |

**No new dependency needed for**: CLI wiring, WASM loading, Ed25519, OTLP, Raft consensus, DNS seeds, Firecracker API.
