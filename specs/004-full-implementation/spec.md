# Feature Specification: Full Functional Implementation

**Feature Branch**: `004-full-implementation`
**Created**: 2026-04-17
**Status**: Draft
**Input**: Master issue #57 — complete all 28 sub-issues (#28–#56) for a fully functional World Compute system. No stubs, no remaining tasks. All tested on real hardware.
**Test Infrastructure**: SSH access to `tensor01.dartmouth.edu` (credentials in `.credentials`)

---

## User Scenarios & Testing

### User Story 1 — Cryptographically Verified Attestation (Priority: P1)

A coordinator receives an attestation quote from a donor node and verifies the full certificate chain — not just structure, but real RSA/ECDSA cryptographic signatures — against pinned root CA certificates (AMD ARK for SEV-SNP, Intel DCAP for TDX, manufacturer roots for TPM2). Invalid chains are rejected. Rekor transparency entries include full Merkle inclusion proof verification against the signed tree head.

**Why this priority**: Without real cryptographic verification, the system accepts any well-formed attestation. This is a Safety First (Principle I) violation — the single most critical gap.

**Independent Test**: Present known-good AMD SEV-SNP certificate chain → accepted. Present chain with wrong root fingerprint → rejected. Submit Rekor entry → retrieve inclusion proof → verify against signed tree head. Run on `tensor01.dartmouth.edu` with real TPM hardware.

**Acceptance Scenarios**:

1. **Given** a valid TPM2 EK certificate chain, **When** the validator processes it, **Then** the chain is accepted and trust tier is assigned based on measurement match
2. **Given** a certificate chain with an expired intermediate, **When** the validator processes it, **Then** the chain is rejected with a clear error
3. **Given** a Rekor log entry, **When** the inclusion proof is retrieved, **Then** the proof validates against the published signed tree head
4. **Given** a tampered inclusion proof, **When** verification is attempted, **Then** the proof is rejected

---

### User Story 2 — Agent Lifecycle and Preemption (Priority: P1)

A donor installs the agent, enrolls, receives heartbeat-based lease offers from the broker, executes work, pauses (checkpointing active sandboxes), resumes, and withdraws cleanly leaving zero host residue. The preemption supervisor delivers SIGSTOP within 10ms of a keyboard event, attempts checkpoint within 500ms, and escalates to SIGKILL if needed.

**Why this priority**: Principle III (Donor Sovereignty) requires sub-second preemption and clean lifecycle. Without this, donors cannot safely participate.

**Independent Test**: Run agent on `tensor01.dartmouth.edu`. Enroll → receive work → inject keyboard event → measure SIGSTOP latency (<10ms) → checkpoint → resume → withdraw → scan for residual files/processes (must find zero).

**Acceptance Scenarios**:

1. **Given** an enrolled agent, **When** a heartbeat is sent, **Then** the broker responds with available lease offers
2. **Given** a running workload, **When** a keyboard event fires, **Then** SIGSTOP reaches all sandbox processes within 10ms (measured)
3. **Given** a paused agent, **When** resume is issued, **Then** the agent resumes from checkpoint and continues processing
4. **Given** a withdrawing agent, **When** withdrawal completes, **Then** zero files, processes, scheduled tasks, or network state remain on host

---

### User Story 3 — Policy Engine Completion (Priority: P1)

A job submission passes through all 10 steps of the deterministic policy engine: identity verification, signature check, artifact registry CID lookup (with real resolution against the ApprovedArtifact registry), workload class check, quota check, egress allowlist validation (declared endpoints checked against approved list), data classification, ban check, and optional LLM advisory. Each step produces an immutable audit record.

**Why this priority**: The policy engine is the gatekeeper for all workload execution. Incomplete steps mean unsafe jobs can be dispatched.

**Independent Test**: Submit job with approved CID and approved endpoints → accepted. Submit job with unknown CID → rejected. Submit job with undeclared network endpoint → rejected.

**Acceptance Scenarios**:

