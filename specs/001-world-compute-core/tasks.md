# Tasks: World Compute — Core Cluster v1

**Input**: Design documents from `/specs/001-world-compute-core/`
**Prerequisites**: plan.md (required), spec.md (required), data-model.md, contracts/

---

## Phase 1: Setup

**Purpose**: Cargo workspace initialization, proto scaffolding, CI, and core type definitions

- [ ] T001 Initialize Cargo workspace with root `Cargo.toml` defining workspace members: `src/`, `gui/src-tauri/`, `adapters/slurm/`, `adapters/kubernetes/`, `adapters/cloud/` in `Cargo.toml`
- [ ] T002 Create `src/lib.rs` with top-level module declarations for agent, sandbox, preemption, scheduler, network, data_plane, verification, ledger, credits, acceptable_use, governance, telemetry, cli
- [ ] T003 [P] Create `proto/donor.proto` with DonorService definition (6 RPCs: Enroll, Heartbeat, GetDonorStatus, UpdateConsent, Withdraw, ConfirmWithdraw) per `contracts/donor.proto.md`
- [ ] T004 [P] Create `proto/submitter.proto` with SubmitterService definition (6 RPCs: SubmitJob, GetJob, StreamJobLogs, CancelJob, ListJobs, FetchResult) per `contracts/submitter.proto.md`
- [ ] T005 [P] Create `proto/cluster.proto` with ClusterService definition (4 RPCs: GetClusterStatus, ListPeers, GetLedgerHead, VerifyReceipt) per `contracts/cluster.proto.md`
- [ ] T006 [P] Create `proto/governance.proto` with GovernanceService definition (4 RPCs: ListProposals, CreateProposal, CastVote, GetReport) per `contracts/governance.proto.md`
- [ ] T007 [P] Create `proto/admin.proto` with AdminService definition (4 RPCs: HaltDispatch, ResumeDispatch, BanNode, RotateCoordinatorKey) per `contracts/admin.proto.md`
- [ ] T008 Configure `build.rs` with tonic-build for proto compilation and CI workflow (GitHub Actions) for `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt --check` in `.github/workflows/ci.yml`
- [ ] T009 [P] Add dependency declarations to `Cargo.toml`: rust-libp2p, tonic, prost, clap, tauri, opentelemetry (+ sdk/exporters), reed-solomon-erasure, ed25519-dalek, threshold-crypto, cid, openraft, wasmtime, tokio, serde
- [ ] T010 [P] Define core type aliases and newtypes in `src/lib.rs`: `Cid`, `PeerId`, `NcuAmount`, `Timestamp`, `DurationMs`, `TrustScore`, `SignatureBundle`, `AttestationQuote` per data-model §4 Type Reference Appendix

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared types, error model, telemetry, identity, and base infrastructure that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [ ] T011 Define shared error model with 20 canonical error codes in `src/lib.rs` per `contracts/errors.md` (WC-001 through WC-020, mapping to gRPC status codes)
- [ ] T012 [P] Define Platform enum (`Linux`, `MacOS`, `Windows`, `Browser`, `Mobile`) and SandboxCapability enum (`Firecracker`, `AppleVF`, `HyperV`, `WSL2`, `WasmOnly`) in `src/sandbox/mod.rs`
- [ ] T013 [P] Define TrustTier enum (T0–T4) with score thresholds and replication constraints in `src/verification/trust_score.rs` per data-model §3.16
- [ ] T014 [P] Define CaliberClass enum (C0–C4) with NCU/hr rates and redemption matching rules in `src/credits/caliber.rs` per data-model §3.17
- [ ] T015 [P] Define AcceptableUseClass enum and ShardCategory enum (`Public`, `OpaqueEncrypted`, `EuResident`, `UsResident`, `UkResident`, `JpResident`) in `src/acceptable_use/mod.rs` per data-model §3.21
- [ ] T016 [P] Define PriorityClass enum (`DonorRedemption`, `PaidSponsored`, `PublicGood`, `SelfImprovement`), ConfidentialityLevel enum, and VerificationMethod enum in `src/scheduler/mod.rs` per data-model §3.5
- [ ] T017 [P] Define WorkloadType enum (`OciContainer`, `WasmModule`) and PreemptClass enum (`Yieldable`, `Checkpointable`, `Restartable`) in `src/scheduler/mod.rs`
- [ ] T018 Implement Ed25519 key generation, PeerId derivation, and key persistence (load-or-create at enrollment) using ed25519-dalek in `src/agent/identity.rs`
- [ ] T019 [P] Initialize OpenTelemetry tracing provider with structured logs, metrics, and traces (FR-105) in `src/telemetry/mod.rs`; configure span naming convention `v1.<ServiceName>/<MethodName>`
- [ ] T020 [P] Implement donor-privacy redaction filter for telemetry emit layer (FR-106): strip PII, hostnames, local IPs, usernames, MAC addresses in `src/telemetry/redaction.rs`
- [ ] T021 Define Agent struct with fields per data-model §3.1 and AgentState enum (Enrolling, Idle, Working, Paused, Withdrawing) with state machine transitions in `src/agent/mod.rs`
- [ ] T022 [P] Define Donor struct with fields per data-model §3.2 (consent_classes, shard_allowlist, credit_balance, caliber_class) in `src/agent/donor.rs`
- [ ] T023 [P] Define Node struct with fields per data-model §3.3 and NodeState enum (Joining, Idle, Leased, Preempted, Quarantined, Offline) with state machine transitions in `src/agent/node.rs`
- [ ] T024 [P] Define LedgerEntry struct, LedgerEntryType enum (`CreditEarn`, `CreditSpend`, `CreditDecay`, `CreditRefund`, `GovernanceRecord`, `AuditRecord`), and Merkle chain linking in `src/ledger/entry.rs` per data-model §3.12.1
- [ ] T025 [P] Define LedgerShard and MerkleRoot structs in `src/ledger/entry.rs` per data-model §3.12.2–3.12.3
- [ ] T026 Implement CIDv1 content-addressed object store (put, get, has, delete with SHA-256 hashing) in `src/data_plane/cid_store.rs` per FR-070
- [ ] T027 [P] Implement configuration and settings management (load from file, env vars, CLI overrides; scoped working directory, OTel endpoint, cpu-cap) in `src/agent/config.rs`
- [ ] T028 [P] Define ResourceEnvelope struct (cpu_millicores, ram_bytes, gpu_class, gpu_vram_bytes, scratch_bytes, network_egress_bytes, walltime_budget_ms) in `src/scheduler/mod.rs` per data-model §3.5

