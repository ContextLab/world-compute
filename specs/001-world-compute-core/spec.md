# Feature Specification: World Compute — Core Cluster v1

**Feature Branch**: `001-world-compute-core`
**Created**: 2026-04-15
**Status**: Draft
**Input**: Initial architecture and design for a planetary-scale, fully
decentralized, volunteer compute cluster built from anyone who opts in, with
both CLI and GUI surfaces and a complete research/whitepaper/README/API
package.

---

## Overview

World Compute is a SETI@home-style volunteer compute federation that runs as
a background app on any opted-in device and automatically forms clusters
from whatever connected machines it can find — a single LAN, the public
internet, an existing HPC cluster donating idle capacity, an opted-in cloud
tenant, an edge device, even a browser tab. It is governed by the project's
ratified constitution (`.specify/memory/constitution.md`), whose five
principles — Safety First, Robustness, Fairness/Donor Sovereignty,
Efficiency & Self-Improvement, and Direct Testing — are load-bearing for
every requirement below.

This spec defines the v1 scope required to reach a safe, testable,
publishable alpha release. The full research package (7 stages, ~28,600
words) lives under `research/`; proposed design docs under `design/`; the
whitepaper at `whitepaper.md`; and the public-facing README at the repo root.

## Clarifications

### Session 2026-04-15

- Q: Which language should the World Compute agent (not just the CLI) be
  implemented in for v1? → A: Rust everywhere — agent, CLI, and GUI
  backend all Rust, using `rust-libp2p`, one toolchain, reproducible
  builds, memory-safe by default (Principle I).
- Q: Which open-source license should World Compute ship under? → A:
  Apache 2.0 — permissive with explicit patent grant, ecosystem-standard
  (Firecracker/Kata/Kubernetes/rust-libp2p/Sigstore/Wasmtime all use it),
  501(c)(3)-compatible, integrator-friendly for HPC and cloud adopters.