1. **Given** a job with a valid artifact CID in the registry, **When** submitted, **Then** it passes the artifact check
2. **Given** a job with an unknown CID, **When** submitted, **Then** it is rejected with error code WC-006
3. **Given** a job declaring endpoints not on the approved list, **When** submitted, **Then** it is rejected at the egress allowlist step
4. **Given** artifact signer and approver are the same identity, **When** submitted, **Then** it is rejected (separation of duties)

---

### User Story 4 — Sandbox Depth: GPU, Firecracker Rootfs, Incident Containment (Priority: P1)

GPU passthrough is verified via real IOMMU group inspection. Firecracker VMs boot from OCI images fetched from the CID store and assembled into rootfs.ext4. Incident containment primitives (FreezeHost, QuarantineWorkloadClass, BlockSubmitter, RevokeArtifact, DrainHostPool) execute real enforcement effects — not just audit records.

**Why this priority**: Without GPU verification, unsafe passthrough is possible. Without rootfs preparation, Firecracker cannot run real workloads. Without containment enforcement, incidents cannot be responded to.

**Independent Test**: On `tensor01.dartmouth.edu`: enumerate GPUs → verify IOMMU groups → store OCI image in CID store → assemble rootfs.ext4 → boot Firecracker VM → execute workload → verify output. Trigger FreezeHost → verify all sandbox processes stopped within 60s.

**Acceptance Scenarios**:

1. **Given** a GPU in a singleton IOMMU group, **When** passthrough is requested, **Then** it is allowed
2. **Given** a GPU in a shared IOMMU group, **When** passthrough is requested, **Then** it is rejected
3. **Given** an OCI image stored in the CID store, **When** rootfs preparation runs, **Then** a bootable ext4 image is produced
4. **Given** a FreezeHost containment action, **When** executed, **Then** all sandbox processes on the target host are stopped within 60 seconds
5. **Given** a QuarantineWorkloadClass action, **When** a job of that class is submitted, **Then** it is rejected by the policy engine

---

### User Story 5 — Security: Adversarial Tests, Confidential Compute, mTLS, Supply Chain (Priority: P1)

All 8 adversarial test scenarios are fully implemented (not `#[ignore]`/`unimplemented!()`). Confidential compute provides client-side AES-256-GCM encryption with TPM-attested key release. mTLS certificates are issued per-account with 90-day auto-rotation. Release binaries are reproducibly built and code-signed.

**Why this priority**: Principle I (Safety First) requires these before any external deployment.

**Independent Test**: Run sandbox escape test (ptrace from inside Firecracker → must fail). Encrypt job → execute on attested node → decrypt result → verify correct. Generate mTLS cert → authenticate → verify accepted → exceed rate limit → verify 429.

**Acceptance Scenarios**:

1. **Given** a sandbox escape attempt via ptrace, **When** executed inside Firecracker VM, **Then** the attempt fails and is logged
2. **Given** a Byzantine donor injecting wrong results, **When** 100 tasks are audited, **Then** the donor is detected and quarantined
3. **Given** a job with confidential-medium classification, **When** key release is requested without valid TPM attestation, **Then** key release is denied
4. **Given** a new account, **When** an mTLS certificate is issued, **Then** it authenticates successfully and auto-rotates before 90-day expiry
5. **Given** two independent builds from the same git commit, **When** compared, **Then** they produce identical binary output

---

### User Story 6 — Integration Test Coverage and Churn Validation (Priority: P1)

All 12 previously untested src/ modules have integration tests. The churn simulator validates 80% job completion at 30% node churn over a 72-hour run. The Phase 1 LAN testnet runs on 3+ physical machines with mDNS discovery, R=3 job execution, preemption, and failure recovery.

**Why this priority**: Principle V (Direct Testing) is non-negotiable. No component ships without real-hardware evidence.

**Independent Test**: Run full test suite → verify every src/ module has integration coverage. Deploy 3+ nodes on `tensor01.dartmouth.edu` cluster → form cluster via mDNS → run R=3 job → kill one node → verify job completes from checkpoint.

**Acceptance Scenarios**:

1. **Given** the full test suite, **When** run, **Then** every src/ module has at least one integration test (zero untested modules)
2. **Given** a 20+ node testbed with 30% churn, **When** jobs are submitted over 72 hours, **Then** at least 80% complete correctly
3. **Given** 3 physical machines on a LAN, **When** agents start, **Then** they form a cluster via mDNS in under 5 seconds
4. **Given** a running R=3 job, **When** one node is killed, **Then** the job reschedules from checkpoint and completes correctly

---

### User Story 7 — Runtime Systems: Credits, Storage, Scheduler, Ledger (Priority: P2)

Credits decay at 45-day half-life with floor protection. Storage enforces per-donor caps with garbage collection. The scheduler performs real ClassAd-style matchmaking with lease management. The ledger uses t-of-n threshold signing with CRDT merge.

**Why this priority**: Required for sustained multi-node operation, but not blocking initial testing.

**Independent Test**: Simulate 90 days of credit earn/spend → verify decay curve. Fill donor storage to cap → verify GC triggers. Submit job → verify broker matches to capable node → kill executor → verify rescheduling. Start 5 coordinators → sign ledger entry → verify 3-of-5 threshold.

**Acceptance Scenarios**:

1. **Given** credits earning over 90 days, **When** decay is applied, **Then** the balance follows the 45-day half-life curve within 1% tolerance
2. **Given** a donor at storage cap, **When** new data arrives, **Then** expired data is garbage collected to make room
3. **Given** a job requiring GPU capabilities, **When** submitted, **Then** the broker matches it only to GPU-capable nodes
4. **Given** 5 coordinator nodes, **When** a ledger entry is signed, **Then** 3-of-5 threshold signature is valid

---

### User Story 8 — Platform Adapters: Slurm, Kubernetes, Cloud, Apple VF (Priority: P2)

The Slurm adapter connects to a real Slurm head node and dispatches jobs via sbatch. The Kubernetes adapter watches a ClusterDonation CRD and creates Pods. The cloud adapter verifies instance identity via metadata services. The Apple VF helper binary uses real Virtualization.framework APIs.

**Why this priority**: Adapters extend reach to HPC, cloud, and macOS but are not required for core functionality.

**Independent Test**: Deploy Slurm adapter on `tensor01.dartmouth.edu` (if Slurm available) → submit SHA-256 test job → verify correct result. Deploy K8s operator on minikube → apply CRD → verify Pod created. Build Apple VF helper on macOS → boot VM → execute workload.

**Acceptance Scenarios**:

1. **Given** a Slurm cluster, **When** the adapter is installed, **Then** jobs are dispatched via sbatch and results collected
2. **Given** a Kubernetes cluster with the operator deployed, **When** a ClusterDonation CRD is applied, **Then** capacity is registered and tasks create Pods
3. **Given** an AWS EC2 instance, **When** the cloud adapter starts, **Then** instance identity is verified via IMDSv2
4. **Given** macOS 13+ hardware, **When** the Apple VF helper starts, **Then** a Linux guest VM boots and executes a workload

---

### User Story 9 — User-Facing: GUI, REST Gateway, Web Dashboard (Priority: P2)

The Tauri desktop app launches with functional donor, submitter, governance, and settings pages backed by real agent IPC. The REST/HTTP+JSON gateway exposes all 6 gRPC services. The web dashboard provides donor and submitter feature parity with the CLI.

**Why this priority**: Required for public-facing operation but not for core system validation.

**Independent Test**: Launch Tauri app → submit job through GUI → verify completion. Call REST endpoint → verify matches CLI output. Load web dashboard → cast governance vote → verify recorded on ledger.

**Acceptance Scenarios**:

1. **Given** the Tauri app, **When** launched, **Then** it displays a functional window with donor dashboard
2. **Given** a REST API call to submit a job, **When** the job completes, **Then** the result matches CLI output
3. **Given** the web dashboard, **When** a governance vote is cast, **Then** it is recorded on the tamper-evident ledger

---

### User Story 10 — Operations: Deployment, Energy, Documentation (Priority: P2)

Docker containers, Helm charts, and release pipelines are functional. Energy metering reports per-node CPU/GPU-time and estimated watts. Documentation includes working quickstart, evidence artifact schema, and incident disclosure policy.