**Checkpoint**: Foundation ready — user story implementation can now begin in parallel

---

## Phase 3: US1 — Donor Joins and Contributes Idle Compute (Priority: P1) MVP

**Goal**: A volunteer installs the agent, joins a cluster, executes sandboxed work, earns credits, and yields instantly on local activity

**Independent Test**: Install agent on single machine; enroll via CLI; submit trivial job; verify correct result, credits earned, keyboard preemption, clean withdrawal

### Sandbox Drivers

- [ ] T029 [P] [US1] Implement Sandbox trait with lifecycle methods (create, start, freeze, checkpoint, terminate, cleanup) and platform-detection factory in `src/sandbox/mod.rs` per FR-010
- [ ] T030 [P] [US1] Implement Firecracker sandbox driver (Linux KVM microVM creation, guest image loading, scoped working directory, network isolation, cleanup) in `src/sandbox/firecracker.rs` per FR-010, FR-011
- [ ] T031 [P] [US1] Implement Apple Virtualization.framework sandbox driver (macOS VM lifecycle, no host filesystem/credential/peripheral access) in `src/sandbox/apple_vf.rs` per FR-010, FR-011
- [ ] T032 [P] [US1] Implement Hyper-V sandbox driver (Windows Pro VM lifecycle with WHPX/WSL2 fallback for Windows Home) in `src/sandbox/hyperv.rs` per FR-010, FR-011
- [ ] T033 [P] [US1] Implement WASM sandbox driver using wasmtime (Tier 3 browser/low-trust workloads, resource limits) in `src/sandbox/wasm.rs` per FR-021
- [ ] T034 [P] [US1] Implement GPU passthrough verification (singleton IOMMU group check, ACS-override rejection) in `src/sandbox/mod.rs` per FR-012

### Preemption

- [ ] T035 [US1] Implement sovereignty event detection (keyboard/mouse activity, foreground app launch, AC-power disconnect, thermal threshold, memory pressure) in `src/preemption/triggers.rs` per FR-040
- [ ] T036 [US1] Implement preemption supervisor: SIGSTOP within 10ms of sovereignty trigger, checkpoint attempt within 500ms, full resource release in `src/preemption/supervisor.rs` per FR-040, FR-041

### Discovery & Network

- [ ] T037 [P] [US1] Implement mDNS peer discovery (find LAN peers within 2 seconds, no internet required) using rust-libp2p mDNS in `src/network/discovery.rs` per FR-060
- [ ] T038 [P] [US1] Implement Kademlia DHT bootstrap and self-organization for internet-connected nodes in `src/network/discovery.rs` per FR-061

### Agent Lifecycle

