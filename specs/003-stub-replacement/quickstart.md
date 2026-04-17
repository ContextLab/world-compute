# Quickstart: Stub Replacement Development

**Branch**: `003-stub-replacement` | **Date**: 2026-04-16

## Prerequisites

- Rust stable (1.95.0+)
- For Firecracker testing: Linux with KVM access (`/dev/kvm`)
- For Apple VF testing: macOS 12+ with Xcode
- For identity provider testing: sandbox/test accounts (BrightID, Twilio, OAuth2 providers)

## Build & Test

```sh
# Build everything
cargo build

# Run all tests (422 existing + new)
cargo test

# Clippy (zero warnings enforced)
cargo clippy --lib -- -D warnings
```

## Phase-by-Phase Development

### Phase A: CLI Wiring (start here)

No external dependencies. Pure code wiring.

```sh
# After wiring, verify each command dispatches:
cargo run -- donor status
cargo run -- job list
cargo run -- cluster status
cargo run -- governance list
cargo run -- admin audit --since "1h"
```

### Phase B: Sandbox VM Lifecycle

Requires platform-specific setup:

```sh
# WASM (cross-platform, start here):
cargo test --lib sandbox::wasm

# Firecracker (Linux + KVM only):
# Install firecracker binary, kernel, rootfs
cargo test --lib sandbox::firecracker

# Apple VF (macOS only):
# Requires Xcode, signing
cargo test --lib sandbox::apple_vf
```

### Phase C: Attestation & Crypto

```sh
# Ed25519 (no external deps):
cargo test --lib policy::rules

# Certificate chain validation (needs test vectors):
cargo test --lib verification::attestation
```

### Phase D: Identity & Verification

Requires provider sandbox accounts:

```sh
# Set environment variables for testing:
export BRIGHTID_NODE_URL="https://app.brightid.org"
export TWILIO_ACCOUNT_SID="test_..."
export TWILIO_AUTH_TOKEN="test_..."
export TWILIO_VERIFY_SID="VA..."

cargo test --lib identity
```

### Phase E: Infrastructure

```sh
# OTLP (needs a collector running):
docker run -p 4317:4317 otel/opentelemetry-collector
export OTEL_ENDPOINT="http://localhost:4317"
cargo test --lib telemetry

# Rekor (hits public staging):
cargo test --lib registry::transparency

# Raft consensus:
cargo test --lib scheduler::coordinator
```

### Phase F: Network

```sh
# NAT detection (needs network):
cargo test --lib network::nat

# DNS seeds (needs DNS resolution):
cargo test --lib network::discovery
```

## Adding reqwest Dependency

Several stubs need an HTTP client. Add to Cargo.toml:

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
oauth2 = "4"
```

## Environment Variables

| Variable | Purpose | Required By |
|-|-|-|
| BRIGHTID_NODE_URL | BrightID API base URL | #19 |
| OAUTH2_GITHUB_CLIENT_ID | GitHub OAuth2 app ID | #20 |
| OAUTH2_GITHUB_CLIENT_SECRET | GitHub OAuth2 app secret | #20 |
| OAUTH2_GOOGLE_CLIENT_ID | Google OAuth2 app ID | #20 |
| OAUTH2_GOOGLE_CLIENT_SECRET | Google OAuth2 app secret | #20 |
| TWILIO_ACCOUNT_SID | Twilio account SID | #21 |
| TWILIO_AUTH_TOKEN | Twilio auth token | #21 |
| TWILIO_VERIFY_SID | Twilio Verify service SID | #21 |
| OTEL_ENDPOINT | OTLP collector endpoint | #23 |
| APPLE_TEAM_ID | Apple Developer team ID | #18 |
| APPLE_KEY_ID | Apple DeviceCheck key ID | #18 |