**Why this priority**: Required for real deployment but not for development testing.

**Independent Test**: `docker build` → verify minimal image. `docker compose up` → verify 3-node cluster. Follow README quickstart on clean machine → verify it works. Compare RAPL readings against wall-meter on `tensor01.dartmouth.edu`.

**Acceptance Scenarios**:

1. **Given** a Dockerfile, **When** built, **Then** the image is under 100MB and runs the agent
2. **Given** docker compose with 3 nodes, **When** started, **Then** they discover each other and form a cluster
3. **Given** a clean machine, **When** following README quickstart, **Then** a working single-node agent is operational within 5 minutes
4. **Given** RAPL-capable hardware, **When** energy metering runs, **Then** estimates are within 20% of real power draw

---

### User Story 11 — Distributed Mesh LLM (Priority: P3)

GPU donor nodes each run a LLaMA-3-8B model at 4-bit quantization. A distributed router selects K-of-N experts per token. Sparse logit aggregation produces coherent text at 3.2+ tokens/second. The self-prompting loop generates actionable improvement tasks. Action tiers gate operations through governance. A kill switch immediately halts all inference.

**Why this priority**: The largest single feature, requiring 280+ GPU nodes for distributed operation. Phase 0-1 (centralized model, read-only + suggest tiers) ships first.

**Independent Test**: Deploy 4+ GPU nodes → register as experts → generate 100 tokens via sparse aggregation → verify coherent output. Trigger kill switch → verify immediate halt. Test self-prompting loop → verify actionable output.

**Acceptance Scenarios**:

1. **Given** 4 GPU nodes running LLaMA-3-8B, **When** the router selects K=4 experts, **Then** sparse logit aggregation produces coherent text
2. **Given** K=4 experts at 100ms inter-node latency, **When** generating tokens, **Then** throughput is 3.2+ tokens/second
3. **Given** the governance kill switch, **When** triggered, **Then** all inference streams halt immediately and the last 3 changes are reverted
4. **Given** the self-prompting loop, **When** run for one cycle, **Then** it produces at least one actionable improvement task
5. **Given** a deploy-major action, **When** proposed, **Then** it requires a full governance vote and 24-hour review period

---

### Edge Cases