- [ ] T039 [US1] Implement agent enrollment flow: generate Ed25519 identity, platform detection, sandbox capability probe, benchmark, register with broker in `src/agent/mod.rs` per FR-001, FR-002
- [ ] T040 [US1] Implement agent heartbeat loop (periodic heartbeat to broker, state reporting, lease management) in `src/agent/mod.rs` per data-model §3.1
- [ ] T041 [US1] Implement agent pause/resume commands (checkpoint active work, transition AgentState, stop/restart advertising capacity) in `src/agent/mod.rs` per FR-002
- [ ] T042 [US1] Implement agent withdrawal flow: stop all work, wipe scoped working directory, remove all host state (no files, processes, scheduled tasks, startup hooks remain) in `src/agent/mod.rs` per FR-004
- [ ] T043 [US1] Implement consent management: granular per-workload-class opt-in/opt-out, refuse unoptted classes in `src/agent/donor.rs` per FR-003

### Attestation & Verification

- [ ] T044 [P] [US1] Implement cryptographic attestation (TPM 2.0 PCR quotes on x86, Apple Secure Enclave signing, soft attestation for WASM) in `src/verification/attestation.rs` per FR-013
- [ ] T045 [P] [US1] Implement Trust Score computation: `clamp(0,1, 0.5*R_consistency + 0.3*R_attestation + 0.2*R_age) * (1 - P_recent_failures)` with 7-day 0.5 cap in `src/verification/trust_score.rs` per FR-052

### Credits

- [ ] T046 [P] [US1] Implement NCU credit computation: 1 TFLOP/s FP32-second normalized with DRF dominant-dimension accounting in `src/credits/ncu.rs` per FR-050
- [ ] T047 [P] [US1] Define Credit (NCU Ledger Account) materialized view struct with balance, decay floor, trailing earn rate in `src/credits/ncu.rs` per data-model §3.11

### Node State Machine

- [ ] T048 [US1] Implement Node state machine transitions (Joining→Idle→Leased→Preempted→Idle, Idle→Quarantined, Idle→Offline→Idle) with heartbeat-based failure detection in `src/agent/node.rs` per data-model §3.3

### CLI

- [ ] T049 [US1] Implement CLI `worldcompute donor` subcommand with join, status, pause, resume, leave, credits, credits --verify, logs subcommands using clap in `src/cli/donor.rs` per FR-002, FR-054, FR-090

### gRPC Service

- [ ] T050 [US1] Implement DonorService gRPC server (Enroll, Heartbeat, GetDonorStatus, UpdateConsent, Withdraw, ConfirmWithdraw handlers) using tonic in `src/agent/donor_service.rs` per `contracts/donor.proto.md`

### Integration Test

- [ ] T051 [US1] Integration test: single donor joins, receives trivial job, earns NCU credits, yields on simulated keyboard activity, withdraws cleanly with no host residue in `tests/integration/test_donor_lifecycle.rs`

**Checkpoint**: US1 fully functional — donor can join, contribute, earn credits, and withdraw

---

## Phase 4: US2 — Submitter Runs a Job and Gets a Correct Result (Priority: P1)

**Goal**: Submitter submits a job, system stages inputs, schedules replicated execution, verifies result via quorum, returns cryptographic proof of correctness

**Independent Test**: Submit known-answer SHA-256 job; verify result matches; verify signed receipt; verify wrong-result donor is detected

### Job Model

- [ ] T052 [P] [US2] Define JobManifest struct with all fields (manifest_cid, workflow_template, priority_class, confidentiality_level, acceptable_use_classes, verification_method) in `src/scheduler/manifest.rs` per data-model §3.5
- [ ] T053 [P] [US2] Define WorkflowTemplate and TaskTemplate structs (task_templates, dependency_edges, fan_out_params) in `src/scheduler/manifest.rs` per data-model §3.5
- [ ] T054 [P] [US2] Implement job manifest parsing, validation (CID verification, signature check, DAG cycle detection, acceptable-use filter) in `src/scheduler/manifest.rs` per FR-020
- [ ] T055 [P] [US2] Define Workflow struct with WorkflowState enum (Pending, Running, Checkpointed, Completed, Failed) and state transitions in `src/scheduler/workflow.rs` per data-model §3.6
- [ ] T056 [P] [US2] Define Job struct with JobState enum (Queued, Dispatching, Running, Verifying, Completed, Checkpointed, Failed) and state transitions in `src/scheduler/job.rs` per data-model §3.7
- [ ] T057 [P] [US2] Define Task struct with TaskState enum (Ready, Dispatched, Running, Checkpointing, Verifying, Accepted, Failed) and state transitions in `src/scheduler/task.rs` per data-model §3.8
- [ ] T058 [P] [US2] Define Replica struct with ReplicaState enum (Leased, Running, Checkpointing, Completed, Failed, Preempted, Expired) in `src/scheduler/replica.rs` per data-model §3.9

