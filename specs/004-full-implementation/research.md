# Research: Full Functional Implementation

**Date**: 2026-04-17
**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

## R1: Certificate Chain Cryptographic Verification

**Decision**: Use `rsa` (0.9) and `p256`/`p384` crates for signature verification in attestation chains
**Rationale**: These are the standard Rust crates for RSA and ECDSA operations, maintained by the RustCrypto project. They integrate with `x509-parser` (already a dependency) for certificate field extraction.
**Alternatives considered**:
- `ring`: Faster but more opinionated API, harder to extract individual signature components for chain validation
- `openssl-rs`: Full OpenSSL binding — too heavyweight, introduces C dependency, conflicts with `rustls-tls` approach

**Implementation notes**:
- TPM2 EK certificates use RSA-2048 signatures
- AMD SEV-SNP VCEK certificates use ECDSA-P384
- Intel TDX PCK certificates use ECDSA-P256
- Pin root CA fingerprints as `const [u8; 32]` SHA-256 digests compiled into the binary

## R2: Merkle Inclusion Proof Verification

**Decision**: Implement RFC 6962 proof verification directly (no external crate)
**Rationale**: The algorithm is ~50 lines of Rust (iterative hash combination up the tree). A dedicated crate would be overkill and add an unnecessary dependency.
**Alternatives considered**:
- `merkle-tree` crate: Provides tree construction but not RFC 6962 inclusion proof verification
- `certificate-transparency` crate: Unmaintained

**Implementation notes**:
- Rekor public key pinned as compile-time constant (fetched from Rekor API `/api/v1/log/publicKey`)
- Verify: leaf_hash → apply proof hashes → compare to signed tree root → verify root signature with Rekor pubkey

## R3: Preemption Latency Measurement

**Decision**: Use `std::time::Instant` for nanosecond-precision timing, `nix` crate for SIGSTOP delivery
**Rationale**: `nix` provides safe Rust wrappers for Unix signals. `Instant::elapsed()` gives reliable monotonic timing.
**Alternatives considered**:
- `libc::kill` directly: Works but unsafe, `nix` wraps it safely
- `tokio::signal`: Async signal handling adds unnecessary complexity for synchronous SIGSTOP

**Implementation notes**:
- Preemption supervisor runs on a dedicated high-priority thread (not tokio runtime)
- Sovereignty trigger → record timestamp → SIGSTOP all sandbox PIDs → record completion → log delta
- Target: delta < 10ms measured on real hardware

## R4: Confidential Compute Encryption

**Decision**: Use `aes-gcm` crate for AES-256-GCM encryption
**Rationale**: AEAD cipher recommended by NIST, widely used, hardware-accelerated on modern CPUs via AES-NI. The `aes-gcm` crate is part of RustCrypto and zero-dependency.
**Alternatives considered**:
- `chacha20poly1305`: Good alternative for non-AES-NI hardware, but AES-GCM has better hardware support on server-class hardware which is the primary use case
- `ring::aead`: Good but `ring` dependency conflicts noted in R1

**Implementation notes**:
- Per-job ephemeral key: 256-bit random via `rand::OsRng`
- Key wrapped with submitter's Ed25519 public key (X25519 key agreement via `x25519-dalek`)
- Confidential-high: key sealed to guest measurement hash, released only to matching attested sandbox

## R5: Threshold Signing

**Decision**: Use `threshold-crypto` crate (already in Cargo.toml) for t-of-n threshold signatures
**Rationale**: Already a dependency, implements Shamir secret sharing + BLS threshold signatures. Coordinator quorum of 3-of-5 maps directly to the API.
**Alternatives considered**:
- FROST (Schnorr threshold): More modern but less mature Rust implementations
- Multi-party ECDSA: Complex, requires multiple rounds of communication

**Implementation notes**:
- Key generation: dealer generates polynomial, distributes shares to 5 coordinators
- Signing: each coordinator produces signature share, any 3 combine to full signature
- Verification: standard BLS signature verification against the group public key

## R6: Slurm Integration

**Decision**: Use slurmrestd REST API (Slurm 21.08+) with `reqwest` HTTP client
**Rationale**: slurmrestd is the official REST API for Slurm, avoids SSH+sbatch complexity, provides structured JSON responses for job status and cluster capacity.
**Alternatives considered**:
- SSH + sbatch/squeue/sacct: Works everywhere but parsing text output is fragile
- Slurm C API bindings: Too complex, requires Slurm headers at build time

**Implementation notes**:
- Check if tensor01.dartmouth.edu runs Slurm with slurmrestd enabled
- If not, fall back to SSH+sbatch approach with structured output parsing
- Capacity reporting via `sinfo --json` or equivalent

## R7: Kubernetes Operator Pattern

**Decision**: Use `kube` (0.88) + `k8s-openapi` (0.21) crates for K8s operator
**Rationale**: `kube` is the standard Rust Kubernetes client, supports CRD watching, Pod creation, and resource management. Well-maintained and async-native.
**Alternatives considered**:
- `k8s-openapi` alone: Lower-level, requires manual HTTP client setup
- Shell out to `kubectl`: Fragile, not suitable for operator pattern

## R8: Mesh LLM Model Loading

**Decision**: Use `candle` (Hugging Face Rust ML framework) for LLaMA-3-8B inference
**Rationale**: Native Rust, no Python dependency, supports GGUF quantized models, CUDA and Metal backends. Avoids the complexity of llama.cpp bindings.
**Alternatives considered**:
- `llama-cpp-rs`: C++ bindings, works but adds build complexity
- `tch-rs` (PyTorch bindings): Heavy dependency, requires libtorch
- Direct ONNX runtime: Good but LLaMA-3 ONNX exports are less common than GGUF

**Implementation notes**:
- Load LLaMA-3-8B-Q4_K_M.gguf (~4.6GB VRAM)
- Each expert node runs full inference locally
- Router sends prompt, expert returns top-256 logits
- candle supports efficient top-k logit extraction

## R9: Energy Metering

**Decision**: Read Intel RAPL via `/sys/class/powercap/intel-rapl/` on Linux, `powermetrics` on macOS
**Rationale**: RAPL is the standard interface for CPU energy measurement on Intel/AMD processors. Available without root on most Linux distributions.
**Alternatives considered**:
- `sysinfo` crate: Provides CPU usage but not energy/power
- NVML for GPU power: NVIDIA Management Library provides GPU watt readings
- External power meter: Most accurate but not automatable

**Implementation notes**:
- Read RAPL energy counter before/after job execution
- Delta gives joules consumed
- GPU: read via NVML `nvmlDeviceGetPowerUsage()` if NVIDIA GPU present
- Estimate watts = joules / seconds
- Compare against wall-meter on tensor01 for calibration (target: within 20%)

## R10: Docker Multi-Stage Build

**Decision**: Two-stage Dockerfile — Rust builder + minimal runtime (distroless or alpine)
**Rationale**: Rust static linking produces small binaries. Multi-stage keeps the image minimal (<100MB target).
**Alternatives considered**:
- Single-stage with rust:slim: Works but image is ~700MB
- Cross-compilation + scratch: Smallest but harder to debug

**Implementation notes**:
- Stage 1: `rust:1.95-bookworm` with `cargo build --release`
- Stage 2: `gcr.io/distroless/cc-debian12` with just the binary
- Docker Compose: 3 services (coordinator, broker, agent) with shared network
