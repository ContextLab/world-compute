# Feature Specification: Replace Implementation Stubs with Real Functionality

**Feature Branch**: `003-stub-replacement`  
**Created**: 2026-04-16  
**Status**: Draft  
**Input**: User description: "Address issue #7 and all sub-issues (#8–#26): replace all implementation stubs with real functionality across CLI, sandbox, attestation, identity, infrastructure, and network modules."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Donor Operates via CLI (Priority: P1)

A volunteer donor launches the World Compute CLI to join the network, check their status, pause/resume contribution, view earned credits, and read logs — all from a single terminal session.

**Why this priority**: The CLI is the primary user-facing entry point. Without functional CLI commands, no other feature can be exercised by end users. All five command groups (donor, job, cluster, governance, admin) are currently inert.

**Independent Test**: Can be fully tested by running each CLI subcommand (e.g., `worldcompute donor join`, `worldcompute job submit`) and verifying it dispatches to the correct module and returns meaningful output.

**Acceptance Scenarios**:

1. **Given** a compiled binary, **When** a user runs `worldcompute donor join --resource-cap 50%`, **Then** the agent lifecycle module initializes and the user sees confirmation of enrollment.
2. **Given** a running donor agent, **When** a user runs `worldcompute donor status`, **Then** the system displays current resource usage, trust score, and credit balance.
3. **Given** a compiled binary, **When** a user runs `worldcompute job submit manifest.yaml`, **Then** the job is validated, dispatched, and the user receives a job ID.
4. **Given** a running cluster, **When** a user runs `worldcompute cluster status`, **Then** peer count, ledger head, and cluster health are displayed.
5. **Given** appropriate governance role, **When** a user runs `worldcompute governance propose "Increase NCU cap"`, **Then** a proposal is created and broadcast to voters.
6. **Given** OnCallResponder role, **When** a user runs `worldcompute admin halt`, **Then** the system triggers an emergency halt with audit trail.

---

### User Story 2 - Submitter Runs a Sandboxed Workload (Priority: P1)

A job submitter sends a workload that gets assigned to a donor node. The donor's sandbox (Firecracker on Linux, Apple VF on macOS, or WASM for lightweight tasks) boots a real VM or runtime, loads the workload from the CID store, executes it in isolation, and enforces egress rules.

**Why this priority**: Sandboxed execution is the core value proposition. Without real VM/WASM lifecycle, the system cannot run any workloads.

**Independent Test**: Can be tested by submitting a sample workload and observing that a sandbox starts, executes, and terminates — producing output artifacts.

**Acceptance Scenarios**:

1. **Given** a Linux donor with Firecracker installed, **When** a workload is assigned, **Then** a microVM boots with the correct kernel, rootfs, and resource limits.
2. **Given** a macOS donor, **When** a workload is assigned, **Then** an Apple Virtualization.framework VM starts with the correct configuration.
3. **Given** a WASM-eligible workload, **When** assigned to any donor, **Then** the WASM module is fetched from the CID store, compiled via wasmtime, and executed with sandbox constraints.

---

### User Story 3 - Platform Verifies Donor Hardware Integrity (Priority: P2)

When a donor enrolls or is re-evaluated, the platform verifies their hardware attestation — validating the full certificate chain for TPM2, AMD SEV-SNP, Intel TDX, or Apple Secure Enclave — to assign an accurate trust score.

**Why this priority**: Trust scores gate what workloads a donor can run. Without real certificate-chain validation, trust scoring relies on structural checks only — sufficient for testing but not for production security guarantees.

**Independent Test**: Can be tested by presenting known-good and known-bad attestation quotes and verifying correct accept/reject decisions with full chain validation.

**Acceptance Scenarios**:

1. **Given** a donor with a valid TPM2 endorsement key chain, **When** attestation is verified, **Then** the full EK→AIK→quote chain is validated and trust score reflects hardware-backed integrity.
2. **Given** a donor with AMD SEV-SNP, **When** attestation is verified, **Then** the ARK→ASK→VCEK chain is validated against AMD's root certificates.
3. **Given** a donor presenting an Apple Secure Enclave attestation, **When** verified, **Then** the DeviceCheck/App Attest API confirms the device's authenticity.
4. **Given** a policy-engine signature check, **When** a manifest is submitted, **Then** Ed25519 signatures are verified against the submitter's registered public key (not just structural checks).

---

### User Story 4 - Donor Proves Personhood and Links Identity (Priority: P2)

A new donor verifies their identity through BrightID (primary), OAuth2 providers, or phone verification to earn humanity points and participate in governance.

**Why this priority**: Identity verification gates governance participation and HP-weighted voting. Without real provider integrations, no user can complete enrollment beyond structural stubs.

**Independent Test**: Can be tested by initiating a BrightID verification flow and confirming the HTTP call to BrightID's API returns a valid verification status.

**Acceptance Scenarios**:

1. **Given** a donor with a BrightID account, **When** they verify via WorldCompute context, **Then** the system calls BrightID's verification API and records the result.
2. **Given** a donor choosing OAuth2 login, **When** they select GitHub as provider, **Then** a real authorization code flow completes and links their identity.
3. **Given** a donor choosing phone verification, **When** they submit their phone number, **Then** an SMS code is sent via the configured provider and can be verified.

---

### User Story 5 - Platform Anchors Decisions to Transparency Logs (Priority: P3)

Policy decisions and artifact signatures are recorded in an immutable transparency log (Sigstore Rekor) so that any participant can audit the history of approvals, deployments, and governance actions.

**Why this priority**: Transparency anchoring is essential for trust but does not block core compute operations. It can be added after the compute pipeline is functional.

**Independent Test**: Can be tested by submitting a transparency log entry to Rekor's API and verifying it appears in the log.

**Acceptance Scenarios**:

1. **Given** an artifact signature event, **When** the registry records it, **Then** a Rekor log entry is created via the REST API and a receipt is returned.
2. **Given** a policy decision, **When** the ledger anchors it, **Then** the decision hash is recorded in the transparency log with a verifiable timestamp.

---

### User Story 6 - Operators Monitor System Health (Priority: P3)

Cluster operators configure an OpenTelemetry endpoint and receive traces and metrics from all nodes, enabling observability dashboards and alerting.

**Why this priority**: Observability supports operations but is not required for core functionality.

**Independent Test**: Can be tested by configuring an OTLP endpoint and verifying that traces and metrics are exported.

**Acceptance Scenarios**:

1. **Given** `otel_endpoint` is configured, **When** the telemetry module initializes, **Then** traces and metrics are exported to the configured OTLP endpoint.

---

### User Story 7 - Coordinators Achieve Consensus (Priority: P3)

Multiple coordinator nodes elect a leader and replicate the scheduling log via Raft consensus, ensuring the cluster survives coordinator failures without losing job state.

**Why this priority**: Consensus is critical for multi-coordinator deployments but single-coordinator mode works for initial testing.

**Independent Test**: Can be tested by starting multiple coordinator instances and verifying leader election and log replication.

**Acceptance Scenarios**:

1. **Given** three coordinator nodes, **When** they start, **Then** a leader is elected via openraft within the configured timeout.
2. **Given** a leader coordinator fails, **When** the failure is detected, **Then** a new leader is elected and scheduling resumes.

---

### User Story 8 - Nodes Discover Peers on the Network (Priority: P3)

Donor and coordinator nodes discover each other through DNS seed nodes and detect their NAT topology to establish connectivity.

**Why this priority**: Network discovery bootstraps the mesh but mDNS already provides local discovery for development.

**Independent Test**: Can be tested by resolving DNS seed addresses and verifying peer records are returned; NAT detection can be tested with a STUN server.

**Acceptance Scenarios**:

1. **Given** a new node starting, **When** it queries DNS seeds, **Then** it receives a list of bootstrap peer addresses to connect to.
2. **Given** a node behind a NAT, **When** NAT detection runs, **Then** the correct NAT type (direct, full cone, symmetric, etc.) is identified via STUN.