### Scheduler

- [ ] T059 [US2] Implement local scheduler lease manager: task-to-node matching via ClassAd-style capability comparison, lease issuance, lease renewal via heartbeat, lease expiry in `src/scheduler/local.rs` per FR-031
- [ ] T060 [US2] Implement multi-factor priority scoring: `P(job) = 0.35*S_ncu + 0.25*S_vote + 0.15*S_size + 0.15*S_age + 0.10*S_cool` with starvation-freedom guarantee in `src/scheduler/priority.rs` per FR-032

### Verification

- [ ] T061 [P] [US2] Implement R=3 canonical-hash quorum verification (collect result hashes from replicas, majority agreement, trust-tier-aware replica selection) in `src/verification/quorum.rs` per FR-024
- [ ] T062 [P] [US2] Implement 3% random audit re-execution on independent high-trust nodes to detect quorum collusion in `src/verification/audit.rs` per FR-025

### Checkpointing

- [ ] T063 [US2] Define Checkpoint struct (checkpoint_cid, task_id, replica_id, sequence_number, checkpoint_type, state_blob_cid, shard_cids) and implement checkpoint commit flow in `src/data_plane/staging.rs` per data-model §3.22, FR-023

### WorkUnitReceipt

- [ ] T064 [US2] Define WorkUnitReceipt struct with all fields (receipt_id, quorum_node_ids, dissenting_node_ids, coordinator_signature, ncu_awarded_per_node, provenance_chain) and CBOR serialization in `src/verification/receipt.rs` per data-model §3.10

### Ledger

- [ ] T065 [P] [US2] Implement CRDT OR-Map balance view (read balance, apply earn/spend/decay entries, merge replicas) in `src/ledger/crdt.rs` per FR-051
- [ ] T066 [P] [US2] Implement threshold signing for ledger entries (t-of-n coordinator signatures, signature collection, verification) in `src/ledger/threshold_sig.rs` per FR-051
- [ ] T067 [P] [US2] Implement transparency log anchoring: compute cross-shard MerkleRoot every 10 minutes, anchor to Sigstore Rekor in `src/ledger/transparency.rs` per FR-051

### Data Plane

- [ ] T068 [P] [US2] Implement RS(10,18) erasure encoding and decoding with repair capability in `src/data_plane/erasure.rs` per FR-071
- [ ] T069 [P] [US2] Implement geographic and AS-diverse shard placement (>=3 continents, <=2 shards/country, >=1 shard/AS) in `src/data_plane/placement.rs` per FR-071, FR-074

### Submitter Entity

- [ ] T070 [P] [US2] Define Submitter struct (submitter_id, credit_balance, acceptable_use_standing, sponsor_tier) with invariant that sponsor_tier never affects scheduling in `src/scheduler/submitter.rs` per data-model §3.4

### Job Input/Output Staging

- [ ] T071 [US2] Implement job input staging (resolve input CIDs from data plane, mount into sandbox) and output capture (hash output, store to CID store) in `src/data_plane/staging.rs` per FR-070

### gRPC Service

- [ ] T072 [US2] Implement SubmitterService gRPC server (SubmitJob, GetJob, StreamJobLogs, CancelJob, ListJobs, FetchResult handlers) using tonic in `src/scheduler/submitter_service.rs` per `contracts/submitter.proto.md`

### CLI

- [ ] T073 [US2] Implement CLI `worldcompute job` subcommand with submit, status, results, cancel, list subcommands in `src/cli/submitter.rs` per FR-090

### Integration Test

- [ ] T074 [US2] Integration test: submit known-answer SHA-256 job, verify correct result, verify signed WorkUnitReceipt, verify NCU credit/debit entries in ledger in `tests/integration/test_submitter_flow.rs`

**Checkpoint**: US1 and US2 fully functional — donors contribute, submitters get verified results

---

## Phase 5: US3 — Zero-Config LAN Cluster (Priority: P1)

**Goal**: 2-3 machines on an isolated LAN form a cluster with no internet, no config, no admin; later merge into global cluster without data loss

**Independent Test**: Air-gap 3 machines, install agent, verify cluster forms in <5s, run job, re-enable internet, verify DHT merge with ledger integrity

### Transport

- [ ] T075 [P] [US3] Implement QUIC transport using rust-libp2p quic module (connection setup, stream multiplexing, TLS 1.3) in `src/network/transport.rs`
- [ ] T076 [P] [US3] Implement TCP fallback transport using rust-libp2p tcp module in `src/network/transport.rs`

### Gossip & Broker

