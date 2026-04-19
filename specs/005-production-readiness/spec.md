# Feature Specification: Production Readiness — eliminate all placeholders and cross firewalls

**Feature Branch**: `005-production-readiness`
**Created**: 2026-04-19
**Status**: Draft
**Input**: User description: "address issue 57 and all sub issues (make sure to read comments of all issues; you'll see notes about the current status of each-- although some might be stale, so you need to also verify!) AND issue 60"

## Clarifications

### Session 2026-04-19

- Q: What is the policy for maintaining the pinned AMD ARK / Intel DCAP / Sigstore Rekor constants across releases? → A: Pin at release time; a CI job periodically refetches from upstream and opens an issue on change.
- Q: What is the fallback-relay hosting model for the release that closes this spec? → A: Project operates 1–2 fallback relays at launch for bootstrap; documented path for volunteers to augment or replace them.
- Q: How is the real cloud-adapter end-to-end test gated? → A: Use the cheapest/freest available option per cloud (AWS free tier, GCP free tier / $300 credit, Azure free tier / $200 credit, or ephemeral student/organization credits if the paid options cannot be avoided). The workflow MUST be triggered either automatically on tagged release OR manually by a repository maintainer/admin/owner via a guarded GitHub Actions `workflow_dispatch` — no other roles can invoke it. Evidence committed per release.
- Q: What architecture does the mesh LLM actually use? → A: **Distributed diffusion**, not autoregressive transformer ensembling. Per `notes/parallel_mesh_of_diffusers_whitepaper.pdf`, the system combines: (a) a Dream-class 7B masked-diffusion LM backbone (Dream 7B / LLaDA 8B / DiffuLLaMA or comparable open-weights masked-diffusion model), (b) SSD-2-style specialization-weighted conditional ensembling of small domain experts contributing per-step score signals combined via PCG (Predictor-Corrector Guidance) — NOT uniform mean-averaging, (c) ParaDiGMS-style parallel denoising across time via Picard iteration, (d) DistriFusion-style stale-activation pipelining to hide WAN latency behind compute, and (e) Petals-style sharded hosting over libp2p with DCUtR hole-punching. The current `src/agent/mesh_llm/` code (router, aggregator, expert, service — all top-K sparse logits over autoregressive experts) is therefore architecturally incorrect and MUST be replaced in this spec, not merely completed.
- Q: What is the minimum real-hardware configuration that counts as "diffusion mesh-LLM smoke test passed"? → A: 3 GPUs on tensor01 (backbone + 2 experts) + 3 GPUs on tensor02 (3 more experts) = 6 total diffusion workers across 2 real machines with a real cross-machine libp2p connection; WAN latency for the DistriFusion-pipelining benchmark is controlled via `tc qdisc netem` emulating 100 ms RTT. All four claims (ParaDiGMS ≥ 2× speedup, DistriFusion masking ≥ 50 % RTT, PCG composition with ≥ 2 experts, end-to-end correctness on constraint-satisfaction / planning / code-infilling prompt) MUST be demonstrated on this footprint.
- Q: How strictly does CI enforce "zero production placeholders"? → A: Hard block with an explicit path:line allowlist reviewed in each PR that adds an entry — but **during the implementation of spec 005 itself, NO allowlist entries are permitted**. The allowlist mechanism exists solely for long-term maintenance after spec 005 closes (e.g., to exempt a doc-comment that legitimately describes historic context). For spec 005 to be declared complete, every current placeholder/stub/TODO must be ELIMINATED, not exempted. The CI check MUST fail the build if: (a) any new occurrence is introduced without an accompanying allowlist addition, OR (b) the allowlist file contains any entry at the moment of the spec-005 "implementation complete" gate.

## Background (verified 2026-04-19)