---

### Edge Cases

- What happens when a BrightID node is unreachable during verification? The system should return a clear error and allow retry, not silently pass or permanently fail.
- What happens when Firecracker is not installed on a Linux donor? The sandbox should report the missing dependency and mark the donor as WASM-only capable.
- What happens when an OAuth2 provider revokes app credentials? The system should fail gracefully with an actionable error message and not crash.
- What happens when the Rekor transparency log is temporarily unavailable? Policy decisions should still proceed but flag the anchoring as pending, with retry.
- What happens when a WASM module in the CID store is corrupted? The module should fail compilation with a clear error and the task should be rescheduled.
- What happens when all coordinator nodes fail simultaneously? The system should detect the condition and refuse new job submissions until a coordinator recovers.
- What happens when DNS seed nodes return stale peer addresses? The node should attempt connection, detect failure, and fall back to mDNS or cached peers.
- What happens when provider credentials (OAuth2, BrightID, Twilio, Apple) expire mid-operation? The current operation fails with a clear error message indicating credential expiry. The agent must be restarted with updated credentials; no hot-reload.
- What happens when Firecracker API socket returns an error during VM setup (invalid kernel, insufficient resources)? The system fails immediately, marks the donor as incompatible for this workload class, and reschedules the task to another donor. Maximum 3 donors attempted per task — after 3 failures the task is marked as failed with a clear error listing all attempted donors and their failure reasons.

## Requirements *(mandatory)*

### Functional Requirements