- [ ] T077 [P] [US3] Implement GossipSub protocol for broker broadcast (task announcements, capacity updates, lease grants) using rust-libp2p gossipsub in `src/network/gossip.rs`
- [ ] T078 [US3] Implement regional broker with ClassAd-style matchmaking (node roster, capability matching, disjoint-AS placement enforcement, standby pool management) in `src/scheduler/broker.rs` per FR-031, FR-034

### NAT Traversal

- [ ] T079 [US3] Implement NAT traversal: UPnP-IGD/NAT-PMP first, then libp2p DCUtR hole punching, then Circuit Relay v2 as final fallback in `src/network/transport.rs` per FR-062

### DNS Bootstrap

- [ ] T080 [US3] Implement DNS bootstrap seed resolution for internet-connected fresh agents in `src/network/discovery.rs` per FR-061

### Cluster Entity

- [ ] T081 [US3] Define Cluster struct (cluster_id, parent_cluster_id, coordinator_ids, broker_ids, node_count, dht_bootstrap_addrs) with merge semantics in `src/network/cluster.rs` per data-model §3.13
- [ ] T082 [US3] Implement DHT island merge: detect internet connectivity, merge LAN DHT with global DHT, reconcile credit ledger via CRDT merge without data loss in `src/network/discovery.rs` per FR-063

### Coordinator

- [ ] T083 [P] [US3] Define Coordinator struct (coordinator_id, shard_id, raft_term, raft_role, threshold_share) and Broker struct (broker_id, region_code, node_roster) in `src/scheduler/coordinator.rs` per data-model §3.14–3.15
- [ ] T084 [US3] Implement coordinator election using openraft (Raft consensus for small coordinator set, leader election, log replication) in `src/scheduler/coordinator.rs`

### gRPC Service

- [ ] T085 [US3] Implement ClusterService gRPC server (GetClusterStatus, ListPeers, GetLedgerHead, VerifyReceipt handlers) using tonic in `src/network/cluster_service.rs` per `contracts/cluster.proto.md`

### CLI

- [ ] T086 [US3] Implement CLI `worldcompute cluster` subcommand with status, peers, ledger-head subcommands in `src/cli/cluster.rs` per FR-090

### Integration Test

- [ ] T087 [US3] Integration test: 3 machines on isolated LAN, no internet, form cluster in <5s, run R=3 job, verify correct result, enable internet, verify DHT merge with ledger integrity in `tests/integration/test_lan_cluster.rs`

**Checkpoint**: All P1 user stories (US1, US2, US3) functional — MVP complete

---

## Phase 6: US4 — Integrator Connects Existing Cluster / Cloud (Priority: P2)

**Goal**: HPC centers (Slurm), Kubernetes clusters, and cloud tenants contribute capacity via adapter components

**Independent Test**: Install Slurm adapter on 2-node testbed; submit job; verify dispatch through Slurm scheduler; verify correct result

### ComputeAdapter Trait

- [ ] T088 [P] [US4] Define ComputeAdapter trait (register, deregister, submit_task, get_status, get_capacity, health_check) in `adapters/mod.rs`

### Slurm Adapter

- [ ] T089 [US4] Implement Slurm pilot-job gateway adapter: connect to Slurm head node, advertise aggregate capacity, dispatch tasks as Slurm jobs, report results in `adapters/slurm/mod.rs` per FR-064
- [ ] T090 [US4] Implement Slurm adapter CLI install/configure/status commands in `adapters/slurm/cli.rs`

### Kubernetes Adapter

- [ ] T091 [US4] Implement K8s operator: watch ClusterDonation CRD, create Pods in configured namespace, enforce resource limits, report results in `adapters/kubernetes/mod.rs` per FR-064
- [ ] T092 [US4] Define ClusterDonation CRD schema (cpuCap, memoryCap, jobClasses, namespace) and K8s operator deployment manifest in `adapters/kubernetes/crd.yaml`

### Cloud Adapter

- [ ] T093 [P] [US4] Implement cloud instance-metadata attester: verify instance identity via AWS/GCP/Azure metadata service, join as first-class donor in `adapters/cloud/mod.rs` per FR-064

### Integration Test

- [ ] T094 [US4] Integration test: Slurm adapter dispatches SHA-256 test job, verifies correct result, adapter appears as aggregate node in cluster status in `tests/integration/test_slurm_adapter.rs`

**Checkpoint**: US4 functional — existing infrastructure can contribute to World Compute

---

## Phase 7: US5 — Philanthropist Contributes Funds (Priority: P2)

**Goal**: Transparent donation infrastructure with public financial reporting

**Independent Test**: Verify public website lists legal entity, donation channels, quarterly report template, and no-priority-for-money policy