The World Compute codebase (main branch, post-merge of PR #59) consists of 94 Rust source files and 802 passing tests across Linux/macOS/Windows CI. Specs 001–004 have shipped the full architectural skeleton: WASM sandbox, Firecracker/Apple VF drivers, libp2p P2P daemon with NAT-traversal stack, request-response TaskDispatch protocol, CRDT ledger with BLS threshold signing, OAuth2/BrightID identity, Sigstore Rekor scaffolding, TPM2/SEV-SNP/TDX certificate-chain verification, Raft coordinator consensus, 10-step policy engine, and Tauri GUI shell.

However, direct code inspection on 2026-04-19 confirms the sub-issue comments on master #57 are still accurate: **16 subsystems have protocol-correct scaffolding but contain explicit placeholders in their critical paths that would prevent real production operation.** In addition, **issue #60** (closed → reopened as blocker) documents that the production mesh has been validated only in-process over `127.0.0.1`; cross-machine mesh formation behind real institutional firewalls has not been demonstrated and, when attempted from `tensor02.dartmouth.edu`, failed silently.

The set of confirmed placeholder sites (grep evidence in session notes):

- `src/verification/attestation.rs:30,34` — `AMD_ARK_SHA256_FINGERPRINT` and `INTEL_ROOT_CA_SHA256_FINGERPRINT` are `[0u8; 32]`; validator bypasses fingerprint pin when zero (#28).
- `src/ledger/transparency.rs:19` — `REKOR_PUBLIC_KEY` is `[0u8; 32]`; `verify_tree_head_signature` bypasses verification when zero (#29, #56).
- `src/agent/lifecycle.rs:136` — heartbeat/pause/withdraw return payloads but do not broadcast over gossip (#30).
- `src/sandbox/firecracker.rs` — `assemble_rootfs` concatenates layer bytes; does not run `mkfs.ext4`, loopback-mount, or extract OCI tar (#33).
- `src/governance/admin_service.rs:81` — `ban()` returns `Ok(())` without updating trust registry (#34).
- `src/agent/mesh_llm/expert.rs:138` — `load_model()` is explicitly a placeholder; no real inference (#27, #54). **Additionally: the entire existing `src/agent/mesh_llm/*.rs` module is architecturally incorrect per `notes/parallel_mesh_of_diffusers_whitepaper.pdf` — it implements autoregressive top-K logit ensembling, but the project's actual mesh-LLM design is distributed masked-discrete-diffusion. The module MUST be replaced (not completed) with diffusion primitives.**
- `src/agent/mesh_llm/service.rs:27` — self-labeled "stub — no real inference yet"; also architecturally on the wrong path (AR-ensembling rather than diffusion swarm).
- `src/verification/receipt.rs:28` — receipt verification is "stub"; coordinator public key not wired.
- `src/agent/daemon.rs:501` — `current_load()` returns a fixed 0.1.
- `src/data_plane/confidential.rs:163` — key sealing is "simplified placeholder".
- `src/sandbox/apple_vf.rs:176,239` — writes `b"placeholder-disk"` on non-macOS.
- `src/governance/governance_service.rs` — SubmitProposal/CastVote RPC handlers are "stub".
- `src/policy/rules.rs:453`, `src/policy/engine.rs:236` — signature fields filled with `vec![0u8; 64]` placeholder before resign step.
- Platform adapters (Slurm #37, Kubernetes #38, Cloud #39) — parsers exist; never exercised against live systems.
- Tauri GUI (#40), Dockerfile + Helm chart (#41), REST gateway HTTP listener (#43) — artifacts exist; never built/run/bound.
- Churn simulator (#51) — statistical model, not real kill-rejoin harness over libp2p.
- Apple VF Swift helper (#52) — Package.swift exists; binary never built.
- Reproducible builds (#53) — signature plumbing exists; no two-independent-build verification, no production-signed binary.
- Cross-machine firewall traversal (#60) — only validated in-process on `127.0.0.1`; tensor02 real-network test failed silently.

This spec closes every one of those gaps. Its north star, per user directive, is that **no TODO, no placeholder, no untested code path remains in the production agent**, and the mesh forms reliably across real institutional firewalls.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Cross-firewall mesh formation on real hardware (Priority: P1)

A volunteer administrator at a university, national lab, enterprise, hospital, or government site — whose machine sits behind a stateful institutional firewall with default-deny outbound — installs the donor agent and joins the World Compute mesh without requesting any firewall change, port forward, paid relay, or manual multiaddr. From that moment on, other donors elsewhere on the public internet can dispatch real WASM jobs to that machine, and jobs that machine submits land on remote executors, using only the long-lived daemon process and outbound-initiated connections the firewall would normally permit to any web service.

**Why this priority**: This is the binding constraint on the project's mission. The majority of high-value potential donors (universities, labs, enterprises, cloud tenants) sit behind firewalls equivalent to or stricter than Dartmouth's. If cross-firewall participation does not work out of the box, none of the rest of the system matters. This is issue #60, which the user designated "the north star for spec 005."

**Independent Test**: Deploy the daemon on `tensor02.dartmouth.edu` (real institutional firewall, verified-hostile to libp2p in spec 004 testing). Leave it running in the foreground for at least 10 continuous minutes. From a second machine on a different network (laptop on home ISP, cloud VM, or a cooperating peer), dial `tensor02` via its reserved relay-circuit address, send a TaskDispatchRequest carrying a real WASM workload, and confirm the response comes back with `TaskStatus::Succeeded` and the expected result bytes. Capture a log trace and commit it as an evidence artifact under `evidence/phase1/firewall-traversal/`.

**Acceptance Scenarios**:

1. **Given** a fresh donor agent installed on a machine behind a stateful institutional firewall, **When** the operator runs `worldcompute donor join --daemon` with no additional network configuration, **Then** within 60 seconds the daemon establishes and maintains at least one long-lived connection to a bootstrap relay, and a log line records the relay-circuit address it has reserved.
2. **Given** two donor agents on two different networks, both behind independent firewalls, and both holding relay reservations, **When** a job is submitted from one to the other using only the reserved-circuit address, **Then** the WASM job executes on the target and the submitter receives a valid `TaskDispatchResponse` with a coordinator-signed receipt.
3. **Given** a donor agent that has held a reservation for at least 10 minutes, **When** the test suite captures a 5-minute debug-level log, **Then** the log contains zero silent dial failures — every `libp2p_swarm::DialFailure` event is surfaced either as a retry with backoff or as a documented fallback (QUIC → TCP → WebSocket-over-443).
4. **Given** a firewall that blocks all outbound traffic except HTTP/HTTPS on 443, **When** the donor agent is started on a machine behind that firewall, **Then** the agent still forms the mesh by negotiating a WebSocket-over-TLS transport on port 443 as the final fallback, with the fallback path and reason written to the log.

---

### User Story 2 - Deep attestation with pinned root CAs (Priority: P1)

A workload submitter requires that every node executing their job prove its hardware root of trust. When a donor with an AMD SEV-SNP or Intel TDX-capable host enrolls, the coordinator validates the full certificate chain — including matching the root CA against a **pinned manufacturer fingerprint** — and records the result in the transparency log with a **verifiable Rekor signed tree head**. Nodes whose attestation chains cannot be anchored to a real manufacturer root are rejected, not silently downgraded.

**Why this priority**: Safety First (constitution principle I). Today the validator enters permissive bypass when the pinned fingerprint is `[0u8; 32]`, which means in practice no attestation is ever rejected for chain-of-trust reasons. This turns the attestation story from a real safety property into ceremony. Same for Rekor: the tree head signature check is bypassed when the public key is all zeros, so the transparency log provides no guarantee. Both are single-line fixes *once the real fingerprints and key are pinned* (#28, #29).

**Independent Test**: Replace the three constants with real production values fetched from AMD (ARK SHA-256), Intel (DCAP root CA SHA-256), and the Sigstore public Rekor instance (Ed25519 public key). Run the existing `tests/verification/test_deep_attestation.rs` against a real AMD SEV-SNP quote (obtained from an EPYC host with `snpguest report`) and against a tampered copy; expect PASS on the real quote and REJECT on the tampered one. Run `tests/transparency/test_rekor_proof.rs` against a real inclusion proof fetched from `https://rekor.sigstore.dev`; expect verification success. Run both tests against zero-byte fingerprints / keys; expect the validator to refuse to start rather than enter bypass mode.

**Acceptance Scenarios**:

1. **Given** an attestation quote signed by a real AMD EPYC processor, **When** the validator inspects the chain, **Then** it anchors to the pinned AMD ARK fingerprint and returns `trust_tier >= 2` (or the appropriate production tier per the trust model).
2. **Given** an attestation quote with a tampered signature, **When** the validator inspects it, **Then** it returns `trust_tier == 0` and emits a structured error (not a bypass warning).
3. **Given** a Rekor log entry fetched from `https://rekor.sigstore.dev`, **When** the transparency verifier checks it, **Then** both the Merkle inclusion proof AND the signed tree head signature verify against the pinned Rekor public key.
4. **Given** any of the three pinned constants is still `[0u8; 32]` at compile time, **When** the binary is built with the `production` cargo feature, **Then** the build fails with a compile-time error — bypass mode is only available in test builds.

---

### User Story 3 - Real Firecracker rootfs from CID store OCI images (Priority: P1)

A workload submitter ships an OCI image referenced by CID. On a Linux host with KVM and Firecracker, the donor agent pulls the layers from the CID store, assembles a real bootable `ext4` rootfs on a loopback device, boots Firecracker with that rootfs, runs the entrypoint inside the microVM, and reads back the exit code and stdout — over vsock, not the concatenated bytes of the layers. A real job that `/bin/sh -c "echo hello; exit 0"` executes in Firecracker and returns `"hello\n"` + exit 0.

**Why this priority**: Firecracker is the primary VM-level isolation path for Linux donors. Today `assemble_rootfs` writes layers' raw bytes into a file — a real Firecracker boot with that file would fail at UEFI/kernel initrd stage. Until this is fixed, the Linux VM driver cannot run anything (#33).

**Independent Test**: Build a minimal OCI image (`scratch` + a 200-byte static-linked `hello` binary), push it into the CID store, request execution via the donor CLI on a Linux KVM host with swtpm already in CI, and assert: (a) the rootfs mounts, (b) Firecracker boots past kernel → init, (c) the entrypoint runs, (d) stdout "hello\n" is returned via vsock, (e) Firecracker shuts down cleanly.

**Acceptance Scenarios**:

1. **Given** a valid OCI image stored as CIDs in the data plane, **When** the donor assembles its rootfs, **Then** the resulting file is a real ext4 filesystem containing the extracted layer contents (verifiable with `file` and `fsck.ext4`).
2. **Given** a Firecracker VM configured with that rootfs, **When** the microVM boots and executes the image's entrypoint, **Then** the stdout and exit code are captured via vsock and returned to the caller within a bounded wall-clock budget.
3. **Given** an invalid or corrupted layer, **When** the donor tries to assemble the rootfs, **Then** the assembly fails with a specific error (bad tar, checksum mismatch, or permission denied) — never silently succeeds with an unbootable file.

---

### User Story 4 - End-to-end Phase 1 LAN testnet with three real machines (Priority: P1)

Three real machines — for example `tensor01`, `tensor02`, and a laptop (or three institutional hosts chosen by the operator) — all running the production binary, form a mesh, accept jobs from each other, survive one node going offline mid-job, and demonstrate a full 72-hour churn run at 30 percent node rotation with at least 80 percent job completion. The entire run is recorded (logs + ledger dump + gossipsub trace) and committed as an evidence artifact.

**Why this priority**: Constitution principle V (Direct Testing) requires real-hardware validation before any release phase gate. Issue #42 is the canonical Phase 1 milestone. Issue #51 is the 72-hour churn run, which today is only a statistical model — never actually executed across real libp2p processes. Both must be real before the project can claim a working federation.

**Independent Test**: Operator runs `scripts/e2e-phase1.sh` with three hosts in a config file; script builds the binary, copies it to each host, starts the daemons, submits N workloads (mixed latency profile: ~70 % "fast" workloads with expected runtime < 5 s, ~30 % "slow" workloads with expected runtime 30–120 s, so kill events land on both in-flight short jobs and long checkpointable ones), kills and restarts nodes on a schedule driven by the churn simulator, and at the end emits (a) job completion rate, (b) per-node uptime histogram, (c) ledger consistency report, (d) gossip traffic summary. Pass criteria: completion ≥ 80% at 30% churn over 72 hours.

**Acceptance Scenarios**:

1. **Given** three production binaries running on three different real machines, **When** any single node is killed with `SIGKILL` mid-job, **Then** the scheduler re-dispatches the in-flight task to a surviving replica within the lease-expiry window (≤ 30 s) and the job completes.
2. **Given** a 72-hour churn run with 30% node rotation per hour, **When** the run completes, **Then** at least 80% of submitted jobs terminate with `Succeeded`, and no ledger invariant (balance conservation, merkle-root consistency, signature chain) is violated.
3. **Given** the evidence bundle produced by the run, **When** a reviewer replays the ledger from genesis, **Then** the replay produces the same final state, and every signed receipt verifies.

---

### User Story 5 - Real platform adapters exercised against live Slurm/K8s/cloud (Priority: P2)

An HPC operator with a Slurm cluster, a cloud-ops engineer with a Kubernetes cluster, and an infrastructure engineer with AWS/GCP/Azure VMs can each register their compute by pointing the World Compute adapter at the native control plane. The Slurm adapter submits a real `sbatch`, observes the job to completion, and reports the result up to the mesh. The Kubernetes adapter deploys the ClusterDonation CRD, the operator reconciles it, and nodes are onboarded. The cloud adapter reads the real IMDSv2 / GCE metadata / Azure IMDS endpoint, retrieves the signed identity document, and treats it as the attestation evidence for the enrolled node.

**Why this priority**: Adapters (#37, #38, #39) extend the donor base by an order of magnitude without changing the core. They are protocol-ready but never exercised against real systems. Kicking the tires on each with a real cluster / real cloud account is the gating step before claiming them as supported.

**Independent Test**: (a) A CI job that stands up a single-node Slurm cluster via `scontrol` in a container and exercises the adapter; (b) A Kind-based Kubernetes CI job that applies the CRD and asserts the operator reconciles; (c) A real AWS/GCP/Azure instance (one each) that enrolls itself using its IMDS identity document. Each produces an evidence log and asserts round-trip correctness.

**Acceptance Scenarios**:

1. **Given** a Slurm cluster reachable via slurmrestd, **When** the adapter enrolls a compute pool, **Then** a real `sbatch` is submitted, observed, and its result returned.
2. **Given** a Kubernetes cluster with the ClusterDonation CRD installed, **When** a ClusterDonation resource is applied, **Then** the operator reconciles, enrolls the cluster's nodes, and reports back via status.
3. **Given** an AWS EC2, GCE, or Azure VM, **When** the donor agent starts, **Then** it fetches the IMDS identity document, validates the cloud provider's signature, and uses the document as its attestation evidence.

---

### User Story 6 - Distributed-diffusion mesh LLM (Priority: P2)

Operators with a cluster of GPU donors can run a **distributed diffusion** language-model swarm — not an autoregressive ensemble. The architecture, per `notes/parallel_mesh_of_diffusers_whitepaper.pdf`, combines five ingredients: (a) a Dream-class 7B open-weights masked-diffusion backbone (Dream 7B, LLaDA 8B, DiffuLLaMA, or comparable), (b) SSD-2-style small specialized diffusion experts contributing per-denoising-step score signals, (c) Predictor-Corrector Guidance (PCG) as the mathematically grounded score-composition rule (explicitly NOT uniform mean-averaging, which Razafindralambo et al. proved fails on FID), (d) ParaDiGMS parallel denoising across timesteps via Picard iteration (2–4× wall-clock speedup), and (e) DistriFusion stale-activation pipelining over libp2p to hide WAN latency behind compute. A smoke test ("complete the following code" / constraint-satisfaction task / planning task) returns a coherent answer end-to-end from a real cluster.

**Why this priority**: This is the project's headline research bet and the single most differentiated capability the federation offers (distributed diffusion has no published end-to-end system yet — this would be the first). The current `src/agent/mesh_llm/` code is architecturally incorrect for this goal (top-K sparse logits are an AR-ensemble pattern) and MUST be rewritten, not completed. Scored P2 because it is not a mesh-formation blocker but is the project's defining deliverable. Not P1 because the P1 work (cross-firewall mesh, deep attestation, real Firecracker, Phase-1 cluster) is what unblocks the diffusion work from happening at all.

**Independent Test**: On 3+ GPU nodes (e.g., 3 GPUs on tensor01 + 3 GPUs on tensor02 = 6 GPU slots), load a Dream-class 7B masked-diffusion backbone on each backbone-hosting node and a handful of small SSD-2-style specialized diffusion experts on other nodes. Issue a constraint-satisfaction prompt (e.g., Countdown / Sudoku / code-infilling — the domains where diffusion LMs outperform AR per Hkunlp / arXiv:2508.15487). Assert: (a) each denoising step is computed across multiple experts in parallel, (b) the score combination uses PCG guidance weights (logged and auditable), (c) ParaDiGMS Picard iteration achieves ≥ 2× wall-clock speedup vs. strict sequential denoising on the same hardware, (d) DistriFusion activation pipelining masks at least 50 % of WAN RTT behind compute (measured by comparing pipelined vs. synchronous wall time), (e) the final output is non-empty, (f) the governance kill switch halts further denoising within one step when triggered.

**Acceptance Scenarios**:

1. **Given** a Dream-class 7B masked-diffusion model checkpoint and a GPU node, **When** the node loads the backbone, **Then** the node can produce a full score field for a masked input at any denoising timestep (not "next-token logits" — full score over the mask set).
2. **Given** a mesh of one backbone node and ≥ 2 specialized-expert nodes, **When** a prompt is submitted, **Then** each denoising step combines scores via PCG with auditable per-expert guidance weights, and the final denoised output is returned.
3. **Given** a ParaDiGMS-eligible denoising schedule, **When** inference runs across ≥ 3 GPUs, **Then** measured wall-clock time is ≥ 2× faster than a strict-sequential baseline on the same hardware.
4. **Given** two expert nodes connected over a simulated 100-ms-RTT link, **When** DistriFusion pipelining is enabled, **Then** the measured wall-clock time is within 20 % of the wall-clock time on a 1-ms-RTT link — validating latency is masked by compute.
5. **Given** a governance kill-switch vote passes, **When** any diffusion worker polls the kill-switch state, **Then** it halts before the next denoising step and reports the halt through telemetry.
6. **Given** uniform mean-averaging is attempted as a score-composition rule (e.g., for comparison), **When** the safety classifier evaluates the output quality, **Then** an observability event is emitted flagging that mean-averaging is degraded relative to PCG composition — the mean-averaging path MUST NOT be the default.

---

### User Story 7 - All remaining placeholders eliminated (Priority: P2)

Every placeholder comment, stub implementation, simplified mock, and permissive-bypass code path in `src/` is either replaced with the real implementation or explicitly removed (along with any unreachable callers). `grep -rn 'placeholder\|stub\|TODO\|todo!\|unimplemented!' src/` returns zero matches. The remaining specific items to eliminate:

- Agent lifecycle (#30) — heartbeat/pause/withdraw actually publish gossip messages (or the daemon event loop takes over cleanly and the lifecycle functions are removed as duplicates).
- Admin `ban()` (#34) — updates the trust registry and broadcasts a governance action.
- Receipt verification (`src/verification/receipt.rs`) — coordinator public key is wired; signature is cryptographically verified; invalid receipts are rejected.
- Daemon `current_load()` — reports real CPU+GPU+memory load derived from OS / NVML / device metrics.
- Confidential compute key sealing (`src/data_plane/confidential.rs:163`) — real TPM2 / HSM-backed seal/unseal (or remove if redundant with attested-key-release path).
- Apple VF `apple_vf.rs:176,239` — write real boot-prepared disk artifacts, or explicitly refuse on non-macOS with a clean `Error::UnsupportedPlatform`.
- Governance RPC handlers (`governance_service.rs`) — SubmitProposal and CastVote persist to the real governance store and emit audit events.
- Policy engine placeholder signatures (`rules.rs:453`, `engine.rs:236`) — the build-then-sign two-step is refactored into a single signed-builder, eliminating the intermediate `vec![0u8; 64]`.
- Churn simulator (#51) — a real kill-rejoin harness that spawns N libp2p processes, kills them on schedule, and measures actual completion.
- Apple VF Swift helper (#52) — the binary is built on macOS CI (or packaged from a prebuilt artifact) and included in the release.
- Reproducible builds (#53) — two-independent-build CI matrix produces bit-identical artifacts; a production-signed binary is released and its signature is verified by the release test.
- Tauri GUI (#40) — built and run on macOS/Linux/Windows; primary flows (enroll, submit, monitor) are smoke-tested.
- Dockerfile + Helm chart (#41) — `docker build` passes in CI; Helm chart deploys to Kind and passes a smoke test.
- REST gateway (#43) — a real HTTP listener is bound; `curl` against each route returns the expected response.

**Why this priority**: Without this cleanup, the project cannot honestly claim production readiness. Every placeholder is either a future runtime failure or a silent trust bypass. Scored P2 because each individual item is small; the collective effect is binary — either zero placeholders remain or we still can't ship.

**Independent Test**: A CI check that runs `scripts/verify-no-placeholders.sh` (a script this spec will author) and fails the build if any of: (a) grep finds `placeholder|stub|TODO|todo!|unimplemented!` in `src/**/*.rs` with exemptions only for doc-comments that explicitly describe historic context, (b) any `[0u8; 32]` / `[0u8; 64]` literal appears outside of `#[cfg(test)]` blocks, (c) any function body is `Ok(())` with no side effects (detected by a small static audit tool).

**Acceptance Scenarios**:

1. **Given** the `verify-no-placeholders.sh` check runs against the production tree, **When** any remaining placeholder is found, **Then** the check fails and names the file + line.
2. **Given** all placeholder sites are fixed, **When** the existing 802 tests are run, **Then** all still pass (no regressions), and the count grows to reflect the new tests added for each fix.

---

### User Story 8 - Operations: deployment, documentation, release pipeline (Priority: P3)

A new operator can go from `git clone` to a running donor agent on their machine in under 15 minutes, following only the README. A release engineer can cut a tagged release via `scripts/release.sh vX.Y.Z` which builds reproducible signed binaries for Linux/macOS/Windows, publishes a Docker image, publishes a Helm chart, and posts evidence artifacts to the release page.

**Why this priority**: Scored P3 because it is enabling infrastructure, not a blocker. But without it, the project cannot be adopted. Includes #41 (deployment), part of #50 (quickstart documentation), and #53 (signed releases).

**Independent Test**: A fresh Ubuntu 24.04 VM (no Rust toolchain, no libp2p, nothing) follows the README quickstart step-by-step; assert that within 15 minutes a donor daemon is running, has dialed at least one bootstrap relay, and shows a green status in `worldcompute admin status`. Separately, `scripts/release.sh` produces three reproducibly-signed binaries and a Docker image that passes the container smoke test.

**Acceptance Scenarios**:

1. **Given** a new operator on a fresh Ubuntu/macOS/Windows machine, **When** they follow the README quickstart, **Then** a donor daemon is running and joined to the mesh within 15 minutes without any step that requires operator judgment beyond "paste this command".
2. **Given** a release tag, **When** `scripts/release.sh` runs, **Then** it produces bit-identical binaries from two independent build machines and publishes them with detached signatures that the release-verification script accepts.

---

### Edge Cases

- What happens when the donor machine's firewall permits only DNS-over-HTTPS (DoH) and HTTPS on 443? The agent's WebSocket-over-TLS transport fallback handles it; if DoH is the only DNS available, the agent uses a bundled DoH resolver for `/dnsaddr/` resolution so bootstrap works without the OS resolver.
- What happens when a donor temporarily loses its relay reservation (the relay reboots)? The agent detects reservation loss via DCUtR/relay events and reacquires a reservation from an alternate public bootstrap relay within 60 seconds, retaining its PeerId so pending dials from other peers resolve when the new address is gossip-propagated.
- What happens when an attestation chain validates but the attesting node is on the governance ban list? The policy engine rejects dispatch and emits a `BannedNode` incident, regardless of attestation.
- What happens when Firecracker rootfs assembly fails halfway through (OCI tar corrupt)? The agent discards the partial image, logs the failure with the offending CID, and removes any loopback device it had mounted — no stale state is left on disk.
- What happens when the Dream-class masked-diffusion backbone weights (or the specialized-expert weights) are not available on an enrolling GPU node? The node advertises `gpu_available: true, diffusion_capable: false`, is eligible for WASM jobs, and is skipped by mesh-LLM router selection.
- What happens when a score-composition step disagrees catastrophically between experts (one expert's score dominates)? The PCG corrector step bounds how far any single expert can pull the denoised sample; out-of-distribution contributions are clipped before the Langevin update. An observability event is emitted when clipping activates on > 10 % of denoising steps for any request.
- What happens when ParaDiGMS Picard iteration fails to converge within the fixed-point budget? The scheduler falls back to strict-sequential denoising for that request, logs the fallback reason, and counts against a per-request retry budget before returning an error to the submitter.
- What happens when the Rekor service is temporarily unreachable? The ledger continues local writes, queues anchor requests, and retries with exponential backoff; transparency anchoring is eventually consistent, not synchronous to each ledger write.
- What happens when a ChurnSimulator run's node is killed mid-TaskDispatch? The coordinator's lease expires, matchmaking re-selects, the workload runs on a surviving replica, and the original receipt is marked superseded.
- What happens when an institutional firewall allows outbound TCP/443 but does SSL inspection (MITM)? The agent detects the unexpected certificate (pin-mismatch with known relay fingerprints) and logs a security warning; the operator can opt in to a `--allow-ssl-inspection` flag that trusts the local root CA but marks the connection tier as `Inspected`.
- What happens when a donor's real CPU load spikes above the sovereignty threshold? The preemption supervisor fires within 1 second, pauses in-flight WASM/Firecracker workloads to checkpoints, and the daemon reports paused state through gossip — already implemented in #45, must not regress.

## Requirements *(mandatory)*

### Functional Requirements

**Cross-firewall mesh formation (from #60)**

- **FR-001**: The donor daemon MUST, when started with default configuration, dial at least one public bootstrap relay and maintain the connection for ≥ 10 continuous minutes on any of the following network profiles: (a) residential NAT, (b) university campus firewall, (c) corporate firewall, (d) cloud security group.
- **FR-002**: The donor daemon MUST obtain a libp2p Relay v2 reservation, log the reservation address, and gossip its new public multiaddr so remote peers can dial it.
- **FR-003**: When outbound TCP and QUIC are both blocked, the donor daemon MUST fall back to WebSocket-over-TLS on port 443 and still form the mesh; fallback MUST be automatic (no user action) and logged with the reason.
- **FR-004**: The donor daemon MUST surface every `libp2p_swarm::DialFailure` at `info` level or higher (never silently at `trace`) with the dial target, transport, and root cause.
- **FR-005**: The donor daemon MUST, when the OS resolver cannot resolve `/dnsaddr/...` multiaddrs (e.g., captive portals, strict DNS filtering), use a bundled DoH resolver as fallback so bootstrap proceeds.
- **FR-006**: The donor daemon MUST support reservation replacement: on reservation loss, reacquire from an alternate relay within 60 seconds, retaining PeerId.
- **FR-007**: A submitter MUST be able to dispatch a WASM job to a peer whose only reachable address is `/p2p/<relay>/p2p-circuit/p2p/<self>` and receive a cryptographically signed receipt.
- **FR-007a**: The project MUST operate at least one (ideally two) fallback relays with public WSS/443 listeners at launch so that SC-001 passes on day one even when no volunteer-run WSS/443 relay is yet online. These relays MUST be listed in `src/network/discovery.rs::PUBLIC_LIBP2P_BOOTSTRAP_RELAYS` alongside the Protocol Labs defaults. `docs/operators/running-a-relay.md` MUST document the one-command procedure for a volunteer to bring up a WSS/443 relay that auto-announces into the mesh; the project-operated relays MUST be retire-able to volunteer replacement without a client update by relying on gossip + peer-exchange discovery.

**Deep attestation (from #28, #29, #56)**

- **FR-008**: The validator MUST pin real AMD ARK and Intel DCAP root CA SHA-256 fingerprints at compile time; the `production` cargo feature MUST fail to build when either is `[0u8; 32]`.
- **FR-009**: The validator MUST reject any attestation chain whose root does not match the pinned fingerprint; no permissive bypass in production builds.
- **FR-010**: The transparency verifier MUST pin the real Rekor Ed25519 public key; verification of a signed tree head that fails signature check MUST reject the entry.
- **FR-011**: The ledger's cross-shard anchor to Rekor MUST go through the pinned-key verification path; ledger writes whose anchor cannot be verified within the retry budget MUST be flagged and eventually require operator intervention to clear.
- **FR-011a**: The pinned AMD ARK fingerprint, Intel DCAP root CA fingerprint, and Sigstore Rekor public key MUST be frozen in source at each tagged release (no fetch at daemon startup). A CI drift-check job MUST run on a schedule (at least weekly) that refetches each value from its authoritative upstream, compares against the in-tree pin, and opens a repository issue within 24 hours of any mismatch. The release-engineering procedure documented in `docs/releases.md` MUST require the drift-check issue queue to be empty before cutting a new tag.

**Firecracker rootfs (from #33)**

- **FR-012**: On Linux hosts with KVM and Firecracker installed, the donor MUST assemble a real ext4 rootfs from CID-referenced OCI layers using `mkfs.ext4` + loopback mount + tar extraction.
- **FR-013**: The assembled rootfs MUST be bootable by Firecracker and the entrypoint MUST execute with stdout/stderr captured via vsock.
- **FR-014**: Rootfs assembly failures (invalid tar, CID mismatch, insufficient disk) MUST return a specific error and MUST NOT leave orphaned loopback devices or partial files.

**End-to-end Phase 1 cluster (from #42, #51)**

- **FR-015**: The system MUST provide a reproducible `scripts/e2e-phase1.sh` that stands up a three-node cluster on real hardware, submits a mix of workloads, records results, and emits an evidence bundle.
- **FR-016**: A real 72-hour churn run at 30% rotation MUST achieve ≥ 80% job completion and MUST produce a ledger that replays identically from genesis.
- **FR-017**: The churn simulator MUST be refactored from a statistical model into a real kill-rejoin harness that spawns real libp2p processes and actually kills / restarts them on schedule.

**Platform adapters (from #37, #38, #39, #52)**

- **FR-018**: The Slurm adapter MUST submit a real `sbatch` against slurmrestd, poll for completion, and return the job result; CI MUST run this against a containerized Slurm control plane.
- **FR-019**: The Kubernetes adapter MUST install the ClusterDonation CRD, deploy the operator to a Kind cluster in CI, and reconcile one ClusterDonation resource end-to-end.
- **FR-020**: The cloud adapter MUST fetch and validate IMDSv2 (AWS), GCE metadata, and Azure IMDS identity documents; a real enrollment against each of the three clouds MUST be captured as an evidence artifact per tagged release.
- **FR-020a**: The real cloud-adapter enrollment test MUST run on the cheapest/freest available tier per provider (AWS Free Tier `t3.micro`, GCP free tier / initial credit, Azure free tier / initial credit, or equivalent student/organization credits). The test MUST be implemented as a GitHub Actions `workflow_dispatch` workflow gated such that only repository `maintain`/`admin`/`owner` permission levels can invoke it, AND MUST additionally run automatically on each tagged release. Evidence (log, IMDS identity document, signed receipt) MUST be committed under `evidence/phaseN/cloud-adapter/<provider>/` as part of the release artifacts. A failed real-cloud run MUST block the release tag from being marked `stable`.
- **FR-021**: The Apple VF Swift helper binary MUST be built on macOS CI, signed, and included in the release package so macOS donors can use VZVirtualMachine isolation without a separate install step.

**Distributed-diffusion mesh LLM (from #27, #54; whitepaper `notes/parallel_mesh_of_diffusers_whitepaper.pdf`)**

- **FR-022**: The mesh LLM MUST use a **masked-discrete-diffusion** architecture — specifically a Dream-class 7B-parameter open-weights masked-diffusion language model (Dream 7B / LLaDA 8B / DiffuLLaMA or equivalent) as the shared backbone. Autoregressive transformer ensembling (e.g., LLaMA top-K logit averaging) is explicitly NOT the target and MUST NOT be shipped as the production path.
- **FR-023**: The current `src/agent/mesh_llm/` implementation (router selecting K-of-N experts per token, top-K sparse logit aggregation, token-level sampling) MUST be replaced with diffusion-native primitives: per-timestep score fields, PCG (Predictor-Corrector Guidance) score composition with per-expert specialization weights, and denoising-step scheduling (not token-step scheduling). Any remaining autoregressive-ensembling code paths MUST be deleted or clearly marked and gated behind a non-default `--ar-ensemble-legacy` experimental flag for benchmark comparison only.
- **FR-024**: An expert node implementation MUST support loading small SSD-2-style specialized diffusion experts that contribute conditional score signals at each denoising step; the composition MUST implement the PCG framework (Bradley and Nakkiran, TMLR 2025) rather than uniform mean-averaging (Razafindralambo et al., TMLR 2026, proved fails on FID).
- **FR-025**: The scheduler MUST support ParaDiGMS-style parallel denoising: denoising timesteps solved in parallel via Picard iteration to achieve ≥ 2× wall-clock speedup over strict-sequential denoising on ≥ 3 GPUs with the same backbone and experts.
- **FR-026**: The inter-node transport layer for diffusion messages MUST implement DistriFusion-style stale-activation pipelining: activation tensors from timestep `t` are usable at timestep `t+1` via asynchronous communication, hiding round-trip time behind compute. The system MUST measurably mask ≥ 50 % of WAN RTT behind compute in a controlled benchmark.
- **FR-027**: The mesh-LLM service MUST handle real diffusion inference RPCs end-to-end (no "stub — no real inference yet" self-describing comment may remain in the production path).
- **FR-028**: A smoke test MUST run a real multi-node distributed diffusion (Dream-class backbone + ≥ 2 SSD-2-style experts across ≥ 6 real GPU workers spanning tensor01 and tensor02, respecting the "max 3 GPUs/job per cluster" hardware budget) on at least one of the domains where diffusion LMs outperform AR per the cited literature — constraint satisfaction (Countdown / Sudoku), planning, or code infilling — and return a coherent result.
- **FR-028a**: The DistriFusion-pipelining benchmark (FR-026) and the ParaDiGMS-speedup benchmark (FR-025) MUST use `tc qdisc netem` on the tensor01↔tensor02 link to emulate 100 ms RTT for the controlled WAN measurement, with the measured wall-clock speedups and RTT-masking percentages recorded in an evidence artifact under `evidence/phase1/diffusion-mesh/`.
- **FR-029**: The safety tier classifier and governance kill switch MUST integrate at the denoising-step granularity: if a kill switch fires mid-inference, the worker halts before the next denoising step, not next token.

**All remaining placeholders (from #30, #34, and inline code verification)**

- **FR-030**: `src/agent/lifecycle.rs` heartbeat / pause / withdraw MUST either broadcast over gossipsub directly or be removed and their callers migrated to the daemon event loop — no duplicate stub path.
- **FR-031**: `src/governance/admin_service.rs::ban()` MUST update the trust registry and broadcast a governance action.
- **FR-032**: `src/verification/receipt.rs` MUST wire the coordinator public key and cryptographically verify receipt signatures; malformed or unsigned receipts MUST be rejected.
- **FR-033**: `src/agent/daemon.rs::current_load()` MUST report a real OS-derived load value (CPU, GPU, memory) rather than a fixed constant.
- **FR-034**: `src/data_plane/confidential.rs` key sealing MUST be wired to TPM2 / HSM-backed seal/unseal (or the function removed if the attested-key-release path makes it redundant).
- **FR-035**: `src/sandbox/apple_vf.rs` MUST either produce a real boot-prepared disk on macOS or return `Error::UnsupportedPlatform` on non-macOS; the current `b"placeholder-disk"` path MUST be removed.
- **FR-036**: `src/governance/governance_service.rs` SubmitProposal and CastVote handlers MUST persist to the governance store and emit audit events.
- **FR-037**: `src/policy/rules.rs` and `src/policy/engine.rs` MUST replace the two-step build-then-resign pattern (which leaves `vec![0u8; 64]` in intermediate state) with a single-pass signed-builder.
- **FR-038**: A CI check script `scripts/verify-no-placeholders.sh` MUST hard-fail the build when grep finds `placeholder|stub|TODO|todo!|unimplemented!` in production `src/` paths, except at lines listed in `.placeholder-allowlist` (format: `path:line — rationale`). Any PR that adds an allowlist entry MUST have that entry reviewed and justified in the PR description. **For spec 005 to be declared complete, `.placeholder-allowlist` MUST be empty.** The allowlist mechanism exists only for long-term post-005 maintenance.

**Operations (from #40, #41, #43, #50)**

- **FR-039**: The Tauri GUI MUST build and run on macOS/Linux/Windows; the three primary flows (enroll, submit, monitor) MUST be smoke-tested in CI via Playwright or Tauri's test harness.
- **FR-040**: The Dockerfile MUST build successfully in CI; `docker run worldcompute:latest --help` MUST succeed; the Helm chart MUST deploy to a Kind cluster in CI and pass a smoke test.
- **FR-041**: The REST gateway MUST bind to a real HTTP listener in the daemon when configured; each documented route MUST be exercised by a CI integration test.
- **FR-042**: The quickstart documentation MUST walk a new operator from `git clone` to a running donor in under 15 minutes on fresh Ubuntu 24.04 / macOS 14 / Windows 11 — verified by a timed test.

**Reproducible builds (from #53)**

- **FR-043**: The CI matrix MUST include a "reproducible-build" job that builds the production binary on two independent runners and asserts the artifacts are bit-identical.
- **FR-044**: The release pipeline MUST produce detached Ed25519 signatures for every shipped binary; a `scripts/verify-release.sh` script MUST verify those signatures using the pinned release public key.

### Key Entities

- **Relay reservation**: A libp2p Relay v2 reservation held by a donor behind NAT/firewall so remote peers can reach it via a circuit; critical for cross-firewall participation.
- **WebSocket-over-TLS transport**: A libp2p transport that tunnels over TLS on port 443, usable when all other transports are blocked by firewall.
- **Pinned root CA fingerprint**: A compile-time-constant SHA-256 digest of a manufacturer root CA (AMD ARK, Intel DCAP) used to anchor attestation chains; today zero, must be real.
- **Pinned Rekor public key**: A compile-time-constant Ed25519 public key used to verify Sigstore Rekor signed tree heads; today zero, must be real.
- **OCI image assembly**: The process of fetching layers from the CID store, extracting them onto an ext4 filesystem, and producing a file that Firecracker can boot.
- **Churn harness**: A process that spawns N real libp2p daemons, kills/restarts them on schedule, and measures real completion rates (vs. a statistical model).
- **Evidence artifact**: A committed bundle under `evidence/phaseN/<area>/` containing logs, ledger dumps, and trace files proving a real-hardware test passed.
- **Placeholder site**: A location in `src/` where the current code returns a hard-coded value, writes `[0u8; N]`, or calls a function whose doc-comment says "stub" / "placeholder"; enumerated in the Background section.
- **Masked-diffusion language model**: A non-autoregressive LM that operates on fully-masked sequences and iteratively denoises them; the production architecture for the mesh LLM per the whitepaper.
- **Dream-class backbone**: Any of Dream 7B / LLaDA 8B / DiffuLLaMA (or a later comparable open-weights masked-diffusion model initialized from an open AR checkpoint) used as the shared large-model backbone in the diffusion swarm.
- **SSD-2-style specialized expert**: A small diffusion expert (≪ backbone size) that contributes a conditional score signal at each denoising step; composed with other experts and the backbone via PCG, per Han/Kumar/Tsvetkov/Ghazvininejad (NAACL 2024).
- **PCG (Predictor-Corrector Guidance)**: The mathematically grounded score-composition rule that combines a DDIM predictor with a Langevin-dynamics corrector on a gamma-powered distribution; per Bradley and Nakkiran (TMLR 2025), the correct framework for combining multiple diffusion score signals.
- **ParaDiGMS parallel denoising**: A Picard-iteration fixed-point solver that evaluates multiple denoising timesteps in parallel; per Shih et al. (NeurIPS 2023), delivers 2–4× wall-clock speedup with no quality loss.
- **DistriFusion stale-activation pipelining**: An asynchronous-communication pattern for diffusion models where activations from timestep `t` are used at timestep `t+1` in place of "fresh" activations, hiding network RTT behind per-step compute; per Li et al. (CVPR 2024).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A donor daemon started behind Dartmouth's institutional firewall (`tensor02.dartmouth.edu`) forms a mesh connection to a public bootstrap relay and holds it continuously for ≥ 10 minutes, on the first try, with no manual firewall changes. (Proof: log trace committed.)
- **SC-002**: A WASM job dispatched between two donors on two independent firewalled networks completes with `Succeeded` status and a cryptographically verifiable receipt, with end-to-end latency under 5 seconds for a trivial workload.
- **SC-003**: Every attestation chain signed by a real AMD EPYC or Intel TDX processor verifies successfully against pinned manufacturer root CAs; every tampered chain is rejected. Zero production paths enter permissive bypass.
- **SC-004**: A real OCI image ships via the CID store and boots inside Firecracker, producing correct stdout within 10 seconds on a typical KVM host.
- **SC-005**: A 72-hour real-hardware churn run at 30% rotation achieves ≥ 80% job completion and emits a replay-identical ledger.
- **SC-006**: `grep -rn 'placeholder\|stub\|TODO\|todo!\|unimplemented!' src/` returns zero production matches AND `.placeholder-allowlist` is empty at the moment spec 005 is declared complete. Any non-zero count or any allowlist entry at that moment means spec 005 does not pass.
- **SC-007**: The existing 802 tests still pass; test count grows to at least 900 as new real-hardware tests are added.
- **SC-008**: A new operator on a fresh machine reaches a running donor agent joined to the mesh in under 15 minutes using only the README.
- **SC-009**: A release binary produced via the CI reproducible-build pipeline verifies bit-identical across two independent runners and carries a verifying Ed25519 signature.
- **SC-010**: The distributed-diffusion mesh-LLM smoke test returns a coherent response to a constraint-satisfaction / planning / code-infilling prompt (the domains where diffusion LMs outperform AR per the cited literature) from a ≥ 3-node GPU cluster using a real Dream-class 7B masked-diffusion backbone plus ≥ 2 SSD-2-style specialized experts with PCG score composition, ParaDiGMS parallel denoising at ≥ 2× speedup, and DistriFusion stale-activation pipelining masking ≥ 50 % of WAN RTT behind compute.

## Assumptions

- The user retains tensor01, tensor02, and at least one off-campus machine as the primary real-hardware test bed; credentials are already stored privately.
- Sigstore's public Rekor instance (`https://rekor.sigstore.dev`) is the transparency log; its public key is stable and can be pinned at build time.
- AMD and Intel publish stable SHA-256 fingerprints for their root CAs that can be pinned at build time; if the manufacturers rotate, a CI check will detect the mismatch and prompt a release update.
- The project-hosted `/dnsaddr/bootstrap.worldcompute.org/...` seeds will eventually resolve to real operator-run bootstrap relays; until then, public Protocol Labs libp2p relays are the default rendezvous (already configured in `src/network/discovery.rs::PUBLIC_LIBP2P_BOOTSTRAP_RELAYS`).
- Phase 1 testing uses up to three real machines with the user's existing hardware; cloud adapter end-to-end verification (FR-020) uses operator-provided AWS/GCP/Azure accounts and is captured as a one-off evidence artifact rather than gated in every CI run.
- The mesh-LLM production architecture is **distributed masked-discrete-diffusion**, NOT autoregressive transformer ensembling, per `notes/parallel_mesh_of_diffusers_whitepaper.pdf`. The initial target backbone is a Dream-class 7B open-weights masked-diffusion LM (Dream 7B, LLaDA 8B, or DiffuLLaMA at time of implementation). If a better open-weights masked-diffusion model exists by the time this is implemented, the operator may substitute it; the architectural primitives (PCG composition, ParaDiGMS denoising, DistriFusion pipelining) are model-agnostic.
- The existing `src/agent/mesh_llm/*.rs` AR-ensembling code is architecturally incorrect and MUST be replaced in this spec; leaving it in place as a "default" and adding a diffusion path alongside is explicitly rejected.
- Docker-in-Docker and Kind-in-CI are acceptable for operator smoke tests in GitHub Actions; if a CI runner blocks nested virtualization, the adapter test falls back to a self-hosted runner.
- The build environment already has `cargo`, `libp2p 0.54+`, `wasmtime 27+`, `candle_transformers` available (per Cargo.toml); no new major dependency is required beyond what spec 004 already set up, with the possible exception of a DoH client library for the firewall-fallback path.
- "Eliminate all placeholders" is scoped to production code paths in `src/`; test-helper placeholders (`#[cfg(test)]`-gated fixtures) are permitted because they are not shipped.