**CLI Wiring (Issues #8–#12)**
- **FR-001**: System MUST dispatch all donor CLI subcommands (join, status, pause, resume, leave, credits, logs) to the agent lifecycle module.
- **FR-002**: System MUST dispatch all job CLI subcommands (submit, status, results, cancel, list) to the scheduler module.
- **FR-003**: System MUST dispatch all cluster CLI subcommands (status, peers, ledger-head) to the network/ledger modules.
- **FR-004**: System MUST dispatch all governance CLI subcommands (propose, list, vote, report) to the governance module.
- **FR-005**: System MUST dispatch all admin CLI subcommands (halt, resume, ban, audit) to the admin service, enforcing OnCallResponder role requirements.

**Sandbox VM Lifecycle (Issues #13–#15)**
- **FR-006**: System MUST configure and start a Firecracker microVM via the API socket, including machine config, boot source, drives, network interfaces, and instance start.
- **FR-006a**: When a Firecracker VM configuration fails, the system MUST mark the donor as incompatible for that workload class and reschedule to another donor, with a maximum of 3 donor attempts per task before marking the task as failed.
- **FR-007**: System MUST start, pause, stop, and save Apple Virtualization.framework VMs via a Swift FFI bridge or helper binary.
- **FR-008**: System MUST fetch WASM modules from the CID store, compile them via wasmtime, and execute them within sandbox constraints.

**Attestation & Crypto (Issues #16–#18)**
- **FR-009**: System MUST perform real Ed25519 signature verification against registered public keys in the policy engine, replacing structural-only checks.
- **FR-010**: System MUST validate the full certificate chain for TPM2 (EK chain), AMD SEV-SNP (ARK→ASK→VCEK), and Intel TDX (DCAP) attestation quotes.
- **FR-011**: System MUST verify Apple Secure Enclave attestation via Apple's DeviceCheck/App Attest API.

**Identity & Verification (Issues #19–#21)**
- **FR-012**: System MUST call BrightID's verification API via HTTP to check a donor's personhood status in the WorldCompute context and record the result.
- **FR-013**: System MUST implement real OAuth2 authorization code flows for email, GitHub, Google, and Twitter providers.
- **FR-014**: System MUST send and verify SMS/voice codes via a phone verification provider.

**Infrastructure (Issues #22–#24)**
- **FR-015**: System MUST submit transparency log entries to Sigstore Rekor's REST API and return verifiable receipts.
- **FR-016**: System MUST export traces and metrics to a configured OTLP endpoint when `otel_endpoint` is set.
- **FR-017**: System MUST implement Raft leader election and log replication via openraft for coordinator consensus.

**Network (Issues #25–#26)**
- **FR-018**: System MUST detect NAT topology using STUN-based probing, replacing the stub that always returns Direct.
- **FR-019**: System MUST resolve DNS seed node addresses for bootstrap peer discovery, replacing placeholder values.

### Key Entities

- **DonorAgent**: Represents a volunteer node's lifecycle state, resource caps, trust score, and credit balance.
- **Sandbox**: An isolated execution environment (Firecracker VM, Apple VF VM, or WASM runtime) with enforced resource and egress constraints.
- **AttestationQuote**: A hardware-backed integrity proof with a certificate chain linking to a platform root of trust.
- **IdentityVerification**: A record of a donor's proof-of-personhood or identity linkage via BrightID, OAuth2, or phone.
- **TransparencyEntry**: An immutable log record of a policy decision or artifact signature, anchored in Sigstore Rekor.
- **CoordinatorState**: A Raft-replicated state machine tracking job scheduling, leader election, and log replication.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 5 CLI command groups (donor, job, cluster, governance, admin) accept and dispatch every defined subcommand, with no "not yet implemented" messages remaining.
- **SC-002**: A sample workload submitted via CLI completes end-to-end through sandbox execution and returns results within 60 seconds on each supported platform.
- **SC-003**: Hardware attestation verification correctly accepts valid certificate chains and rejects invalid/expired chains with 100% accuracy on test vectors.
- **SC-004**: At least one identity verification path (BrightID, OAuth2, or phone) completes a full end-to-end flow from user initiation to recorded verification.
- **SC-005**: Transparency log entries are retrievable from Sigstore Rekor after submission, with verifiable timestamps and receipts.
- **SC-006**: Telemetry data (traces and metrics) appears at a configured OTLP endpoint within 30 seconds of system activity.
- **SC-007**: A 3-node coordinator cluster completes leader election and survives a single-node failure without losing scheduled jobs.
- **SC-008**: NAT detection correctly identifies at least 3 NAT types (direct, full cone, symmetric) when tested against known network configurations.
- **SC-009**: DNS seed resolution returns valid peer addresses and nodes successfully bootstrap from them.
- **SC-010**: All existing tests continue to pass after stub replacement, plus new integration tests cover each replaced stub — zero regressions.

## Clarifications

### Session 2026-04-16

- Q: What end-to-end completion target defines success for a minimal test workload? → A: Under 60 seconds on each supported platform.
- Q: What happens when provider credentials (OAuth2, BrightID, Twilio) expire or are rotated mid-operation? → A: Fail the current operation with a clear error; require agent restart for new credentials.
- Q: What happens when Firecracker API socket returns an error during VM configuration? → A: Fail immediately, mark donor as incompatible for this workload, reschedule to another donor.

## Assumptions

- Firecracker binary, guest kernel, and rootfs images are available on the host or fetchable from the CID store. Firecracker testing requires a Linux environment with KVM support.
- Apple Virtualization.framework testing requires macOS 12+ on Apple Silicon or supported Intel Macs.
- BrightID's verification API (v6) remains stable and the WorldCompute context is registered.
- OAuth2 provider app credentials (client ID, client secret) are configured via environment variables or a secrets manager — not hardcoded.
- Phone/SMS verification requires a funded account with the chosen provider (e.g., Twilio). Testing will use the provider's sandbox/test mode.
- Sigstore Rekor's public instance (rekor.sigstore.dev) is used for development; production may use a private instance.
- AMD, Intel, and Apple root certificates for attestation chain validation are available as bundled trust anchors or fetched from vendor APIs.
- The existing 422 tests serve as a regression baseline — no test may be removed or weakened to accommodate stub replacement.
- Single-coordinator mode remains functional as a fallback when Raft consensus is not configured.
- DNS seed domain names will be registered and configured before the network bootstrap feature is deployed.