- [ ] T095 [P] [US5] Create Apache 2.0 LICENSE file at repository root per FR-099
- [ ] T096 [P] [US5] Create legal entity documentation placeholder (501(c)(3) public charity, Delaware, ISRG model, EAR/OFAC compliance notes) in `docs/legal/entity.md` per FR-100
- [ ] T097 [P] [US5] Create quarterly financial report template (inflows, outflows by category, audit status, incident disclosures) in `docs/legal/quarterly-report-template.md` per FR-101
- [ ] T098 [P] [US5] Create governance bylaws placeholder documenting two-body structure (TSC + Board), seat limits, and financial-donation-no-priority refusal mechanism in `docs/governance/bylaws.md` per FR-102, FR-103
- [ ] T099 [US5] Create public funding page placeholder with donation channels, public ledger link, and sponsorship tier documentation (charitable, not transactional) in `docs/funding/README.md` per FR-101

**Checkpoint**: US5 documentation and legal scaffolding in place

---

## Phase 8: US6 — Governance Member Proposes and Votes (Priority: P3)

**Goal**: TSC/Board members propose policy changes, vote, and record outcomes on the tamper-evident ledger

**Independent Test**: Submit governance proposal via CLI; verify in proposal list; cast votes; verify ledger record

### Entities

- [ ] T100 [P] [US6] Define GovernanceProposal struct with ProposalType enum (PolicyChange, AcceptableUseRule, PriorityClassRebalance, EmergencyHalt, ConstitutionAmendment) and ProposalState enum (Draft, Open, Passed, Rejected, Withdrawn, Enacted) in `src/governance/proposal.rs` per data-model §3.18
- [ ] T101 [P] [US6] Define Vote struct with VoteChoice enum (Yes, No, Abstain), signature verification, one-vote-per-voter enforcement in `src/governance/vote.rs` per data-model §3.19

### Voting System

- [ ] T102 [US6] Implement compute proposal board: submit proposals, list active proposals, self-voting exclusion (FR-059) in `src/governance/board.rs` per FR-055
- [ ] T103 [US6] Implement Humanity Points (HP) system: layered composite Sybil resistance score (email 1HP, phone 3HP, social 2HP, vouching 2HP, proof-of-personhood 3HP, active donor 5HP) in `src/governance/humanity_points.rs` per FR-057
- [ ] T104 [US6] Implement quadratic voting: vote cost scales as n-squared, per-epoch 20-vote budget, anomaly detection for sock-puppet campaigns in `src/governance/voting.rs` per FR-058
- [ ] T105 [US6] Implement proposal lifecycle: Draft→Open→Passed/Rejected→Enacted with quorum rules, write outcome to ledger as GovernanceRecord LedgerEntry in `src/governance/proposal.rs` per FR-104

### gRPC Service

- [ ] T106 [US6] Implement GovernanceService gRPC server (ListProposals, CreateProposal, CastVote, GetReport handlers) using tonic in `src/governance/governance_service.rs` per `contracts/governance.proto.md`

### Admin Service

- [ ] T107 [P] [US6] Implement AdminService gRPC server (HaltDispatch, ResumeDispatch, BanNode, RotateCoordinatorKey handlers) with admin-role mTLS cert requirement in `src/governance/admin_service.rs` per `contracts/admin.proto.md`

### CLI

- [ ] T108 [US6] Implement CLI `worldcompute governance` subcommand with propose, list, vote, report subcommands in `src/cli/governance.rs` per FR-104, FR-090
- [ ] T109 [US6] Implement CLI `worldcompute admin` subcommand with halt, resume, ban, audit subcommands (admin-cert required) in `src/cli/admin.rs` per FR-090

### Integration Test

- [ ] T110 [US6] Integration test: submit governance proposal, cast votes from authorized accounts, verify proposal state transitions, verify outcome recorded as GovernanceRecord in ledger in `tests/integration/test_governance.rs`

**Checkpoint**: US6 functional — governance lifecycle works end-to-end

---

## Phase 9: Self-Improvement — Mesh LLM (Principle IV)

**Goal**: Distributed ensemble-of-experts LLM where GPU donors each run a small model; router selects K-of-N per token; mesh self-prompts to improve the cluster