- What happens when a donor's internet drops mid-heartbeat? (Broker detects missed heartbeat, marks node offline, reschedules leased tasks from checkpoint)
- What happens when all 3 replicas of an R=3 job produce different results? (No majority — task fails, all 3 nodes take trust score penalty, task rescheduled on different nodes)
- What happens when a coordinator partition splits the Raft group? (Minority partition cannot commit; majority continues; on rejoin, follower replays missed entries via log replication)
- What happens when the churn simulator kills a broker node? (Regional broker failover — another well-behaved agent is elected as transient broker)
- What happens when a GPU kernel exceeds the 200ms preemption window? (Kernel runs to completion; preemption latency is logged; donor's GPU certification may require re-testing)
- What happens when the mesh LLM proposes a deploy-major change but governance rejects it? (Change is discarded; mesh returns to read-only for the rejected domain; next cycle proposes alternatives)
- What happens when Rekor staging is unreachable? (Transparency anchoring is deferred; entries queue locally; next successful anchor includes all queued entries)
- What happens when coordinator quorum is lost? (Graceful degradation — local brokers continue dispatching from cached leases; ledger writes queue locally; CRDT merge reconciles on rejoin; system does not halt new dispatch)

## Requirements

### Functional Requirements

**Category 1: Core Infrastructure Depth (#28, #29, #30, #31, #32, #33, #34, #45)**

- **FR-001**: System MUST verify RSA/ECDSA signatures in TPM2, SEV-SNP, and TDX certificate chains against pinned manufacturer root CAs
- **FR-002**: System MUST verify Rekor inclusion proofs cryptographically against the signed tree head, not just format-validate
- **FR-003**: Agent MUST send periodic heartbeats to the broker and receive lease offers in response
- **FR-004**: Agent MUST checkpoint all active sandboxes on pause and leave zero host residue on withdrawal
- **FR-005**: Preemption supervisor MUST deliver SIGSTOP within 10ms of a sovereignty trigger (measured and logged)
- **FR-006**: Policy engine MUST resolve artifact CIDs against the ApprovedArtifact registry before dispatch
- **FR-007**: Policy engine MUST validate declared egress endpoints against an approved endpoint list
- **FR-008**: System MUST enumerate PCI devices and verify singleton IOMMU groups before allowing GPU passthrough
- **FR-009**: Firecracker driver MUST fetch OCI images from CID store and assemble bootable rootfs.ext4
- **FR-010**: Incident containment primitives MUST execute real enforcement effects (FreezeHost stops processes, QuarantineWorkloadClass triggers policy rejection, BlockSubmitter cancels in-flight jobs)

**Category 2: Security (#35, #46, #47, #53)**

- **FR-011**: All 8 adversarial tests MUST be fully implemented with no `#[ignore]` or `unimplemented!()` macros
- **FR-012**: Confidential-medium jobs MUST use client-side AES-256-GCM encryption with TPM-attested key release
- **FR-013**: Confidential-high jobs MUST use SEV-SNP/TDX guest-measurement sealed keys
- **FR-014**: System MUST issue per-account Ed25519 mTLS certificates with 90-day auto-rotation
- **FR-015**: System MUST enforce rate limits per class (DONOR_HEARTBEAT 120/min, JOB_SUBMIT 10/min, GOVERNANCE 5/min)
- **FR-016**: Release binaries MUST be reproducibly built and Ed25519 code-signed
- **FR-017**: Agent MUST reject dispatch from unsigned or unattested peers

**Category 3: Test Coverage (#36, #51, #42)**

- **FR-018**: Every src/ module MUST have at least one integration test exercising its public API with real inputs
- **FR-019**: Churn simulator MUST validate 80% job completion at 30% node churn
- **FR-020**: Phase 1 LAN testnet MUST run on 3+ physical machines with mDNS discovery in under 5 seconds

**Category 4: Platform Adapters (#37, #38, #39, #52)**

- **FR-021**: Slurm adapter MUST connect to a real Slurm head node and dispatch jobs via sbatch
- **FR-022**: Kubernetes adapter MUST watch ClusterDonation CRD and create Pods with enforced resource limits
- **FR-023**: Cloud adapter MUST verify instance identity via AWS IMDSv2, GCP metadata, or Azure IMDS
- **FR-024**: Apple VF helper MUST use real Virtualization.framework APIs for VM lifecycle on macOS 13+

**Category 5: Runtime Systems (#44, #49, #55, #56)**

- **FR-025**: Credits MUST decay at 45-day half-life with floor protection (`trailing_30d_earn_rate * 30`)
- **FR-026**: Storage MUST enforce per-donor caps and garbage collect expired/orphaned data
- **FR-027**: Scheduler MUST perform ClassAd-style bilateral matchmaking with lease management
- **FR-028**: Ledger MUST use t-of-n threshold signing (3-of-5 target) with CRDT OR-Map merge
- **FR-028a**: When coordinator quorum is lost, local brokers MUST continue dispatching from cached leases; ledger writes MUST queue locally and reconcile via CRDT merge when quorum is restored

**Category 6: User-Facing (#40, #43)**

- **FR-029**: Tauri desktop app MUST launch with functional donor, submitter, governance, and settings pages
- **FR-030**: REST/HTTP+JSON gateway MUST expose all 6 gRPC services
- **FR-031**: Web dashboard MUST provide donor and submitter feature parity with CLI

**Category 7: Operations (#41, #48, #50)**

- **FR-032**: Multi-stage Dockerfile MUST produce a minimal container image
- **FR-033**: Docker Compose MUST create a functional 3-node cluster
- **FR-034**: Energy metering MUST estimate per-node power within 20% of real draw
- **FR-035**: README MUST provide working quickstart instructions verified on a clean machine
- **FR-036**: Evidence artifact JSON schema and directory structure MUST be defined

**Category 8: Mesh LLM (#54)**

- **FR-037**: Router MUST select K-of-N expert nodes per output token using LLaMA-3 tokenizer (128K vocab)
- **FR-038**: Each expert MUST return top-256 (token_id, logit) pairs (~1.5KB per token)
- **FR-039**: Aggregator MUST compute weighted average of sparse logit distributions and sample next token
- **FR-040**: Self-prompting loop MUST generate actionable improvement tasks on 1-24 hour cadence
- **FR-041**: Action tiers MUST gate operations: read-only (no approval), suggest (human review), sandbox-test (automated validation), deploy-minor (2-of-3 quorum), deploy-major (full governance vote + 24h review)
- **FR-042**: Governance kill switch MUST immediately halt all inference and revert last 3 changes
- **FR-043**: System MUST gracefully degrade below 280 nodes (fall back to centralized model)

### Key Entities

- **AttestationChain**: Certificate chain with platform type, leaf/intermediate/root certs, cryptographic signatures, manufacturer OIDs
- **InclusionProof**: Merkle proof path from leaf hash to signed tree root, with Rekor public key verification
- **Lease**: Broker-issued task assignment with TTL, heartbeat-renewed, expiry triggers rescheduling
- **ContainmentAction**: Enforcement primitive (Freeze/Quarantine/Block/Revoke/Drain) with audit record, actor identity, reversibility
- **ConfidentialBundle**: AES-256-GCM encrypted job data with per-job ephemeral key, attestation-gated key release
- **MeshExpert**: GPU donor node running a small LLM, reporting capacity, health, and model metadata to the router
- **ActionTier**: Classification of mesh LLM output (read-only through deploy-major) with corresponding approval requirements

## Success Criteria

### Measurable Outcomes

- **SC-001**: Zero in-code TODO comments remain in src/ (currently 15 → 0)
- **SC-002**: Zero `#[ignore]` or `unimplemented!()` macros in tests/ (currently 8 → 0)
- **SC-003**: All 12 previously untested src/ modules have integration tests (0 → 12)
- **SC-004**: Total test count increases from 489 to 700+ with zero failures
- **SC-005**: Preemption latency measured at under 10ms on real hardware (Principle III)
- **SC-006**: Agent withdrawal leaves zero host residue verified by filesystem/process scan
- **SC-007**: 80% job completion at 30% churn over 72-hour run (Principle II)
- **SC-008**: Phase 1 LAN testnet: 3+ physical machines, cluster in <5s, R=3 job, failure recovery
- **SC-009**: All CI checks pass on Linux, macOS, and Windows
- **SC-010**: Mesh LLM generates 3.2+ tokens/second at K=4 experts, 100ms latency
- **SC-011**: Governance kill switch halts all inference within 1 second
- **SC-012**: Every functional requirement has a corresponding passing test on real hardware

## Clarifications

### Session 2026-04-17

- Q: What happens when coordinator quorum is lost? → A: Graceful degradation — local brokers continue dispatching from cached leases; ledger writes queue locally until quorum is restored. CRDT merge reconciles on rejoin. System does not halt.

## Assumptions

- Test hardware at `tensor01.dartmouth.edu` is available for the duration of development with SSH access
- The test host has Linux with KVM support for Firecracker testing
- GPU hardware may not be available on the test host; GPU-specific tests may require additional hardware or cloud instances
- macOS testing for Apple VF requires access to a macOS 13+ machine (developer workstation)
- Windows CI testing uses GitHub Actions runners (no dedicated Windows hardware needed)
- Slurm adapter testing requires access to a Slurm cluster; if not available on `tensor01`, this will use a minimal 2-node Slurm setup or be tested on a partner cluster
- Kubernetes adapter testing uses minikube/kind on CI, with optional real-cluster testing
- Cloud adapter testing requires at least one real cloud instance (AWS EC2 preferred); can be a spot instance for cost efficiency
- Mesh LLM testing at full distributed scale (280+ nodes) is not possible in this spec; Phase 0-1 (centralized model + 4-node ensemble proof of concept) is the target
- LLaMA-3-8B model weights are available via Hugging Face for mesh LLM testing
- The 72-hour churn simulation can run as a background job on the test cluster
- All external service dependencies (Rekor, BrightID, Twilio) use staging/sandbox instances where available