- Q: Which legal home for the project organization at founding? → A: US
  501(c)(3) public charity (Delaware, ISRG/Let's Encrypt model) — large
  US-taxpayer donor pool, foundation-grant-friendly, well-understood
  governance vehicle. Project MUST comply with US export controls and
  OFAC sanctions; a future EU subsidiary remains an option if EU-donor
  fiscal sponsorship becomes material.
- Q: How should per-donor storage residency constraints be modeled? → A:
  Per-donor shard-category allowlist — donors declare at enrollment
  which shard categories they will host (e.g., `public`,
  `opaque-encrypted`, `eu-resident`, `us-resident`); the scheduler
  respects the declaration absolutely (Principle III); residency-
  constrained shards live in a separate placement class with its own
  erasure-code parameters so the main pool's RS(10,18) dispersal
  guarantee is unaffected.
- Q: What is the minimum mandatory telemetry every production component
  must emit in v1? → A: Structured logs + metrics + traces (full
  OpenTelemetry trifecta) with donor-privacy redaction enforced at the
  emit layer. Telemetry MUST NOT leak donor PII, submitter job
  contents, or host-identifying information.
- Q: Should NCU gate job submission (requiring donation before use)?
  → A: NO — NCU boosts priority but never gates access. Anyone can
  submit jobs for free. Multi-factor priority score (FR-032) with
  public human-verified voting, job size, queue age, and user cooldown.
  Starvation-freedom guaranteed: S_age ensures no job waits forever.
- Q: What is the concrete self-improvement mechanism (Principle IV)?
  → A: Distributed ensemble-of-experts mesh LLM (FR-120–126). Each
  GPU donor runs a complete small model; router selects K-of-N per
  token; mesh self-prompts to improve the cluster. Phased rollout
  from centralized (Phase 0) to full autonomous (Phase 4, ~5000+
  nodes). LLaMA-3 tokenizer standardized.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Donor Joins and Contributes Idle Compute (Priority: P1)

A volunteer downloads, installs, and runs the World Compute background
agent on their personal laptop. With one click / one command, they opt in
to donate idle resources. The agent joins the nearest cluster (LAN first,
internet fallback), begins receiving and executing sandboxed work units
when the machine is idle, and instantly yields all resources the moment
the human user returns. The donor sees their contribution accruing credits
they can redeem for cluster compute later, and can pause or withdraw at
any time with no residue on their machine.

**Why this priority**: Without donors, there is no cluster. This is the
primary value loop and also the riskiest surface (Principle I). It must
ship first.

**Independent Test**: Install the agent on a single laptop; enroll it via
CLI and/or GUI; submit a trivial test job from a second machine on the
same LAN; verify (a) the job runs, (b) the result is correct, (c) the
donor can see credits earned, (d) touching the keyboard or mouse on the
donor machine suspends the workload within one second, (e) stopping the
agent leaves no files, no processes, and no network state on the donor.

**Acceptance Scenarios**:

1. **Given** a fresh machine with no prior World Compute state, **When** the
   user installs the agent and runs `worldcompute donor join`, **Then** the
   agent establishes a peer identity, auto-discovers any LAN peers via
   mDNS within 2 seconds, joins or creates a cluster, and begins
   advertising its capacity to the scheduler.
2. **Given** an enrolled donor with the agent running idle, **When** a job
   is scheduled to their machine, **Then** it executes inside a hardened
   VM/microVM sandbox (Tier 1 Firecracker on Linux, Apple VF on macOS,
   Hyper-V/WSL2 on Windows) with no access to the host filesystem,
   credentials, network identity, camera, mic, clipboard, or peripherals.
3. **Given** a job running on a donor machine, **When** the local user
   touches the keyboard, moves the mouse, launches a foreground app,
   unplugs AC power, or crosses a thermal threshold, **Then** all
   cluster workloads are SIGSTOP-frozen within 10 ms and fully
   checkpoint-and-release resources within 500 ms (sub-second yield per
   Principle III).
4. **Given** a donor who stops or uninstalls the agent, **When** they
   reboot, **Then** no World Compute files, processes, scheduled tasks,
   startup hooks, network routes, or host-level configuration remain.
5. **Given** a completed job, **When** the donor inspects their account,
   **Then** they see an auditable, tamper-evident, credit ledger entry
   showing the NCU (Normalized Compute Units) earned, the job class, and
   the verification outcome.

---

### User Story 2 - Submitter Runs a Job and Gets a Correct Result (Priority: P1)

A scientist, student, philanthropist, or ordinary donor redeeming their
earned credits submits a compute job (CLI: `worldcompute submit job.yaml`
or web dashboard upload). The system stages the inputs, schedules
replicated execution on eligible donor nodes, verifies the result, and
returns it with a cryptographic proof of correctness. Submitters with no
donated hardware can still submit public-good jobs but those jobs run only
on excess capacity and are preempted by donor-redemption jobs.

**Why this priority**: Without submitted work, donors have nothing to run
and the cluster has no reason to exist. P1 alongside donor onboarding.

**Independent Test**: Write a hello-world job manifest that computes
something whose correct answer is known (e.g., SHA-256 of a known input,
or a small matrix multiplication). Submit it; verify the returned result
matches the expected answer; verify the result carries a signed
verification artifact; verify a deliberately wrong job (submitted by a
simulated malicious donor in the testnet) is detected and rejected by the
quorum / audit layer.

**Acceptance Scenarios**:

1. **Given** a valid job manifest (OCI image OR WASM module, command,
   inputs, output sink, priority class, replica count defaults), **When**
   the submitter runs `worldcompute submit ./job.yaml`, **Then** the job
   is assigned a CID, staged to the content-addressable data plane, and
   enqueued to the scheduler.
2. **Given** a submitted task, **When** the scheduler dispatches it,
   **Then** at least R=3 replicas execute on independently-chosen donor
   nodes (disjoint by trust bucket and network autonomous system) and a
   canonical-hash quorum decides the accepted result.
3. **Given** a task whose result is accepted by quorum, **When** the
   system records the outcome, **Then** it is recorded in the
   append-only, Merkle-chained, threshold-signed credit & provenance
   ledger with a verifiable witness for both correctness and accounting.
4. **Given** a submitter with confidentiality needs, **When** they submit
   a job marked `confidential`, **Then** the scheduler restricts
   execution to Trust Tier T3+ nodes (SEV-SNP, TDX, or H100 Confidential
   Compute) with hardware-attested guest measurement.
5. **Given** a submitter with no earned credits submitting a `public-good`
   job, **When** the scheduler places it, **Then** it receives a lower
   composite priority score (S_ncu = 0, relying on S_vote, S_size,
   S_age, S_cool) per FR-032. The job is NEVER permanently blocked —
   S_age ensures it reaches the top of any finite queue within ~7
   hours — but higher-scoring jobs (e.g., donor-redemption with NCU)
   are scheduled first.

---

### User Story 3 - Zero-Config Cluster Forms on an Isolated LAN (Priority: P1)

Two or three machines on the same local network install the agent and run
it. With no internet, no central registry, no configuration files, no
admin intervention, they discover each other, elect coordinators, form a
functioning cluster, and can accept and run jobs. When the network later
gains internet access, the LAN cluster transparently merges into the
global cluster without losing in-flight work or credit history.

**Why this priority**: This is the "fully decentralized" constitutional
commitment. If a three-machine LAN can't form a cluster, nothing about the
rest of the design is believable. P1.

**Independent Test**: Air-gap 3 real machines on a switch with no
internet. Install the agent on all 3. Observe that within 5 seconds they
have a functioning cluster (verifiable via `worldcompute cluster status`).
Submit and run a test job against the LAN-only cluster. Re-enable
internet on the gateway. Verify the LAN merges with the global network
without duplicating credits or dropping jobs.

**Acceptance Scenarios**:

1. **Given** N≥2 machines on a LAN with the agent installed but no prior
   peer knowledge, **When** the agents start, **Then** mDNS discovery
   finds all peers within 2 seconds and they form a Kademlia DHT island.
2. **Given** a LAN-only cluster of 3 machines, **When** a job is submitted
   to one of them, **Then** it is accepted, scheduled, replicated across
   the LAN peers, and returns a correct result.
3. **Given** a LAN cluster that later gains internet connectivity,
   **When** any node successfully reaches a DNS bootstrap seed, **Then**
   the LAN DHT merges with the global DHT without introducing
   inconsistency in the credit ledger or job records.
4. **Given** a node behind a symmetric NAT that cannot hole-punch, **When**
   the agent starts, **Then** it automatically establishes connectivity
   via libp2p Circuit Relay v2 or a community relay, with no user action.

---

### User Story 4 - Integrator Connects an Existing Cluster / Cloud Tenant (Priority: P2)

An HPC center (Slurm), a Kubernetes cluster, or a cloud tenant (AWS,
GCP, Azure) wants to contribute idle or dedicated capacity to World
Compute. An operator installs a single adapter component that exposes the
local cluster as an aggregate node to World Compute, respecting local
resource policies, quotas, and priorities.

**Why this priority**: P2 — massively accelerates growth, but the core
P2P donor path must exist first.

**Independent Test**: On a Slurm testbed (can be 2 nodes), install the
World Compute Slurm adapter. Submit a job to World Compute that is
dispatched to the Slurm bridge. Verify the pilot job runs under Slurm's
normal scheduler, returns correct results, respects local Slurm priority
preemption, and reports capacity accurately to World Compute. Repeat for
Kubernetes via the operator.

**Acceptance Scenarios**:

1. **Given** a functioning Slurm cluster, **When** the operator installs
   the Slurm adapter and provides cluster credentials, **Then** the
   adapter appears as a single aggregate node in World Compute with
   capacity equal to the opt-in subset of Slurm resources.
2. **Given** a Kubernetes cluster with the World Compute operator
   installed, **When** a CRD `ClusterDonation` is applied, **Then** jobs
   are dispatched as standard K8s Pods honoring namespace resource limits.
3. **Given** a cloud tenant running the agent on multiple instances,
   **When** the instances start, **Then** they attest via the cloud's
   instance metadata service and join World Compute as first-class donors.

---

### User Story 5 - Philanthropist / Financial Supporter Contributes Funds (Priority: P2)

A financial supporter — individual, foundation, corporation — wants to
fund the project (distinct from a "hardware donor" who contributes
compute). They contribute through a transparent, publicly-auditable
channel. Funds are managed by a named legal entity (US 501(c)(3) at
launch) and spent under a published, quarterly-reported budget aligned to
the constitution. Financial donation never buys compute priority (that
would violate Principle III); corporate sponsorship tiers are charitable,
not transactional.

**Why this priority**: P2 — the project needs money to run audits, buy
test hardware, and pay core developers, but the technical system can be
bootstrapped by volunteers before the legal entity is fully stood up.

**Independent Test**: Verify that the project's public website lists a
legal entity, accepts donations via an open platform (Open Collective or
equivalent), publishes a quarterly financial report, and documents that
no financial donation confers compute scheduling priority.

**Acceptance Scenarios**:

1. **Given** the project is incorporated, **When** a financial supporter
   visits the public "Support" page, **Then** they see a legal entity name, tax
   status, donation channels, a public ledger of funds, and the
   governance model.
2. **Given** a corporate sponsor making a tier donation, **When** the
   sponsor attempts to request compute priority, **Then** the project
   declines and the bylaws document the refusal mechanism.
3. **Given** a quarterly reporting period closes, **When** the governance
   body publishes the report, **Then** it shows inflows, outflows by
   category, audit status, and incident disclosures.

---

### User Story 6 - Governance Member Proposes and Votes on a Policy Change (Priority: P3)

A member of the Technical Steering Committee (TSC) or the Board proposes
an amendment to cluster policy — e.g., a new acceptable-use rule, a
priority-class rebalance, an emergency halt. The proposal is published,
discussed, voted on per the published rules, and the outcome is recorded
on the same tamper-evident ledger that records compute and credits.

**Why this priority**: P3 — governance tooling is needed for long-term
health but can follow the initial technical launch.

**Independent Test**: Submit a governance proposal via the CLI or web
dashboard; verify it appears in the public proposal list; cast votes
from authorized accounts; verify the vote record and outcome appear on
the ledger with a verifiable witness.

---

### Edge Cases

- **Mid-job hardware failure**: a node running a replica dies. The
  scheduler detects missed heartbeats within 1 heartbeat interval (target
  5–15 s), reschedules from the latest committed checkpoint onto an
  alternative node in the same trust/placement bucket, and the job
  continues without submitter intervention.
- **Concurrent local-user return**: a donor's machine is mid-job when
  the user unlocks their laptop. The agent freezes (SIGSTOP) the workload
  within 10 ms, attempts a 500 ms checkpoint, then fully releases all
  resources. A pre-warmed replica elsewhere continues the job.
- **Network partition**: a sub-cluster is cut off from the global network
  for hours. It continues to operate as a LAN cluster (jobs may run,
  credits accrue in the local CRDT replica). On re-merge, the CRDT
  reconciles and no committed credits are lost. If the partition lasted
  so long that another partition re-ran the same task, the quorum rules
  accept the earliest-threshold-witnessed result and refund any
  second-place replicas per the replica-co-vote protection rule.
- **Byzantine donor flood**: an attacker donates 1,000 colluding VMs
  across 50 /24 subnets. Libp2p /16 IP-diversity caps, 7-day Trust Score
  floor, disjoint-AS replica placement, and randomized 3% audit
  re-execution defeat the attack with high probability within hours at
  ~3% defender overhead.
- **Sandbox escape / P0 incident**: a CVE in Firecracker is disclosed.
  The control plane remotely disables affected agent versions within
  one release cycle. New job dispatch to affected sandboxes halts
  cluster-wide until a fix is verified by direct test (Principle I + V).
- **Credit hoarding / inflation**: credits decay with a 45-day half-life
  and outstanding supply is monitored against trailing redemption
  demand; decay rate is adjusted if supply exceeds 110% of demand.
- **Donor withdrawal with unspent credits**: unspent credits remain
  redeemable for a configurable window (default 180 days) after
  withdrawal; beyond that they return to the public-good pool.
- **Mobile / browser donor churn**: browser and mobile donors are Trust
  Tier T0, capped to workloads with low checkpoint cost and high
  replication.
- **GPU kernel preemption**: in-flight CUDA kernels do not honor SIGSTOP
  instantly. v1 GPU donors run shorter kernel windows (target ≤200 ms)
  and the scheduler targets GPU jobs only at GPU-certified donors who
  have passed a real-hardware preemption-latency test.
- **Coordinator compromise**: threshold-signed witnesses on every ledger
  record, published Merkle roots to an external transparency log
  (Sigstore Rekor or equivalent), and a public mirror ensure that a
  single compromised coordinator cannot rewrite history undetected.

---

## Requirements *(mandatory)*

### Functional Requirements

#### Donor lifecycle

- **FR-001**: The system MUST provide a background agent for Linux, macOS,
  and Windows that can run unattended and start on user command (not
  silently — opt-in is required per Principle I).
- **FR-002**: The agent MUST support enrollment, status, pause/resume,
  configuration, and withdrawal via both CLI (`worldcompute donor ...`)
  and a local GUI (desktop + web dashboard).
- **FR-003**: The agent MUST allow the donor to grant granular, revocable
  consent per workload class (e.g., scientific, public-good ML, indexing,
  self-improvement) and MUST refuse any class the donor has not
  explicitly opted into.
- **FR-004**: The agent MUST leave no persistent host-level state beyond
  an explicitly scoped, size-capped working directory that is wiped on
  withdrawal or agent exit.
- **FR-005**: The agent MUST be reproducibly buildable and code-signed;
  the control plane MUST refuse to dispatch jobs to unattested or
  unsigned agents.
- **FR-006**: The agent, CLI, and GUI backend MUST be implemented in Rust
  as a single unified codebase. `rust-libp2p` is the P2P stack. This
  commitment is load-bearing for Principle I (memory-safety of the
  host-resident security-critical binary), for Principle IV (lowest
  runtime footprint and no GC pauses that would jeopardize the
  sub-second preemption budget), and for having one reproducible-build
  pipeline and one audit surface for the entire privileged host
  component.

#### Sandboxing & host integrity (Principle I)

- **FR-010**: All workloads MUST execute inside a hypervisor- or VM-level
  sandbox (Firecracker on Linux, Apple Virtualization.framework on macOS,
  Hyper-V on Windows Pro, WSL2 utility VM on Windows Home with WHPX
  fallback). Process-only sandboxes (namespaces, seccomp, gVisor alone)
  are NOT sufficient and MUST NOT be used for production workloads.
- **FR-011**: Workloads MUST have no access to the host filesystem,
  credentials, network state, LAN peers, peripherals (camera, mic,
  clipboard, GPS), or user processes.
- **FR-012**: GPU passthrough MUST verify singleton IOMMU group before
  exposing the GPU to a guest; the ACS-override patch is prohibited.
- **FR-013**: The control plane MUST perform cryptographic attestation
  (TPM 2.0 PCR on x86; SEV-SNP/TDX/SGX where present; macOS Secure
  Enclave signing; soft attestation for WASM donors) before dispatching
  any job.
- **FR-014**: Any discovered sandbox escape or host-data exfiltration
  MUST trigger a cluster-wide halt of new dispatches to affected agent
  versions within one release cycle (P0).

#### Job model & execution

- **FR-020**: A job manifest MUST specify: workload artifact (OCI image
  CID or WASM module CID), command/entrypoint, inputs (CIDs), output
  sink, resource requirements (CPU, memory, storage, optional
  GPU/tier), priority class, replica count (default R=3), max wallclock,
  confidentiality level, and acceptable-use category.
- **FR-021**: Both OCI containers AND WASM modules MUST be first-class
  workload formats.
- **FR-022**: The task → workflow → job hierarchy MUST be supported:
  atomic tasks (unit of scheduling), workflows (DAG of tasks with
  dependencies), jobs (instantiation of a workflow).
- **FR-023**: Tasks MUST checkpoint to the content-addressable data
  plane at least every 60 seconds for long-running jobs, so that any
  replica can be resumed on any other eligible node from the latest
  committed checkpoint.
- **FR-024**: Default verification MUST use R=3 replicated execution with
  canonical-hash quorum. Trust Tier T3 (SEV-SNP/TDX) MAY collapse to R=1.
  Trust Tier T0 (browser/WASM) MUST use R≥5 and be limited to
  public-data workloads.
- **FR-025**: 3% of accepted results MUST be randomly re-audited on
  independent high-trust nodes to detect quorum collusion.
- **FR-026**: The system MUST support batch, embarrassingly parallel, and
  DAG workloads in v1. MPI-like tightly-coupled workloads and
  long-running services are OUT of scope for v1.

#### Robustness & scheduling

- **FR-030**: The scheduler MUST tolerate continuous node churn without
  loss of accepted results: any replica lost in flight MUST be
  rescheduled from the latest committed checkpoint within one heartbeat
  interval.
- **FR-031**: The scheduler architecture MUST be hierarchical and
  decentralized: (a) a sharded-Raft global system of record, (b)
  regional libp2p gossip brokers using ClassAd-style matchmaking, (c)
  fully-autonomous local agents. No tier may be on the critical path of
  another tier's hard guarantees.
- **FR-032**: The scheduler MUST use a continuous multi-factor priority
  score for all jobs (LOCAL_USER preemption remains an absolute
  override outside this formula). The composite score is:
  `P(job) = 0.35·S_ncu + 0.25·S_vote + 0.15·S_size + 0.15·S_age + 0.10·S_cool`
  where all signals are normalized to [0,1]:
  - **S_ncu**: `1 - exp(-α·balance)` where α is tuned so that the
    median donor balance yields S_ncu ≈ 0.7 (donors get priority
    boost, not access gating — NCU is never required to submit a job;
    see research/08-priority-redesign.md for derivation)
  - **S_vote**: population-normalized public importance vote score
    from verified human voters (see FR-055–058)
  - **S_size**: exponential decay penalizing larger/longer jobs
    (Slurm-style backfill — small jobs fill gaps)
  - **S_age**: exponential saturation with a 4-hour half-life
    ensuring every job reaches the top of any finite queue
    (starvation freedom guarantee)
  - **S_cool**: exponential decay over a 24-hour trailing window
    penalizing users who recently consumed cluster compute
    (prevents monopolization)
  Anyone on Earth can submit a job for free. NCU boosts priority but
  NEVER gates access. No job is ever permanently blocked.
- **FR-033**: A reserved slice of total cluster capacity (5–10%) MUST be
  allocated at all times to self-improvement workloads (Principle IV),
  concretely: the distributed mesh LLM system (see FR-120–126). This
  slice MUST NOT starve under load and MUST NOT be monopolizable.
- **FR-034**: The system MUST enforce disjoint-bucket placement of
  replicas: different autonomous systems, different geographic regions,
  different trust buckets.

#### Preemption & donor sovereignty (Principle III)

- **FR-040**: The local agent MUST freeze (SIGSTOP) all cluster
  workloads within 10 ms of any of: keyboard/mouse activity, foreground
  app launch, AC-power disconnect, thermal threshold, memory pressure,
  user-defined trigger. Full resource release within 500 ms.
- **FR-041**: When a replica is preempted, the scheduler MUST maintain
  job liveness by promoting a pre-warmed hot standby or launching a new
  replica from the latest checkpoint without submitter action.
- **FR-042**: Donated hardware earns NCU credits that boost the donor's
  job priority via S_ncu (FR-032). Donors with NCU balance > 0 MUST
  receive a priority boost proportional to their balance. When a donor
  redeems NCU for their own job, the scheduler MUST guarantee a
  MINIMUM allocation of resources of at least the same caliber class
  (0=RPi, 1=CPU laptop, 2=workstation, 3=server, 4=high-end GPU) as
  was donated, averaged over the accounting window, per Constitution
  Principle III. If no same-caliber node is available within the p95
  SLA (SC-007), the donor MAY accept a lower-tier node voluntarily
  with a 30% NCU refund, but MUST NOT be silently downgraded. NCU is
  a priority accelerator, NOT an access gate — donors are never worse
  off than non-donors, and non-donors are never fully blocked.

#### Credit, accounting, and trust

- **FR-050**: The compute credit unit MUST be the Normalized Compute
  Unit (NCU): 1 TFLOP/s FP32-second on a reference platform, normalized
  multidimensionally (compute, memory, storage, network) with DRF
  dominant-dimension accounting.
- **FR-051**: Credits MUST be recorded in a tamper-evident,
  threshold-signed, Merkle-chained, CRDT-replicated append-only ledger
  (NOT a blockchain). Merkle roots MUST be anchored to an external
  transparency log (Sigstore Rekor or equivalent) every 10 minutes.
- **FR-052**: The Trust Score for a donor node MUST be computed as a
  bounded function of result consistency, attestation level, age, and
  recent-failure penalty; capped at 0.5 for the first 7 days, ramping
  to 1.0 after 30 days of consistent quorum agreement.
- **FR-053**: Credits MUST decay with a 45-day half-life to prevent
  hoarding; a minimum floor protects donors on break.
- **FR-054**: Donors MUST be able to inspect and cryptographically
  verify their own credit history locally with `worldcompute donor
  credits --verify`.

#### Public voting and Sybil resistance

- **FR-055**: Any human MUST be able to submit a compute proposal
  ("I want to run X, here's why it matters") to the public proposal
  board without holding NCU credits.
- **FR-056**: Verified humans MUST be able to upvote or downvote
  proposals on the public board. Vote weight is proportional to the
  voter's Humanity Points (HP) score. Voters MUST hold HP >= 5 for
  full vote weight; below that, fractional weight (HP/5).
- **FR-057**: Humanity Points MUST be earned through a layered
  composite score for Sybil resistance: Tier 1 (low friction) — email
  verification (1 HP), phone number (3 HP), linked social accounts
  with public activity history verifiable via OAuth2 (GitHub, LinkedIn,
  or X/Twitter — 2 HP each, max 3). Tier 2 (higher friction) — web-of-trust vouching
  from existing verified voters (2 HP per vouch, max 3), proof-of-
  personhood protocol participation (e.g., BrightID, Idena — 3 HP
  each). Tier 3 (highest trust) — active World Compute donor with
  Trust Score > 0.7 (5 HP), because proof-of-hardware costs 3–4
  orders of magnitude more to fake than digital identities. Biometric
  collection (e.g., iris scans) is NOT permitted per Principle I.
- **FR-058**: The voting system MUST implement anti-gaming measures:
  quadratic voting (vote cost scales as n²), per-epoch vote budgets
  (20 votes per 7-day epoch), anomaly detection for coordinated sock-
  puppet campaigns, and a public audit log of all votes on the
  tamper-evident ledger.
- **FR-059**: Self-voting exclusion: a proposal submitter MUST NOT be
  able to vote on their own proposal; donors MUST NOT be able to
  vote-boost proposals that exclusively benefit their own jobs.

#### Decentralized bootstrap & discovery

- **FR-060**: On a LAN with no internet and no central registry, N≥2
  fresh agents MUST discover each other and form a working cluster
  within 5 seconds via mDNS.
- **FR-061**: On the open internet, a fresh agent with no prior peers
  MUST bootstrap via DNS seeds and Kademlia DHT self-organization.
- **FR-062**: The agent MUST perform NAT traversal via UPnP-IGD/NAT-PMP
  first, then libp2p DCUtR hole punching, then Circuit Relay v2 as a
  final fallback, with no user action required.
- **FR-063**: A LAN cluster that later gains internet MUST merge with
  the global DHT without data loss or credit inconsistency.
- **FR-064**: Adapters MUST exist for Slurm/PBS (pilot-job gateway),
  Kubernetes (operator + CRD), and cloud providers (instance-metadata
  attestation). Mobile and browser adapters may be deferred to later
  phases but the data and control plane MUST NOT preclude them.

#### Storage / data plane

- **FR-070**: All workload artifacts, inputs, outputs, and checkpoints
  MUST be content-addressed with CIDv1 (SHA-256).
- **FR-071**: Cold data MUST be stored with Reed-Solomon RS(10,18)
  erasure coding, with shard placement enforcing ≥3 continents, ≤2
  shards per country, ≥1 shard per autonomous system.
- **FR-072**: Submitter data and code MUST be protected from snooping
  donors by one of: plaintext (public jobs), encrypted bundle with
  TPM-agent-attested key release (confidential-medium), or SEV-SNP/TDX/
  H100-CC guest-measurement key wrapping (confidential-high).
- **FR-073**: The data plane MUST enforce a configurable per-donor
  storage cap and MUST implement garbage collection for expired or
  withdrawn content.
- **FR-074**: The data plane MUST support a per-donor shard-category
  allowlist, declared at enrollment and revocable at any time. Donors
  MUST be able to restrict which shard categories (at minimum:
  `public`, `opaque-encrypted`, and per-jurisdiction categories such as
  `eu-resident`, `us-resident`, `uk-resident`, `jp-resident`) they will
  host, and the scheduler MUST respect the declaration absolutely.
  Residency-constrained shards MUST live in a separate placement class
  with its own erasure-code parameters chosen to maintain durability
  within the smaller constrained pool, so that the main pool's
  RS(10,18) geographic-dispersal guarantee (FR-071) remains intact.
  Donors with full legal prohibition on non-resident data hosting MUST
  be able to opt out of the main pool entirely and still donate compute
  and host only residency-matched shards.

#### Acceptable use & safety

- **FR-080**: The system MUST refuse jobs categorized as: unauthorized
  network scanning, malware distribution, illegal content, targeted
  surveillance, credential cracking against third parties. Enforcement
  is a first-class concern, not an afterthought.
- **FR-081**: Donors MUST be able to opt out per category; the scheduler
  MUST respect donor opt-outs absolutely.
- **FR-082**: Security incident disclosure MUST occur within a pre-
  committed timeframe (proposed: 72 hours after mitigation, 30 days
  after detection if mitigation is delayed).

#### CLI & GUI (reach)

- **FR-090**: A single CLI binary (`worldcompute`) MUST provide donor,
  submitter, and (for authorized accounts) admin/governance subcommands.
  The CLI MUST be implemented in Rust with `clap`, statically linked,
  and reproducibly built, sharing the agent's codebase per FR-006.
- **FR-091**: A desktop GUI MUST exist for Linux/macOS/Windows using
  OS-native WebView (Tauri) to minimize installer size and attack
  surface; Electron is REJECTED for Principle I reasons.
- **FR-092**: A web dashboard (React SPA served from a static CDN) MUST
  provide donor and submitter workflows at parity with the CLI.
- **FR-093**: The public API MUST be gRPC-primary with a REST/HTTP+JSON
  gateway generated from the same protobuf schema, so the CLI, GUI, and
  third-party integrations share one contract.
- **FR-094**: Mobile and browser-donor modes MAY be implemented in
  Phase 3 once their sandbox stories are independently audited.
- **FR-095**: The user-facing surface MUST be accessible (WCAG 2.1 AA)
  and internationalized from v1; English plus at least 2 additional
  launch languages (initial targets: Spanish and Simplified Chinese,
  subject to governance revision).

#### Governance & funding

- **FR-099**: All source code, specifications, research, and
  documentation MUST be released under the Apache License 2.0, with the
  explicit patent grant intact. Third-party dependencies MUST be
  license-compatible with Apache 2.0; AGPL and other strong-copyleft
  dependencies are NOT permitted in the agent, CLI, GUI, broker, or
  coordinator code paths.
- **FR-100**: The project MUST be incorporated as a US 501(c)(3) public
  charity (Delaware, modeled on the Internet Security Research Group /
  Let's Encrypt) prior to the public GA release. The entity MUST comply
  with US export controls (EAR) and OFAC sanctions programs; a future
  EU subsidiary (e.g., Dutch Stichting) MAY be added if EU-donor fiscal
  sponsorship becomes material.
- **FR-101**: The project MUST publish a public ledger of financial
  inflows and outflows and a quarterly financial report.
- **FR-102**: Governance MUST be two-body: a Technical Steering
  Committee for technical decisions and a Board of Directors for
  financial/legal decisions. No company may hold >2 seats on either body.
- **FR-103**: Financial donations MUST NOT confer compute scheduling
  priority, and the bylaws MUST document the refusal mechanism.
- **FR-104**: A governance proposal/vote system MUST exist via CLI and
  web dashboard, with records written to the same ledger as compute
  provenance.

#### Distributed mesh LLM (self-improvement, Principle IV)

- **FR-120**: The self-improvement system MUST be a distributed
  ensemble-of-experts LLM ("mesh LLM") where each participating GPU
  donor node runs a complete small language model (e.g., LLaMA-3-8B
  quantized to 4-bit) as one "expert" in a Mixture-of-Experts
  architecture.
- **FR-121**: All participating models MUST use the same tokenizer
  (LLaMA-3 tokenizer, 128K vocabulary). This is the universal
  interface — models with incompatible tokenizers MUST NOT participate
  in the mesh until a cross-tokenizer vocabulary mapping is validated.
- **FR-122**: A lightweight router model MUST select K-of-N experts
  per token generation step. Each selected expert returns sparse
  top-256 logits (~1.5 KB). The router computes the weighted average
  of these distributions and samples the next token. This requires
  one parallel network round-trip per token (not N sequential hops).
- **FR-123**: The mesh LLM MUST be able to self-prompt — generating
  tasks for itself, evaluating outputs, and using results to improve
  the cluster (scheduler optimization, security log analysis, test
  generation, configuration tuning, governance proposal drafting).
- **FR-124**: The mesh LLM MUST be able to carve off subsets of itself
  to work as independent parallel agents — e.g., one subset optimizes
  scheduling while another analyzes storage health.
- **FR-125**: Safety: all mesh LLM outputs that modify cluster
  configuration, policy, or code MUST be classified into action tiers
  (read-only, suggest, modify-minor, modify-major, deploy) and MUST
  be sandboxed. Tier "modify-major" and above MUST require human
  governance approval before deployment. All actions MUST be logged
  to the tamper-evident ledger. A governance kill switch MUST be able
  to halt all mesh LLM operations immediately.
- **FR-126**: Phased rollout: Phase 0–1 use a centralized model on
  project-operated infrastructure. Phase 2 (~280+ GPU nodes at 5%
  capacity) enables a minimal distributed ensemble with human-reviewed
  outputs only. Phase 3 (~1000+ nodes) adds learned routing and
  sandboxed self-modification. Phase 4 (~5000+ nodes) enables full
  autonomous self-improvement with governance-gated deployment.

#### Observability

- **FR-105**: Every production component (agent, sandbox driver,
  preemption supervisor, local scheduler, broker, coordinator, data
  plane, ledger, adapters, CLI, GUI) MUST emit OpenTelemetry-compatible
  structured logs, metrics, and traces. A component that does not
  emit all three categories MUST NOT be deployed to production.
- **FR-106**: Telemetry emission MUST enforce donor-privacy redaction
  at the emit layer: no donor personally-identifying information, no
  submitter job contents or inputs/outputs, and no host-identifying
  information (hostnames, local IPs, usernames, MAC addresses) may
  appear in any telemetry stream. Redaction MUST be unit-tested as a
  gating requirement for releases.
- **FR-107**: Telemetry MUST be sufficient to investigate Principle I
  (sandbox isolation), Principle II (churn recovery), Principle III
  (preemption latency and donor-redemption SLAs), and Principle V
  (direct-test evidence capture) incidents after the fact, per the
  constitutional prohibition on "we don't know what happened"
  post-mortem conclusions.

#### Direct testing (Principle V)

- **FR-110**: No component MAY be deployed to production without a
  direct-test evidence artifact showing real-hardware execution,
  inputs, expected outputs, observed outputs, and pass/fail.
- **FR-111**: Safety-critical paths (sandbox isolation, preemption
  latency, data durability, attestation, acceptable-use filters) MUST
  be tested with adversarial cases on every release.
- **FR-112**: The staged-release plan (Phase 0 single laptop → Phase 1
  3–5 LAN → Phase 2 20–50 federated → Phase 3 500–5000 alpha → Phase 4
  GA) MUST have published pass/kill gates; failing a gate blocks
  promotion to the next phase.

### Key Entities

- **Agent**: The per-host background program that hosts peer identity,
  runs sandboxes, enforces donor sovereignty, and communicates with
  coordinators and peers.
- **Donor**: A person (or organizational operator) who opts in to run
  the agent on one or more machines. Owns a peer identity and a credit
  balance.
- **Node**: A logical instance of the agent on a single machine. Has a
  trust tier, caliber class, capability fingerprint, and current
  Trust Score.
- **Submitter**: A person or account who submits jobs. May or may not
  also be a donor. Has an account balance (NCU credits) and an
  acceptable-use history.
- **Task**: The atomic unit of scheduling. Specified by CIDs for
  workload + inputs, command, and resource requirements.
- **Workflow**: A DAG of tasks with dependencies, expressed as a
  manifest.
- **Job**: An instantiation of a workflow with priority class,
  confidentiality level, and replica policy.
- **Replica**: One execution instance of a task; R≥1 replicas run per
  task and a quorum decides the accepted result.
- **Work Unit Receipt**: The signed, ledger-recorded artifact proving a
  task's execution, verification, and credit allocation.
- **Credit (NCU)**: The normalized compute-unit token tracked in the
  ledger.
- **Cluster**: Any self-organized set of mutually-discovered agents.
  Clusters are fractal — a LAN cluster is also a sub-cluster of the
  global cluster once connected.
- **Coordinator**: An elected, operator-hardened node that hosts a shard
  of the global ledger and participates in threshold signing. Small
  number (target ~100–1000) and not ordinary donor hardware.
- **Broker**: A regional matchmaker that runs libp2p gossip and
  ClassAd-style matching between tasks and nodes.
- **Trust Tier**: Classification of a node's maximum allowable workload
  sensitivity (T0 browser/WASM, T1 TPM-attested CPU VM, T2 TPM + GPU,
  T3 SEV-SNP/TDX, T4 H100 Confidential Compute).
- **Caliber Class**: Classification of a node's hardware performance
  tier (0 RPi, 1 laptop, 2 workstation, 3 server, 4 high-end GPU),
  used to enforce the "same caliber" donor-redemption guarantee.
- **Ledger**: The append-only, Merkle-chained, threshold-signed,
  CRDT-replicated record of jobs, results, credits, and governance
  actions. NOT a blockchain.
- **Governance Proposal**: A structured change request (policy,
  acceptable-use, priority class, emergency halt) voted on by the TSC
  or Board and recorded to the ledger.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A fresh donor can install the agent and begin earning
  credits (via a correctly-executed test job) in under 5 minutes from
  download on a representative consumer laptop.
- **SC-002**: On a donor machine actively running a cluster workload,
  the local human user experiences no perceptible slowdown (keystroke
  latency, frame drop, or app responsiveness) when they resume activity;
  full interactive-resource yield occurs in under 1 second 99% of the
  time.
- **SC-003**: Two to three freshly installed agents on an isolated LAN
  with no internet form a working cluster (capable of accepting and
  completing a test job) in under 5 seconds from agent start.
- **SC-004**: In the Phase 2 federated testnet, 80% of submitted test
  jobs complete correctly over a 72-hour window with 30% simulated node
  churn.
- **SC-005**: In the Phase 3 public alpha, 90% of submitted jobs
  complete correctly over a 30-day window, and 0 real-world Principle I
  (sandbox escape or host-data exfiltration) incidents occur.
- **SC-006**: A deliberately malicious donor injecting wrong results is
  detected and quarantined within 100 audited tasks with ≥95%
  probability.
- **SC-007**: A job submitted with NCU balance > 0 (donor redemption)
  receives scheduling within p95 < 30 minutes in the steady-state
  cluster. A job submitted with NCU = 0 and positive public votes
  receives scheduling within a worst-case bound of ~7 hours
  (starvation freedom via S_age). No job is ever permanently blocked.
- **SC-008**: The project publishes a quarterly financial report and
  an aggregate energy/carbon footprint report within 14 days of the
  quarter close for every quarter from GA onward.
- **SC-009**: Joules-per-successful-NCU improves by at least 10%
  year-over-year from the first year of GA operation (Principle IV:
  growth without efficiency improvement is a governance failure).
- **SC-010**: 100% of production components at GA have published
  direct-test evidence artifacts with real-hardware execution traces;
  unverified components cannot be deployed (Principle V enforcement).
- **SC-011**: Security-incident disclosure reaches the public
  disclosure page within 72 hours of mitigation for Principle I
  incidents, measured across all incidents in a year.
- **SC-012**: 100% of jobs dispatched to production donors carry a
  signed verification artifact queryable by the submitter.

## Assumptions

- The project launches as a US 501(c)(3) public charity (Delaware,
  ISRG/Let's Encrypt model), incorporated in parallel with the Phase 2
  testnet and in place before Phase 3 alpha. Compliance with US export
  controls (EAR) and OFAC sanctions programs is a baseline assumption;
  an EU subsidiary may be added post-GA if EU-donor fiscal sponsorship
  warrants it.
- The minimum viable donor pool for Phase 2 is ~50 nodes distributed
  across ≥18 autonomous systems; below that threshold, cold storage
  falls back to 3× replication on hot nodes only.
- Coordinator nodes (the sharded-Raft system of record) are a small
  set (target ~100–1000) operated by the project and vetted partners,
  NOT ordinary volunteer donor hardware. They are not a blockchain; they
  are a federated, hardened, transparency-logged operator quorum.
- GPU passthrough in v1 is limited to Linux donors with verified
  singleton IOMMU groups and NVIDIA consumer-GPU registration-time
  probing. macOS GPU donation is blocked on Apple paravirtual-GPU
  availability.
- Mobile and browser-donor modes are deferred to Phase 3 and require
  independent security audit before GA.
- Long-running services, tightly-coupled MPI, and real-time workloads
  are OUT of scope for v1.
- The self-improvement capacity slice defaults to 5–10% of cluster
  capacity and is tunable by governance; it is never 0.
- Donors and submitters are globally distributed; the project does not
  assume any single national jurisdiction's laws beyond the legal entity
  home, though it complies with acceptable-use rules in all
  jurisdictions it reaches.
- All cryptographic primitives used are current, vetted, and auditable;
  no novel crypto is introduced for v1.

## Out of Scope (v1)

- Long-running services / persistent state between jobs beyond
  checkpoints
- Tightly-coupled MPI, latency-sensitive HPC
- Real-time workloads (e.g., interactive inference SLOs)
- Homomorphic computation over submitter-private data (future research)
- A World Compute-native cryptocurrency or token traded on external
  markets
- Federated learning coordinator (may be a future workload class)
- Mobile native donor apps (Phase 3)
- Browser WASM donor mode (Phase 3)

## Related Documents

- `research/01-job-management.md` — Job management architecture research
- `research/02-trust-and-verification.md` — Trust, verifiable compute,
  blockchain analysis
- `research/03-sandboxing.md` — Sandboxing & host integrity research
- `research/04-storage.md` — Distributed storage / erasure coding research
- `research/05-discovery-and-bootstrap.md` — P2P discovery / libp2p / NAT
- `research/06-fairness-and-credits.md` — Fairness, scheduling, credits
- `research/07-governance-testing-ux.md` — Governance, funding, testing,
  CLI/GUI
- `research/08-priority-redesign.md` — Multi-factor priority queue,
  open-access model, human-verified voting, Sybil resistance
- `research/09-mesh-llm.md` — Distributed MoE mesh LLM architecture
  for self-improvement (Principle IV)
- `research/10-prior-art-distributed-inference.md` — Survey of Petals,
  Hivemind, Exo, SWARM, proof-of-personhood systems
- `design/architecture-overview.md` — Consolidated architecture design
  document
- `whitepaper.md` — Public-facing whitepaper
- `README.md` (repo root) — Public README with API reference