- [ ] T111 [P] Implement router model scaffold: K-of-N expert selection per token generation step with LLaMA-3 tokenizer (128K vocab) in `src/agent/mesh_llm/router.rs` per FR-122, FR-121
- [ ] T112 [P] Implement expert node registration, health tracking, and capacity reporting for mesh LLM participation in `src/agent/mesh_llm/expert.rs` per FR-120
- [ ] T113 Implement sparse logit aggregation: each expert returns top-256 logits (~1.5KB), router computes weighted average, samples next token in `src/agent/mesh_llm/aggregator.rs` per FR-122
- [ ] T114 Implement self-prompting agent loop: mesh generates tasks for itself (scheduler optimization, security log analysis, test generation, config tuning) in `src/agent/mesh_llm/self_prompt.rs` per FR-123
- [ ] T115 Implement agent subsetting: carve off independent parallel agent subsets for concurrent tasks (e.g., scheduling optimization + storage health analysis) in `src/agent/mesh_llm/subset.rs` per FR-124
- [ ] T116 Implement action tier classification (read-only, suggest, modify-minor, modify-major, deploy) with safety sandboxing: modify-major+ requires human governance approval in `src/agent/mesh_llm/safety.rs` per FR-125
- [ ] T117 Implement governance kill switch: immediately halt all mesh LLM operations on governance command, log to ledger in `src/agent/mesh_llm/safety.rs` per FR-125
- [ ] T118 Define MeshLLMService gRPC endpoints (RegisterExpert, GetRouterStatus, SubmitSelfTask, HaltMesh) in `proto/mesh_llm.proto` and implement handlers in `src/agent/mesh_llm/service.rs`
- [ ] T119 Integration test: router selects experts, generates a token via sparse logit aggregation, action logged to ledger in `tests/integration/test_mesh_llm.rs`

**Checkpoint**: Mesh LLM Phase 0–1 (centralized) operational

---

## Phase 10: Desktop GUI

**Goal**: Tauri desktop app providing donor, submitter, governance, and mesh LLM dashboards

- [ ] T120 [P] Initialize Tauri project scaffold: `gui/src-tauri/` Rust backend + `gui/src/` React/TypeScript frontend, configure Tauri dependencies in `gui/src-tauri/Cargo.toml` and `gui/package.json` per FR-091
- [ ] T121 [P] Implement Tauri Rust backend bridge: expose agent IPC commands (donor status, job submit, cluster status, governance) as Tauri commands in `gui/src-tauri/src/main.rs`
- [ ] T122 [P] Implement donor dashboard page: enrollment status, credit balance, trust score, caliber class, active leases, preemption history in `gui/src/pages/DonorDashboard.tsx` per FR-091
- [ ] T123 [P] Implement submitter dashboard page: job submission form, job list, job status, result download, receipt verification in `gui/src/pages/SubmitterDashboard.tsx` per FR-091
- [ ] T124 [P] Implement proposal board and voting UI: list proposals, create proposal, cast vote, view results in `gui/src/pages/GovernanceBoard.tsx` per FR-091
- [ ] T125 [P] Implement mesh LLM status page: expert roster, router health, self-improvement task history in `gui/src/pages/MeshLLMStatus.tsx`
- [ ] T126 Implement settings and consent configuration page: workload class opt-in/out, shard category allowlist, CPU cap, storage cap, OTel endpoint in `gui/src/pages/Settings.tsx` per FR-002, FR-003

**Checkpoint**: Desktop GUI provides feature parity with CLI

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: REST gateway, credit decay, acceptable-use enforcement, web dashboard, security hardening, documentation, and quickstart validation

### Web Dashboard

- [ ] T127 [P] Implement web dashboard React SPA (static CDN-servable) with donor and submitter workflows at parity with CLI in `gui/src/` (shared components) per FR-092
- [ ] T128 [P] Implement REST/HTTP+JSON gateway generation from protobuf schema (grpc-gateway style) for CLI/GUI/third-party integration in `src/network/rest_gateway.rs` per FR-093

### Credit Decay

- [ ] T129 Implement 45-day half-life credit decay with minimum floor protection (`trailing_30d_earn_rate * 30`) as CreditDecay LedgerEntry in `src/credits/decay.rs` per FR-053

### Acceptable Use

- [ ] T130 Implement acceptable-use policy enforcement filter: refuse unauthorized scanning, malware, illegal content, surveillance, credential cracking at job submission in `src/acceptable_use/filter.rs` per FR-080
- [ ] T131 Implement per-donor shard-category allowlist enforcement in data plane placement (separate placement class for residency-constrained shards) in `src/data_plane/placement.rs` per FR-074

### Accessibility & i18n

- [ ] T132 [P] Implement WCAG 2.1 AA accessibility compliance for GUI (keyboard navigation, screen reader support, contrast ratios) in `gui/src/` per FR-095
- [ ] T133 [P] Implement internationalization framework with English + 2 additional launch languages in `gui/src/i18n/` per FR-095

### Security

