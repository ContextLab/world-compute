# Release Engineering — spec 005 procedures

This document is the authoritative checklist for cutting a tagged World Compute release. Every requirement below is enforced by CI or by the release script; this file is the *why* and *how*, CI is the *what*.

## 1. Pre-release drift check

Before cutting any release, the drift-check queue must be empty (FR-011a).

`.github/workflows/drift-check.yml` runs weekly (Mon 03:00 UTC) and on-demand. It:

1. Fetches AMD ARK-Milan + ARK-Genoa chains from `https://kdsintf.amd.com/vcek/v1/<gen>/cert_chain`.
2. Fetches Intel DCAP Root from `https://certificates.trustedservices.intel.com/Intel_SGX_Provisioning_Certification_RootCA.cer`.
3. Fetches Sigstore Rekor SPKI from `https://rekor.sigstore.dev/api/v1/log/publicKey`.
4. Computes SHA-256 of each and compares against the in-tree pins in `src/verification/attestation.rs` and `src/ledger/transparency.rs`.
5. Opens a repository issue tagged `drift-check` on any mismatch.

**Release gate**: no open `drift-check` issue at the time of the tag.

## 2. Build the `production`-feature binary

```bash
cargo build --release --features production
```

The `production` feature triggers compile-time assertions in `src/features.rs` that fail the build if any pinned fingerprint is still `[0u8; 32]`. This is the single safety gate that prevents shipping a binary that silently bypasses attestation.

## 3. Reproducible build

For each release tag, the reproducible-build CI workflow (`.github/workflows/reproducible-build.yml`) spins up two independent Ubuntu 24.04 runners with identical Nix-based toolchains, builds the binary on each, and runs `diffoscope` on the output artifacts. Any byte-level difference fails the release.

Required inputs:

- `SOURCE_DATE_EPOCH` = commit timestamp (derived from `git log -1 --format=%ct`)
- `rust-toolchain.toml` pinning exact rustc version
- `cargo-auditable` builds embed the Cargo.lock into the binary

## 4. Sign the release

```bash
ops/release/sign-release.sh <artifact> <release-private-key.pem> > <artifact>.sig
```

Produces a detached Ed25519 signature using the release private key (held offline; only the release engineer touches it). The public key is pinned as `RELEASE_PUBLIC_KEY` in `ops/release/verify-release.sh` and in the README.

Operators verify with:

```bash
ops/release/verify-release.sh <artifact> <artifact>.sig
```

## 5. Evidence artifact requirements

A release may be marked `stable` only when every SC with a real-hardware requirement has at least one `overall: pass` evidence bundle committed on the tagged commit:

| SC | Area | Evidence location |
|-|-|-|
| SC-001 (cross-firewall mesh) | firewall-traversal | `evidence/phase1/firewall-traversal/<ts>/` |
| SC-003 (deep attestation) | attestation | `evidence/phase1/attestation/<ts>/` |
| SC-004 (real Firecracker) | firecracker-rootfs | `evidence/phase1/firecracker-rootfs/<ts>/` |
| SC-005 (72h churn) | churn | `evidence/phase1/churn/<ts>/` |
| SC-008 (quickstart) | quickstart | `evidence/phase1/quickstart/<platform>/<ts>/` |
| SC-010 (diffusion mesh-LLM) | diffusion-mesh | `evidence/phase1/diffusion-mesh/<ts>/` |

The FR-020a cloud-adapter live run must have evidence for each provider it targets (AWS/GCP/Azure) committed under `evidence/phase1/cloud-adapter/<provider>/<ts>/`.

Run `scripts/validate-evidence.sh <dir>` on each bundle before the release tag to confirm structure.

## 6. Placeholder completion gate (spec-005 closing gate)

```bash
scripts/verify-no-placeholders.sh --check-empty
```

Exit code must be 0. `.placeholder-allowlist` must be empty. This is the single binary check that determines whether spec 005 has passed.

After spec 005 closes, the `--check-empty` gate is dropped from CI; `.placeholder-allowlist` may accumulate legitimate historic-context entries going forward, but no release may ship with a non-empty allowlist on `main` until a future spec explicitly re-invokes the completion gate.

## 7. Release checklist (executed by release engineer)

- [ ] No open `drift-check` issues.
- [ ] `cargo build --release --features production` succeeds.
- [ ] `cargo test --features production` passes (test count ≥ 900).
- [ ] `cargo clippy --lib --tests --features production -- -D warnings` passes.
- [ ] `scripts/verify-no-placeholders.sh --check-empty` exits 0.
- [ ] Reproducible-build CI on current HEAD is green.
- [ ] Every required evidence bundle from §5 is present for the current commit.
- [ ] `ops/release/sign-release.sh` produces signatures for every shipped binary.
- [ ] `ops/release/verify-release.sh` passes on the produced signatures.
- [ ] Tag is pushed; GitHub release notes link to each evidence bundle.

## 8. Post-release monitoring

- `verify-no-placeholders.yml` runs on every PR thereafter.
- Drift-check continues weekly.
- If a fingerprint rotation is detected, the drift-check issue is the operator's signal to cut a patch release within the documented response window (target: 7 days).

## 9. Rollback

If a released binary is found to regress safety (Principle I), donor sovereignty (Principle III), or data integrity (Principle II), the release engineer:

1. Marks the GitHub release as `pre-release` (hides from "latest").
2. Emits a governance `EmergencyHalt` proposal per constitution Emergency Powers.
3. Publishes a rollback advisory within 24 hours.
4. Retracts the signed artifact from distribution mirrors.

## References

- [specs/005-production-readiness/spec.md](../specs/005-production-readiness/spec.md)
- [specs/005-production-readiness/contracts/evidence-artifact-format.md](../specs/005-production-readiness/contracts/evidence-artifact-format.md)
- [specs/005-production-readiness/contracts/ci-verify-no-placeholders.md](../specs/005-production-readiness/contracts/ci-verify-no-placeholders.md)
- [.specify/memory/constitution.md](../.specify/memory/constitution.md)
