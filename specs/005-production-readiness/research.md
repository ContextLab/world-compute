# Phase 0 Research — Spec 005 Production Readiness

**Feature**: 005-production-readiness
**Date**: 2026-04-19
**Scope**: Resolve every NEEDS CLARIFICATION flag in the plan's Technical Context and document best-practice choices for each new subsystem introduced by spec 005.

All research items below were derived from the plan's Technical Context and from the 44 FRs in the spec. Each item follows the Decision / Rationale / Alternatives format.

---

## 1. WebSocket-over-TLS-443 libp2p transport (FR-003)

**Decision**: Use `libp2p-websocket` with `libp2p-tls` (rustls-backed) to build a WSS transport that listens on port 443 and dials WSS addresses (`/ip4/.../tcp/443/tls/ws/p2p/...`). Enable it as a fallback transport behind automatic transport-selection logic that prefers QUIC → TCP → WSS-443 in that order.

**Rationale**: `libp2p-websocket` is a production transport in `rust-libp2p 0.54` with working examples (see `rust-libp2p/examples/browser-webrtc`). `libp2p-tls` is already in our dep tree for Noise-over-TLS and is actively maintained. Port 443 is allowed through virtually every institutional firewall (HTTPS cannot be blocked without breaking the web). TLS-inside-WebSocket-inside-TCP is the same pattern used by Signal, WhatsApp, Telegram for firewall traversal. No new foundational library is required.

**Alternatives considered**:
- **HTTP/3 / MASQUE**: More future-proof, but `libp2p-masque` does not yet exist as a crate and would be a multi-month research effort.
- **Custom obfuscated protocol on 443**: Defeated by SNI-aware middleboxes; adds DPI-evasion complexity the project cannot maintain.
- **Tor pluggable transports**: Too heavyweight; Tor Browser users already do this, volunteer-compute operators should not have to.

**Implementation notes**:
- The transport must negotiate ALPN so middleboxes treat it as normal HTTPS.
- When an SSL-inspecting middlebox is detected (certificate pin mismatch against known relay fingerprints), log a security warning and require `--allow-ssl-inspection` opt-in per the Edge Cases section of the spec.
- Reservation/circuit negotiation works identically over WSS transport; relay_v2 is transport-agnostic.

---

## 2. DNS-over-HTTPS resolver fallback (FR-005)

**Decision**: Use `hickory-resolver` (formerly `trust-dns-resolver`) 0.24+ in DoH mode with Cloudflare `1.1.1.1` and Google `8.8.8.8` as the default upstreams, bundled into the agent binary. Apply it only when the OS resolver fails to resolve a `/dnsaddr/` multiaddr within a bounded timeout (5 s).

**Rationale**: `hickory-resolver` is the canonical async Rust DNS library, works on all three target platforms, and supports DoH via RFC 8484. Bundling two independent public resolvers provides redundancy. The fallback is *only* engaged on OS-resolver failure, so it does not add startup latency in the common case. Captive portals and strict DNS filtering — common in universities, hotels, and enterprise guest networks — are addressed by this fallback.

**Alternatives considered**:
- **Systemd-resolved integration**: Linux-only; doesn't help macOS or Windows.
- **DNS-over-TLS (DoT)**: Also works, but DoH is more firewall-permissive (uses 443; DoT uses 853 which is sometimes blocked).
- **Hard-code IP addresses**: The project's `/dnsaddr/bootstrap.worldcompute.org/...` seeds *will* migrate in the future; hard-coding defeats the purpose of the DNS layer.

**Implementation notes**:
- Do NOT use DoH as the primary resolver — OS-resolver-first keeps the happy path fast and unsurprising.
- Log when DoH fallback is engaged (FR-004 requires visible dial failures; extend to resolver events).

---

## 3. Pinned root CA fingerprints (FR-008, FR-011a)