- [ ] T134 Implement mTLS certificate management: per-account Ed25519 cert issuance, 90-day auto-rotation via ACME-like protocol, admin cert handling in `src/network/tls.rs` per contracts/README.md auth spec
- [ ] T135 [P] Implement rate limiting per contracts/README.md rate limit classes (DONOR_HEARTBEAT 120/min, JOB_SUBMIT 10/min, etc.) in `src/network/rate_limit.rs`
- [ ] T136 Implement reproducible build configuration and code-signing verification (agent refuses dispatch to unattested/unsigned agents) in `src/agent/mod.rs` per FR-005

### Adversarial Tests

- [ ] T137 [P] Adversarial test: sandbox escape attempt (read /etc/passwd from inside sandbox) per quickstart AT-0.1 in `tests/adversarial/test_sandbox_escape.rs`
- [ ] T138 [P] Adversarial test: host-network probe from inside sandbox per quickstart AT-1.2 in `tests/adversarial/test_network_isolation.rs`
- [ ] T139 [P] Adversarial test: wrong-result injection by compromised donor, verify quarantine within 100 audited tasks per quickstart AT-1.4, SC-006 in `tests/adversarial/test_byzantine_donor.rs`
- [ ] T140 [P] Adversarial test: malformed-peer libp2p flood for 60s, verify cluster remains operational per quickstart AT-1.3 in `tests/adversarial/test_flood_resilience.rs`

### Documentation

- [ ] T141 [P] Update repository root README.md with project overview, architecture summary, quickstart instructions, and API reference links
- [ ] T142 [P] Create evidence artifact JSON schema and directory structure (`evidence/phase0/`, `evidence/phase1/`) per quickstart §8

### Quickstart Validation

- [ ] T143 Run quickstart.md Phase 0 single-machine smoke test: install, enroll, 100x trivial workload, verify no host residue, all AT-0.x adversarial tests per quickstart §3
- [ ] T144 Run quickstart.md Phase 1 three-machine LAN cluster test: cluster formation <5s, R=3 job, preemption, node failure recovery, DHT merge, all AT-1.x adversarial tests per quickstart §4–5

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational phase completion
- **US2 (Phase 4)**: Depends on Foundational phase completion; can run in parallel with US1
- **US3 (Phase 5)**: Depends on Foundational phase completion; can run in parallel with US1 and US2
- **US4 (Phase 6)**: Depends on US1 + US2 + US3 completion (needs working cluster)
- **US5 (Phase 7)**: Depends on Foundational only — can run in parallel with US1–US3
- **US6 (Phase 8)**: Depends on Foundational + Ledger (from US2 Phase 4)
- **Mesh LLM (Phase 9)**: Depends on US1 (agent) + US2 (scheduler) + US3 (network)
- **GUI (Phase 10)**: Depends on US1 + US2 + US3 (needs backend functionality to wrap)
- **Polish (Phase 11)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependencies on other stories
- **US2 (P1)**: Can start after Foundational — independently testable; shares ledger types with US1
- **US3 (P1)**: Can start after Foundational — independently testable; uses discovery from US1
- **US4 (P2)**: Needs working cluster (US1+US2+US3) to integrate against
- **US5 (P2)**: Documentation-only — can start after Foundational
- **US6 (P3)**: Needs ledger infrastructure from US2

### Parallel Opportunities

- All proto files (T003–T007) can be written in parallel
- All enum/type definitions (T012–T017) can be written in parallel
- All sandbox drivers (T030–T034) can be written in parallel
- All data-model entity structs (T052–T058) can be written in parallel
- All ledger subcomponents (T065–T067) can be written in parallel
- All adapter implementations (T089, T091, T093) can be written in parallel
- All GUI pages (T122–T126) can be written in parallel
- All adversarial tests (T137–T140) can be written in parallel
- US1, US2, US3, and US5 can proceed in parallel after Foundational phase

---

## Implementation Strategy

### MVP First (US1 + US2 + US3)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: US1 — Donor joins
4. Complete Phase 4: US2 — Submitter runs job
5. Complete Phase 5: US3 — LAN cluster
6. **STOP and VALIDATE**: Run quickstart Phase 0 + Phase 1
7. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. US1 → Donor can join and contribute (MVP increment 1)
3. US2 → Jobs run and return verified results (MVP increment 2)
4. US3 → Zero-config clustering works (MVP increment 3 — full P1 MVP)
5. US4 → HPC/cloud integration (P2)
6. US5 → Funding infrastructure (P2)
7. US6 → Governance tooling (P3)
8. Mesh LLM → Self-improvement (Principle IV)
9. GUI → Desktop app
10. Polish → Production readiness

---

## Notes

- [P] tasks = different files, no dependencies — safe to parallelize
- [US#] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Target 1–3 days per task, touching at most 2–3 files