**Decision**: Pin three 32-byte SHA-256 fingerprints in `src/verification/attestation.rs` and `src/ledger/transparency.rs`:
1. AMD ARK (Ark Root Key): `c4a8...` — SHA-256 of the DER-encoded AMD ARK certificate from [https://kdsintf.amd.com/vcek/v1/Milan/cert_chain](https://kdsintf.amd.com/vcek/v1/Milan/cert_chain) and [https://kdsintf.amd.com/vcek/v1/Genoa/cert_chain](https://kdsintf.amd.com/vcek/v1/Genoa/cert_chain). Both ARKs are identical for current EPYC generations.
2. Intel DCAP root CA: SHA-256 of the DER-encoded Intel SGX/TDX Root CA certificate from [https://api.trustedservices.intel.com/sgx/certification/v4/rootcacrl](https://api.trustedservices.intel.com/sgx/certification/v4/rootcacrl).
3. Sigstore Rekor public key: 32-byte Ed25519 public key from [https://rekor.sigstore.dev/api/v1/log/publicKey](https://rekor.sigstore.dev/api/v1/log/publicKey).

Each value is fetched and verified at release-cut time; CI drift-check workflow (`.github/workflows/drift-check.yml`) runs weekly, refetches each value, diffs against the pinned constant, and opens a repository issue on mismatch.

**Rationale**: Pin-at-release with drift monitoring is the industry-standard pattern for security-critical root material (e.g., browsers pin CT log keys this way, Sigstore clients pin Rekor keys this way, `rustls-native-certs` uses platform trust stores but specific-purpose clients pin). Fetch-at-startup would add a trust-on-first-use vulnerability and a network dependency on daemon boot. Pure-manual-review would miss silent rotations.

**Alternatives considered**:
- **Multi-fingerprint list with any-match**: Useful during a rotation window; not needed yet because AMD/Intel rotate infrequently and the drift check provides > 7-day warning.
- **`production` feature flag vs. compile-time assert**: Ultimately use both — `#[cfg(feature = "production")]` gates the fail-build assertion so test builds can exercise bypass paths.

**Implementation notes**:
- Record the fetch URL and the DER digest verification in `docs/releases.md` as part of the release procedure.
- The drift-check workflow uses GitHub's `gh issue create` to open the issue on mismatch and assigns it to `@ContextLab/release-engineers`.

---

## 4. Real Firecracker rootfs assembly (FR-012 – FR-014)

**Decision**: Assemble the rootfs via a four-stage pipeline:
1. Pull OCI layers from CID store by hash, validate each against its declared digest.
2. `mkfs.ext4` against a sparse file of declared size (default 1 GB, configurable per-manifest).
3. Loopback-mount the file at a temporary mount point using `losetup` + `mount -o loop,rw`.
4. Extract each OCI layer (tar.gz) onto the mounted filesystem in order, applying whiteouts per OCI image spec v1.0.

Use the `tar` crate for extraction and `oci-spec` 0.7 for manifest parsing. Use `nix::mount::mount` (Linux-only) for programmatic mount without shelling out. Fail closed on any error and clean up loopback devices via a scope-guard drop pattern so aborted assemblies never leave orphaned devices.

**Rationale**: This is the canonical OCI-to-ext4 pipeline used by containerd, Kata Containers, and Firecracker's own devtool examples. It is the minimum viable real rootfs — anything less would fail to boot. Using the crates above (already widely deployed) instead of shelling out to `e2fsprogs` + `tar` keeps error handling structured and avoids quote-escaping bugs.

**Alternatives considered**:
- **virtio-fs instead of ext4**: Would be better for development iteration but adds vhost-user-fs daemon complexity and is not yet as well-supported in Firecracker as block devices.
- **Use BlockIO mode without a filesystem**: Only works for statically-linked init processes; defeats the generality of OCI-image workloads.
- **Shell out to `umoci`**: Adds a non-Rust dependency; `oci-spec` + `tar` in-process gives same functionality.

**Implementation notes**:
- `mkfs.ext4` does need to be shelled out (there is no pure-Rust ext4 formatter); mark this as a required system binary in the agent's install check.
- The cleanup path must `umount` before `losetup -d`; reverse order is a common bug.
- For the Firecracker `boot_args`, use `init=/sbin/init console=ttyS0 reboot=k panic=1 pci=off` as the baseline (Firecracker's canonical settings).

---

## 5. Real CPU+GPU+memory load metric (FR-033)

**Decision**: Implement `current_load()` as a weighted combination of:
- `sysinfo::System::global_cpu_info().cpu_usage()` → CPU load (0.0–1.0)
- `nvml_wrapper::Nvml::device_count()` + per-GPU `utilization_rates()` → GPU load (0.0–1.0, max across devices)
- `sysinfo::System::memory_usage_percent()` / 100.0 → memory load
- Return `max(cpu, gpu, mem)` so the most loaded resource dominates

Cache the result for 500 ms to avoid per-heartbeat overhead.

**Rationale**: `sysinfo` 0.33 is cross-platform (Linux/macOS/Windows) and actively maintained. `nvml-wrapper` is NVIDIA's official bindings via NVML. Using `max(...)` is correct for the sovereignty-yield decision because the donor experiences the worst-loaded resource. AMD GPU support via `rocm_smi_lib` is deferred to a follow-up because current volunteer hardware is dominantly NVIDIA.

**Alternatives considered**:
- **Just CPU**: Misses GPU saturation which is where volunteer workloads live.
- **Custom per-platform code paths**: `sysinfo` already does this; re-implementing is reinventing.
- **OpenTelemetry metrics only**: Metrics are for monitoring; sovereignty decisions need a synchronous read.

**Implementation notes**:
- Wrap the NVML calls in a `OnceCell<Option<Nvml>>` so nodes without NVIDIA GPUs do not pay startup cost.
- Metal-based Apple Silicon GPU load is exposed via `IOKit`; defer to follow-up and return 0.0 for GPU on macOS initially.

---

## 6. TPM2-backed key sealing (FR-034)

**Decision**: Use `tss-esapi` 7.x (Parsec project / IBM TSS 2.0 Rust bindings) to implement `seal(plaintext, pcr_policy) → sealed_blob` and `unseal(sealed_blob) → plaintext`, binding the seal to a PCR policy that includes PCR0 (firmware) + PCR7 (secure-boot state). On non-TPM systems, fall back to file-backed software sealing with a clear trust-tier downgrade. On `--attested-release-only` deployments, *remove* the function entirely because attested-key-release subsumes it.

**Rationale**: `tss-esapi` is the only actively-maintained Rust TPM2 binding. PCR0+PCR7 policy is the industry-standard "seal to the current boot state" binding — widely used by BitLocker, Clevis, sbctl. The "remove if attested-release subsumes" path is spec-compliant (FR-034 explicitly allows removal); research should revisit whether the attested-key-release path (already in spec 004) makes the TPM path redundant and lean toward removal if so to minimize complexity.

**Alternatives considered**:
- **SEV-SNP firmware-backed secrets only**: Works on AMD SEV systems but not on Intel TDX or commodity hosts.
- **Software sealing only**: Defeats the purpose of "safety first" on TPM-capable hosts.
- **Custom C bindings to tpm2-tools**: Adds a native build dependency; the Rust bindings are good enough.

**Implementation notes**:
- Defer the final "seal vs. remove" decision to implementation phase after reading `src/data_plane/confidential.rs` and confirming whether the attested-release path is already production-ready.
- If keeping TPM2: ship a `tpm2-tools` dependency check in the installer and fall back to software sealing with a warning on non-TPM hosts.

---

## 7. Masked-discrete-diffusion backbone selection (FR-022)

**Decision**: Target **LLaDA 8B** (ML-GSAI, arXiv:2502.09992) as the initial backbone. Fallback/alternatives: **Dream 7B** (Hkunlp, arXiv:2508.15487) if LLaDA weights are less accessible at implementation time; **DiffuLLaMA** (arXiv:2410.17891) as a third option.

**Rationale**: LLaDA 8B has the most mature Hugging Face ecosystem presence at time of writing (HF: `GSAI-ML/LLaDA-8B-Base` and `GSAI-ML/LLaDA-8B-Instruct`) with Apache-2.0-compatible research license and strong planning/reasoning benchmarks. Dream 7B is neck-and-neck on benchmarks but initialized from Qwen2.5 7B (slight license-clarity wrinkle). DiffuLLaMA is initialized from LLaMA (more restrictive license). All three share the same masked-discrete-diffusion formalism so the composition code (PCG, ParaDiGMS) is backbone-agnostic.

**Alternatives considered**:
- **Mercury Coder (Inception Labs, commercial)**: Closed-weights; disqualified.
- **Non-diffusion alternatives (Qwen2.5, Mistral, Llama-3.1)**: The whitepaper explicitly argues AR ensembling is strictly inferior to diffusion ensembling; AR is not on the roadmap.
- **Train our own 7B diffusion model**: Out of budget.

**Implementation notes**:
- Use the Hugging Face safetensors format; the agent pulls weights from a CID-addressed mirror after first-run.
- Implement as a feature-gated module: the backbone crate is optional behind `--features mesh-llm-diffusion` so non-GPU donors don't pay the ~12 GB dependency closure.

---

## 8. Diffusion inference runtime (FR-022, FR-023)

**Decision**: Use `candle-core` + `candle-nn` + `candle-transformers` 0.7+ (HuggingFace's pure-Rust ML framework) as the primary inference runtime. Provide an optional path for PyTorch inference via `tch` 0.17 (torch-sys bindings) for operators who prefer the Python ecosystem's model zoo.

**Rationale**: `candle` is actively maintained by Hugging Face, supports CUDA and Metal backends, and is already in our dep tree for the existing mesh_llm code (even though that code is architecturally incorrect). It gives us pure-Rust masked-diffusion inference without a Python dependency. `tch` is the escape hatch if a specific research model hasn't been ported to candle.

**Alternatives considered**:
- **vllm-rs**: Focused on AR inference; diffusion not first-class.
- **burn**: Less mature than candle for LLM-class models.
- **Pure PyTorch via pyo3**: Python interpreter in the agent process is a safety-audit nightmare.

**Implementation notes**:
- Pin candle to ≥ 0.7 (Metal backend is stable there).
- Use int8/int4 quantization (GGUF or AWQ) if available for the chosen backbone to keep per-node VRAM within tensor01/tensor02's 3-GPU-per-job budget.

---

## 9. PCG score composition (FR-023, FR-024)

**Decision**: Implement PCG per Bradley & Nakkiran (TMLR 2025, arXiv:2408.09000): at each denoising step `t`, compute the DDIM predictor `x̂_0^{(pred)}` and the Langevin corrector updates with per-expert specialization weights `{w_e}`. The composed score is:

```
s_composed(t) = Σ_e w_e · s_e(x_t, t, c_e)
```

where `c_e` is the expert's conditioning context, subject to a clipping bound `||s_e(x_t, t, c_e)||_∞ ≤ τ(t)` that prevents any single expert from dominating.

**Rationale**: PCG is the composition rule the whitepaper points to as theoretically grounded. The clipping bound addresses the Razafindralambo et al. (TMLR 2026, arXiv:2601.11444) result that naive mean-averaging fails on FID. Per-expert weights are the specialization channel — operators can tune them per task domain.

**Alternatives considered**:
- **Uniform mean-averaging**: Explicitly ruled out by FR-024.
- **Hard expert selection (K-of-N with winner-take-all)**: Sacrifices the smooth bidirectional context integration that the whitepaper identifies as diffusion's superpower.
- **Learned combiner network**: Research-grade; adds training complexity not justified at this phase.

**Implementation notes**:
- Expose `w_e` and `τ(t)` as per-request parameters so benchmarks can vary them.
- Log per-step clipping activations for auditability (Edge Case: > 10 % clipping triggers an observability event).

---

## 10. ParaDiGMS parallel denoising (FR-025)

**Decision**: Implement Picard-iteration parallel denoising per Shih et al. (NeurIPS 2023, arXiv:2305.16317): given a denoising schedule of `T` steps, guess the full trajectory `{x_1, x_2, ..., x_T}`, compute the residual `R_t = x_{t-1} − Denoise(x_t, t)` across all steps in parallel, and iterate the fixed-point until `||R||_∞ < ε`. Target 4–8 parallel-step blocks with convergence threshold `ε = 1e-3` and max iterations `K = 10` before falling back to sequential.

**Rationale**: This is exactly the construction that gives 2–4× wall-clock speedup in the paper. The convergence-budget-with-fallback pattern gives a hard worst-case bound (fall back to sequential if Picard doesn't converge within `K` iterations) so no single pathological prompt can stall the swarm.

**Alternatives considered**:
- **Jacobi iteration**: Slower convergence than Picard in practice.
- **Parallel sampling via ODE solvers (DPMSolver++)**: Reduces step count but not parallelism-per-step.

**Implementation notes**:
- Parallelism unit is one denoising step → one GPU in the swarm.
- Expose convergence metrics as telemetry; Edge Case requires explicit fallback logging.

---

## 11. DistriFusion stale-activation pipelining (FR-026)

**Decision**: Implement DistriFusion per Li et al. (CVPR 2024, arXiv:2402.19481): when GPU A needs GPU B's activations at timestep `t`, A uses B's timestep `t-1` activations (already delivered) rather than waiting for fresh ones. The staleness window is configurable (default 1 step; max 3). Implement over libp2p using a request-response protocol parallel to `TaskDispatch`: `/worldcompute/diffusion-activation/1.0.0` carrying CBOR-encoded activation tensors.

**Rationale**: This is the paper's exact recipe. The claimed 6.1× speedup on 8-A100 SDXL demonstrates the pipelining pattern works in practice. Running it over libp2p (instead of NCCL / GLOO) makes it WAN-compatible, which is the whole point for the volunteer swarm.

**Alternatives considered**:
- **Synchronous NCCL collectives**: Only works in tightly-coupled data centers.
- **All-reduce per step**: Defeats the staleness-hiding property.

**Implementation notes**:
- Use the existing libp2p request-response CBOR infrastructure (spec 004's `TaskDispatch`) as the template.
- Compress activation tensors with zstd before transmission (typical 2–3× reduction for fp16 activations).
- Measure RTT-masking percentage in the benchmark per FR-026.

---

## 12. Real 72-hour churn harness (FR-017)

**Decision**: Build `scripts/churn-harness.sh` as a multi-process local harness that spawns N (default 10) real `worldcompute donor join --daemon` processes on the local machine plus peers on tensor01 and tensor02 via SSH, randomly kills and restarts them on a Poisson schedule tuned to 30 % rotation/hour, and asserts ≥ 80 % job completion over a 72-hour window. Submit workloads at a steady rate (1/minute) from a driver process. Emit one ledger dump per hour as evidence.

**Rationale**: This converts the current statistical simulator into a *real* churn test that exercises the actual libp2p code, Raft coordinator, CRDT merge, BLS threshold signing. Running some peers locally and some on real remote machines gives a realistic mix of transport paths.

**Alternatives considered**:
- **Chaos Mesh / Litmus**: Over-engineered for a shoestring-budget deployment.
- **Simulated libp2p only**: Defeats the point — the simulator is what we're replacing.

**Implementation notes**:
- Log every kill + restart with timestamp for post-hoc analysis.
- The full 72-hour run is an evidence-artifact producer, not a CI check; CI runs a 1-hour smoke version.

---

## 13. Reproducible builds (FR-043)

**Decision**: Build on GitHub Actions Linux runners using a Nix-based deterministic build environment (via `cachix/install-nix-action`). Set `SOURCE_DATE_EPOCH` to the commit timestamp, pin the Rust toolchain exactly via `rust-toolchain.toml`, and use `cargo-auditable` to embed dependency SBOM. Build on two independent runners and diff the output binaries with `diffoscope`; fail on any difference.

**Rationale**: Nix gives hermetic builds that are bit-identical across runners when the inputs are identical. `SOURCE_DATE_EPOCH` is honored by rustc for embedded timestamps. `cargo-auditable` embeds the Cargo.lock so the SBOM is inline. This pattern is used by Arti (the Rust Tor reimplementation) and Rust-for-Linux.

**Alternatives considered**:
- **Docker Buildx**: Less deterministic than Nix; layer caching varies.
- **Bazel**: Adds a whole new build system; over-engineering.
- **Trust-me-bro release builds**: Defeats the security property.

**Implementation notes**:
- Accept that macOS and Windows reproducible builds are harder; initial reproducible-build mandate is Linux-only, with macOS/Windows targeted as a follow-up.
- Signatures are detached Ed25519 per artifact; the release public key is pinned in a new constant `RELEASE_PUBLIC_KEY` and shipped in `scripts/verify-release.sh`.

---

## 14. Evidence artifact format (FR-015, FR-016, FR-020a, FR-028a, others)

**Decision**: Every evidence-producing script writes to `evidence/phase1/<area>/<timestamp>/` and emits:
- `run.log` — full stderr/stdout captured during the run
- `metadata.json` — structured metadata (machine IDs, software versions, start/end times, git SHA)
- `results.json` — structured pass/fail per assertion with measured values
- `trace.jsonl` — NDJSON event trace for replay (ledger writes, dispatches, failures)
- Optional `screenshots/*.png` for GUI evidence
- An `index.md` linking all the above, written in the format expected by `docs/releases.md`

**Rationale**: This mirrors the constitution's Principle V requirement for direct-test evidence artifacts and gives a consistent structure reviewers can grep across. JSONL traces are replayable.

**Alternatives considered**:
- **Prometheus + Grafana snapshots**: Nice for monitoring but not for reviewable evidence; add as secondary.
- **OpenTelemetry traces as primary evidence**: Valid; keep in mind for phase 2 if the JSONL format gets unwieldy.

**Implementation notes**:
- Evidence directories MUST be committed to the repository (small sizes: < 10 MB per run is the soft limit).

---

## 15. Placeholder-allowlist tooling (FR-038, SC-006)

**Decision**: `.placeholder-allowlist` is a newline-separated text file at the repository root. Each non-empty line is of the form:

```
<path>:<line_number> — <rationale>
```

`scripts/verify-no-placeholders.sh` greps `src/` for the placeholder tokens, reads the allowlist, and fails the build on any occurrence not matching `path:line` in the allowlist. At spec-005-completion the file MUST exist and MUST be empty (zero lines). PRs that introduce an allowlist entry require a rationale in the PR description.

**Rationale**: Simple, human-readable, diffable, grep-able. No YAML indentation footguns. No tool dependency beyond `grep` and `bash`.

**Alternatives considered**:
- **Rust proc-macro attribute**: Over-engineered; the check is a grep.
- **`#[allow(clippy::todo)]` style**: Doesn't cover doc-comments or the `TODO` / `stub` tokens.

**Implementation notes**:
- The script runs on every PR via `.github/workflows/verify-no-placeholders.yml`.
- Doc-comments that genuinely describe historic context (e.g., "this module replaced an earlier stub") are allowed to use the token but MUST be listed in the allowlist during spec-005-development. **After spec 005 closes**, the allowlist may hold a handful of such entries; during spec 005 it MUST be empty.

---

## Resolved NEEDS CLARIFICATION summary

- **Diffusion inference runtime**: candle 0.7+ primary, `tch` 0.17 optional.
- **Backbone model**: LLaDA 8B primary; Dream 7B / DiffuLLaMA fallbacks.
- **WSS-443 transport library**: `libp2p-websocket` + `libp2p-tls`.
- **DoH resolver library**: `hickory-resolver` 0.24+.
- **OCI extraction libraries**: `oci-spec` 0.7 + `tar` 0.4 + `nix::mount`.
- **TPM2 library**: `tss-esapi` 7.x; decision to keep-vs-remove deferred to implementation reading of current confidential compute code.
- **Load-metric library**: `sysinfo` 0.33 + `nvml-wrapper` 0.10.
- **Reproducible-build environment**: Nix (Linux-only for initial release).
- **Evidence format**: `evidence/phase1/<area>/<ts>/{run.log,metadata.json,results.json,trace.jsonl,index.md}`.
- **Allowlist format**: `.placeholder-allowlist` at repo root, `path:line — rationale` lines, empty at spec-005 completion.

All NEEDS CLARIFICATION flags in Technical Context are now resolved. Phase 1 can proceed.
