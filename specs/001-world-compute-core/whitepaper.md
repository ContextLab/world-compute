# World Compute: A Decentralized, Volunteer-Built Compute Public Good

**The World Compute Project**
**Date**: 2026-04-15
**Version**: Draft v0.1

---

## Abstract

The world's personal computers, laptops, servers, and workstations sit largely idle. This paper describes World Compute, a planetary-scale volunteer compute federation that turns that idle hardware into a public good — a single, self-organizing cluster that any person or institution can contribute to and draw from. World Compute borrows the volunteer-compute model pioneered by BOINC and Folding@home, extends it with modern cryptographic accounting, libp2p-based peer discovery, hypervisor-boundary sandboxing, and a three-tier hierarchical scheduler, and governs itself as a US 501(c)(3) public charity. Unlike prior systems, it requires no blockchain, no staking, no token, and no trusted central operator. Two machines on a LAN with the agent installed form a cluster in under two seconds with zero configuration; that cluster transparently merges into the global federation the moment internet connectivity is available. Donor machines are protected by hardware VM boundaries and sub-second preemption — if you touch your keyboard, cluster jobs freeze instantly.

---

## Introduction and Motivation

A laptop sitting idle on a desk delivers roughly the same throughput as a server rack did ten years ago. Most of that capacity goes unused. Globally, the underutilized compute on personal devices dwarfs the capacity of most national supercomputer centers. Volunteer compute projects recognized this fact in the late 1990s. BOINC and its affiliated projects — SETI@home, Einstein@home, Rosetta@home — demonstrated that ordinary people would donate idle cycles to science, and at peak ran on hundreds of thousands of machines. Folding@home exceeded 2.4 exaFLOPS during the COVID-19 pandemic, briefly surpassing every supercomputer on Earth.

These projects plateau for understandable reasons. Each runs its own centralized server infrastructure; joining one project's compute pool does not help another. Donors cannot easily redeem their contribution for compute they need. The sandboxing model — running unsigned native binaries in a process sandbox — has not kept pace with the threat landscape. Credit accounting is per-project and not auditable by donors. There is no mechanism for a donor's laptop and a university's idle cluster to discover each other and cooperate.

A 2026 redesign can address each of these. The libp2p networking stack, battle-tested at the scale of the Ethereum Beacon Chain (500,000+ nodes), provides zero-configuration LAN discovery and NAT traversal as primitives. Firecracker microVMs, Apple Virtualization.framework, and Hyper-V provide hardware-enforced VM isolation at near-native performance — the same technology AWS Lambda uses to run untrusted code safely. Reed-Solomon erasure coding with content-addressed storage (CIDv1) provides durable data storage across unreliable, churning donors. Threshold-signed, hash-chained append-only ledgers with external transparency anchors provide tamper-evident accounting without a blockchain. Modern CPU and GPU hardware increasingly includes hardware attestation roots (TPM 2.0, AMD SEV-SNP, Intel TDX) that let the system prove what code is running on a donor's machine.

World Compute assembles these components into a single coherent system, governed by five constitutional principles that are binding on every design decision.

---

## The Five Principles

The World Compute Constitution (`.specify/memory/constitution.md`, version 1.0.0, ratified 2026-04-15) defines five principles that override every other consideration. Understanding them is understanding the system.

**Safety First** is the precondition for the project's existence. Donors lend hardware they use for their lives and livelihoods. A single real breach of a donor machine would permanently destroy public trust. Consequently, every workload runs inside a hardware-enforced VM boundary — not a container, not a namespace, not a process sandbox. The agent never accesses host credentials, files, peripherals, or LAN state. Cryptographic attestation proves what is running before any job is dispatched. A discovered sandbox escape is a P0 incident that halts new dispatches cluster-wide until a fix is verified.

**Robustness** reflects the reality that a cluster of the general public's machines experiences churn rates orders of magnitude higher than a datacenter. Every node is assumed unreliable. Every job checkpoints to erasure-coded storage. Every in-flight task has pre-warmed replicas elsewhere. The control plane survives the loss of any region or coordinator. Network partitions are the normal operating condition, not an edge case.

**Fairness and Donor Sovereignty** is what keeps donors participating. Donors are not a resource to be exploited; they are sovereign owners generously sharing. Their local user always takes absolute priority over cluster workloads. If a donor touches their keyboard, cluster jobs freeze within 10 milliseconds. In exchange, donors earn credits they can redeem for compute of at least the same caliber class as what they donated — a hard contractual guarantee, not a hope. Paying sponsors never preempt donor-redemption jobs.

**Efficiency and Self-Improvement** recognizes that wasted cycles on donor machines are a real cost — to donors and to the planet. The scheduler is locality- and energy-aware. A permanently reserved 5–10% of cluster capacity runs self-improvement workloads: scheduler optimization, sandbox hardening, storage efficiency, and research into making the system cheaper and safer. At planetary scale, the system publishes its aggregate energy and carbon footprint and is required to reduce joules-per-useful-result year over year.

**Direct Testing** is non-negotiable. No component ships until it has been tested on real hardware, with real sandboxes, returning real correct answers. Mocks may be used for regression speed but cannot be the sole evidence of correctness for any component entering production. Every release produces a published evidence artifact: the jobs that ran, the systems they ran on, the expected outputs, and the observed outputs. A failing direct test blocks deployment. There is no "we'll fix it next release" exception for Principles I, II, III, or V.

These principles are not aspirational. They are load-bearing for every requirement in the system. When a design decision appears later in this paper, it traces back to one or more of them.

---

## System Overview

World Compute is easiest to understand as three concentric layers: the **agent** on every donor machine, the **cluster** of self-organizing agents, and the **global federation** that ties clusters together.

### The Agent

Every participating machine runs a single background daemon — the World Compute agent. It is a statically linked, reproducibly built, code-signed binary (Rust with `clap` for the CLI surface, Tauri for the desktop GUI). It hosts the machine's peer identity (an Ed25519 keypair; the Peer ID is the multihash of the public key), drives the local VM sandbox, enforces donor sovereignty in real time, and communicates with the broader cluster via libp2p.

The agent drops privileges to the minimum required after initialization. It accepts donor configuration — which job classes to accept, resource caps, scheduling windows — via a local CLI or GUI. A donor can pause, resume, or permanently withdraw at any time. On withdrawal, the agent wipes its working directory, revokes its keypair, and leaves no files, processes, scheduled tasks, or network state on the host.

### Fractal Self-Organizing Clusters

Clusters form automatically. Two machines on the same LAN run mDNS (RFC 6762 multicast DNS) and discover each other within 200 milliseconds. They perform a Noise protocol handshake, exchange peer records, build a two-node Kademlia DHT island, and form a functioning cluster. No configuration files. No admin intervention. No internet required.

When the LAN has internet access, any node that reaches a DNS bootstrap seed (analogous to Bitcoin's DNS seeds — a small set of DNS names the project operates, returning current bootstrap peer addresses as TXT records) merges the local DHT island into the global Kademlia DHT. The merge is automatic; Kademlia routing table updates handle it without any special procedure. The LAN cluster does not disappear — it becomes a sub-cluster of the global federation, preferring local peers for data-local jobs while participating in the global work queue simultaneously.

If internet connectivity later drops, the LAN cluster continues operating autonomously. Principle II requires graceful degradation: a partition is a normal operating condition, not a failure.

### Three-Tier Scheduler

The scheduler is hierarchical and decentralized, avoiding both the fragility of a single central scheduler and the Byzantine vulnerability of a fully gossip-only design.

The **global control plane** is approximately 100–1,000 elected coordinator nodes running sharded Raft — one Raft group per shard of the job catalog, roughly 64 shards initially. Coordinators are hardened, operator-vetted nodes, not ordinary volunteer hardware. They hold the durable workflow catalog, the credit ledger, and the acceptable-use policy. They are the system of record but are never on the critical path of a single task's execution.

**Regional brokers** own task queues for a geographic or network region. They match tasks to nearby agents using ClassAd-style bilateral matchmaking (borrowed from HTCondor, which has used this approach in production for over 30 years): tasks advertise requirements, agents advertise capabilities, the broker finds compatible pairs. Brokers manage leases, speculative execution, and lineage tracking. On a 3-machine LAN, one of the agents becomes a transient local broker. Brokers are drawn from the donor pool — any well-behaved, consistently available agent can serve as a broker for its local neighborhood.

**Local agents** own donor sovereignty entirely. The agent — and only the agent — decides when to freeze, checkpoint, or terminate a running sandbox. No network round-trip is involved. Preemption authority lives locally because the 10-millisecond SIGSTOP deadline is physically impossible to meet if a remote coordinator must be consulted first. This design decision is a direct consequence of Principle III.

### Content-Addressed Data Plane

All workload artifacts, inputs, outputs, and checkpoints are identified by CIDv1 (SHA-256). The scheduler never moves bytes; it moves CIDs. Storage is the job of the data plane.

Cold data is stored with Reed-Solomon RS(10,18) erasure coding: 10 data shards and 8 parity shards, any 10 of 18 sufficient for reconstruction. Shard placement enforces geographic dispersion: one shard per autonomous system, no more than 2 shards per country, at least 3 continents represented. This means a single-country internet outage affects at most 2 of 8 parity shards, well within the loss budget. At 10% per-shard unavailability (a realistic donor online rate), the probability of data loss is approximately 2×10⁻⁵.

Hot data (active job inputs and outputs during execution) lives on the executor nodes with 3× synchronous replication. It is wiped on job completion — no residual cluster state on donor machines, per Principle I.

### Layered Verification

The system cannot assume donor nodes are honest. Result verification uses three complementary mechanisms.

By default, every task runs on R=3 independent replicas drawn from disjoint autonomous systems, disjoint /24 IP prefixes, and disjoint Trust Score buckets. The broker collects all three results and accepts the canonical hash agreed upon by at least 2 of 3. Disagreeing nodes take a Trust Score penalty; no credits are earned.

On top of quorum, 3% of accepted results are randomly re-executed on independent high-trust nodes as a spot-check audit. At a 3% audit rate, an attacker who has fooled 100 quorums faces a 95% probability of detection. Over 200 jobs, that rises to 99.8%.

For nodes with hardware attestation (AMD SEV-SNP, Intel TDX, TPM 2.0-measured Firecracker), the quorum can collapse to R=1 while maintaining equivalent trust guarantees — the hardware proves what code ran and that it was not tampered with. This is the upgrade path as attested hardware becomes more prevalent.

Every node accumulates a Trust Score: a bounded function of result consistency (50% weight), attestation level (30%), and age (20%), discounted by recent failures. New nodes start capped at 0.5 for the first 7 days, ramping toward 1.0 after 30 days of consistent quorum agreement.

### Tamper-Evident Credit Ledger

Credit accounting uses an append-only, Merkle-chained, threshold-signed log — detailed in the next section.

---

## Trust and Correctness Without a Blockchain

The most common question about a decentralized compute system is: "Why don't you use a blockchain?" The second most common is: "If not a blockchain, how do you prevent fraud?"

### Why Not a Blockchain

A blockchain — whether proof-of-work or proof-of-stake — requires global consensus on the ordering of every event. That consensus has a cost: latency, energy, and complexity. For a system governed by Principle IV (efficiency as a core obligation), burning donor cycles and kilowatt-hours on consensus coordination is unacceptable. Proof-of-work is explicitly an energy waste by design. Proof-of-stake requires financial collateral, which Principle III forbids — a small donor with a spare laptop should be able to participate without posting economic stake.

The blockchain ecosystem's primitives, however, are valuable and are used throughout World Compute: Ed25519 signatures, Merkle DAGs, hash-chained logs, threshold signatures, and transparency logs. The chain itself is not needed.

### The Ledger Architecture

The credit ledger is a **CRDT-replicated, hash-chained, threshold-signed append-only log with periodic Merkle-root anchors to external transparency logs**.

Each ledger event (credit earned, credit spent, job result accepted, governance action taken) is a record containing: the hash of the preceding record, the event type, the node and job identifiers, the NCU amount, a timestamp, and a threshold-signed witness from at least 3 of the coordinator quorum. Records form a Merkle chain per coordinator, equivalent in structure to IETF Certificate Transparency (RFC 6962). The chain is content-addressed with CIDv1, giving it the same durability as the storage plane.

Every 10 minutes, the coordinator quorum computes the Merkle root of all per-coordinator log heads and anchors it to two external transparency logs: Sigstore Rekor (already operated for open-source supply-chain transparency) and a World Compute-operated CT-style log mirrored to a third-party operator. These anchors are public, permanent, and independent of the project. A compromised coordinator cannot rewrite history without producing a Merkle root that contradicts the already-published external anchors. Third parties can verify any credit event with no trusted intermediary.

Donor-facing balance views are derived from the append-only log via a CRDT OR-Map index. Reads are local and instant. Writes propagate via GossipSub across the coordinator quorum and are replicated with the same RS(10,18) durability as other cold data.

### Comparison to Blockchain Alternatives

| Property | Blockchain (PoW/PoS) | World Compute Ledger |
|-|-|-|
| Write latency | Seconds to minutes (block time) | Sub-second |
| Energy cost | High (PoW) / Medium (PoS) | Negligible |
| Collateral required | Yes (PoS) | No |
| Tamper evidence | Yes | Yes (external transparency anchors) |
| Auditability | Public chain | Public transparency log |
| Donor can verify their own balance | Yes | Yes (local verification, `worldcompute donor credits --verify`) |
| Global consensus on every event | Yes (block inclusion) | No (coordinator quorum, not global) |

The ledger gives every property that matters — tamper evidence, public auditability, cryptographic non-repudiation of every event — at a fraction of the coordination cost. A donor can verify their own credit history locally in O(log n) time without trusting any server.

For further detail, see `research/02-trust-and-verification.md`.

---

## Donor Sovereignty

Principle III is the commitment that makes donors willing to participate at all. It has concrete, measurable, testable requirements.

**Sub-second preemption.** The agent runs a sovereignty monitor observing keyboard and mouse activity, foreground application changes, thermal state, battery level, memory pressure, and user-defined triggers. When any trigger fires, the agent sends SIGSTOP to all running workload processes within 10 milliseconds. CPU and GPU timeslices yield immediately — the donor's interactive experience is unaffected. Within 500 milliseconds, the agent attempts a checkpoint: the job flushes its state to the local scratch directory and notifies the broker. If the checkpoint completes, the broker reschedules the job from that checkpoint on another available node. If it does not complete in time, the agent sends SIGKILL and the broker reschedules from the last committed checkpoint stored in the RS(10,18) data plane.

GPU preemption is a known hard problem. In-flight CUDA kernels do not respond to SIGSTOP instantaneously. In v1, GPU donors run workloads with kernel windows targeted at 200 milliseconds or less, and GPU certification requires passing a real-hardware preemption-latency test. This is an honest limitation, documented as an open question rather than glossed over.

**Same-caliber redemption guarantee.** Credits are tagged with the **caliber class** of the hardware that earned them: 0 (Raspberry Pi), 1 (consumer CPU laptop), 2 (consumer GPU workstation), 3 (prosumer GPU), 4 (data center GPU). When a donor redeems credits, the scheduler is required to place that job on hardware of caliber class at or above the earned class. The 95th-percentile queue time for a same-caliber redemption job is under 2 hours. If a donor is willing to accept a lower-caliber node, they receive a compensation credit multiplier on the consumed credits.

**No "pay for priority."** Paying sponsors occupy a scheduling class explicitly below donor-redemption jobs. A sponsor cannot pay to preempt a donor's redemption allocation. This is a bylaw-level commitment, not merely a scheduler policy.

**NCU credits.** The Normalized Compute Unit (NCU) is defined as 1 TFLOP/s of FP32 throughput for 1 second on a reference platform (NVIDIA A10G). Credit is multi-dimensional using Dominant Resource Fairness: compute, memory, storage, and network are each measured, and the dominant dimension determines the credit earned and consumed. This prevents donors from over-claiming credit on dimensions they minimally stress, and prevents submitters from under-declaring resource usage.

Credits decay with a 45-day half-life to prevent hoarding. A minimum balance floor protects donors who take a brief break. Supply is monitored weekly; if outstanding credits exceed 110% of trailing redemption demand, the decay rate increases automatically until the ratio normalizes. This is the lesson of BOINC's credit inflation problem, applied concretely.

---

## How Clusters Form

The self-organizing story is best told with a single concrete example.

Two researchers at a university place three machines on a switch in a conference room with no internet access and no pre-configuration. They install the World Compute agent on all three and start it. Within 2 seconds, all three machines have discovered each other via mDNS multicast, performed Noise protocol handshakes, formed a 3-node Kademlia DHT, and established a GossipSub mesh. The cluster is operational. They can submit and run jobs.

Six hours later, the switch is connected to the university's network. Any node that successfully reaches a DNS bootstrap seed merges the local DHT into the global DHT via normal Kademlia routing table updates. The LAN cluster doesn't stop being a cluster — it becomes a visible sub-cluster of the global federation, preferring local peers for data-local work while participating in the global work queue for jobs that match its capability profile.

For machines behind NAT, the agent tries in order: UPnP-IGD/NAT-PMP port mapping (succeeds on roughly half of consumer routers), then libp2p DCUtR UDP hole punching (succeeds for approximately 85% of internet hosts based on WebRTC production data), then Circuit Relay v2 as a guaranteed fallback. Machines behind symmetric NAT — common on cellular and many corporate networks — always use Circuit Relay. Well-connected donors with public IP addresses can opt in to run relay nodes, earning compute credits for relay bandwidth donated.

The key invariant: no donor needs to configure anything. No port forwarding. No IP addresses. No registration with a central server. The agent handles all of it.

For the technical details of the libp2p stack selection, S/Kademlia Sybil resistance, and adapter patterns for HPC clusters, Kubernetes operators, and cloud tenants, see `research/05-discovery-and-bootstrap.md`.

---

## Sandboxing and Host Integrity

Principle I's requirement — "no path to the host kernel, host filesystem, host network credentials, peripherals, or host user data" — cannot be satisfied by a container or process sandbox. The attack surface of a process sharing the host kernel is the entire kernel syscall table. Kernel exploits have repeatedly crossed container boundaries. For World Compute, where workloads are submitted by arbitrary third parties worldwide, the boundary must be a hardware-enforced hypervisor privilege boundary.

The sandbox tier per platform:

**Linux (primary tier)** uses Firecracker microVMs on KVM. Firecracker's minimal device model — no USB, no PCI bus, no BIOS, no legacy hardware emulation — gives it a small auditable attack surface. Boot-to-userspace latency is under 125 milliseconds. CPU overhead is under 5% compared to bare metal. These are CI-enforced specifications, not marketing claims. AWS Lambda and Fly.io run Firecracker in production at scale. Where KVM is unavailable (nested virtualization, some cloud VMs), Kata Containers with QEMU-lite is the fallback — still VM-boundary isolated.

**macOS** uses Apple Virtualization.framework, the same foundation Docker Desktop uses on Apple Silicon since mid-2025. Guest memory is mapped outside the host process address space. The guest kernel runs at ARM EL2 (on Apple Silicon) or VMX root mode (on Intel), providing hardware ring-level isolation.

**Windows** uses Hyper-V isolation via the WSL2 utility VM infrastructure on Windows Pro/Enterprise. Windows Home uses QEMU with WHPX (Windows Hypervisor Platform) acceleration — still VM-boundary isolated, with higher overhead.

GPU passthrough on Linux uses VFIO with IOMMU isolation. The agent verifies at registration time that the GPU occupies a singleton IOMMU group; it rejects passthrough if not. The ACS-override patch, which bypasses this isolation requirement, is explicitly prohibited. macOS GPU passthrough to guest VMs is not supported by Virtualization.framework as of 2026; Mac donors are CPU-only nodes. Windows GPU donors use the NVIDIA CUDA on WSL2 path.

**Attestation** uses a two-layer model. The agent binary itself is measured: TPM 2.0 PCR extension on x86 hosts, Apple Notarization chain on macOS, Authenticode on Windows. The workload image is verified by content-addressed CID before dispatch. For hosts with SEV-SNP or TDX, the hardware produces a signed attestation report covering the guest memory pages at launch, providing cryptographic proof that a specific, unmodified workload is running even against a potentially compromised hypervisor.

Sandbox escapes are P0 incidents. The control plane can remotely disable affected agent versions and halt new dispatches to those nodes cluster-wide within one release cycle. This is Principle I's emergency clause made operational.

Adversarial testing of sandboxes — VM escape attempts, IOMMU isolation verification, network isolation, peripheral isolation, attestation bypass — is required on every release and is a condition of the staged release gates. See `research/03-sandboxing.md` for the full red-team test plan.

---

## Storage

Every artifact in World Compute — workload images, job inputs, outputs, checkpoints, and the credit ledger — is identified by a CIDv1 content address (SHA-256). The address is a cryptographic commitment to the content. A donor cannot serve you the wrong bytes for a CID; the hash check fails immediately.

Cold storage uses Reed-Solomon RS(10,18): 10 data shards, 8 parity shards. Any 10 of 18 shards reconstruct the original. Shard placement is enforced by the coordinator: one shard per autonomous system, no more than 2 shards per country, at least 3 continents. This means:

- Any 8 simultaneous random shard losses are tolerated.
- Complete loss of all nodes in any single country (capped at 2 shards) is tolerated.
- Simultaneous loss of nodes in 3 different countries (6 losses) is tolerated.

At 10% per-shard unavailability — realistic for donors who are online 14–22 hours per day — the probability of data loss is approximately 2×10⁻⁵. Without the geographic placement enforcement, that figure rises to 4×10⁻³ at 20% churn — 200 times worse.

Job data is encrypted client-side before sharding using ChaCha20-Poly1305 with a random per-chunk key. Chunk keys are wrapped with the submitter's public key and stored in the job manifest. Individual donor storage nodes see only ciphertext shards, indistinguishable from random bytes. No individual donor, and no set of fewer than 10 colluding donors, can reconstruct the plaintext.

The data plane is built on libp2p transport and CIDv1 primitives, with a custom erasure-coding and placement layer rather than reusing IPFS Bitswap. IPFS does not natively enforce geographic shard placement, does not provide guaranteed availability windows, and does not integrate with job scheduling locality hints. The design reuses proven components (liberasurecode for Reed-Solomon, go-cid for CIDv1 addressing, libp2p for transport) while building a custom orchestration layer on top.

For detailed failure probability analysis, metadata plane design, and the hot/cold tier transition architecture, see `research/04-storage.md`.

---

## Governance and Funding

World Compute is incorporated as a US 501(c)(3) public charity. The mission statement is deliberately narrow: operate and improve a volunteer decentralized compute cluster as a public good. The legal structure prevents mission drift by tying tax-exempt status to that specific purpose.

Governance is two-body. A **Technical Steering Committee** (TSC) of 5–7 members, elected by active contributors, makes technical decisions. A **Board of Directors** of 5 members, with seats elected by the TSC, by the individual-donor membership, and one independent seat, makes financial and legal decisions. No company holds more than 2 seats on either body. No TSC member simultaneously serves on the board. Constitutional amendments require a 2/3 supermajority of both bodies.

Financial donations are accepted in fiat currency. Cryptocurrency donations are accepted and converted to fiat immediately; no treasury assets are held in cryptocurrency. Funding is diversified across individual donations, tiered corporate sponsorships (structured as charitable donations, not membership dues), and grants from science and technology foundations. Quarterly financial reports are published within 30 days of quarter close, in both machine-readable and human-readable form. IRS Form 990 is published immediately upon filing.

**Financial donations do not confer compute scheduling priority.** This is a bylaw-level commitment stated in every major-donor agreement. A sponsor can submit jobs through the same submitter pathway as any other user, at the PAID_SPONSORED priority class that explicitly sits below DONOR_REDEMPTION. The scheduler is governed by the constitution, not by donor-relations commitments.

Governance proposals — policy changes, acceptable-use rule amendments, emergency halts — are submitted via the CLI or web dashboard, discussed publicly, and voted on per the published rules. The outcome is recorded on the same tamper-evident ledger as compute provenance. TSC meeting minutes are published within 7 days.

A Public Good Review Board approves workloads for the PUBLIC_GOOD scheduling class: open-access results, non-harmful purpose, scientific validity review, and alignment with donor opt-in classes. Standard approval requires a 14-day public comment period and 60% weighted vote. Humanitarian emergencies use a 7-day emergency track.

---

## Staged Release Plan

World Compute does not ship until it works. Principle V is explicit: no component enters production without a direct-test evidence artifact. The staged release plan operationalizes this.

**Phase 0: Single-Machine Smoke Test.** The agent installs from source and reproducible binary, starts a sandbox, runs a trivial workload (SHA-256 of a known file) correctly 100/100 times, and leaves no files outside the scoped working directory. Adversarial tests include: workload attempts to read `/etc/passwd` (must be blocked), workload attempts to write outside its directory (must fail), workload exits non-zero (agent reports failure and cleans up). A sandbox escape is a kill condition at this phase.

**Phase 1: 3–5 Machine LAN Testnet.** Physical machines — not nested VMs — on a real network. At least one ARM machine. Peer discovery succeeds without manual IP configuration. A node failure mid-job is detected by missed heartbeat, and the job reschedules and completes from checkpoint on another node. Resource yield occurs within 1 second of a simulated keyboard event. No cross-node data leakage. A kill condition: any cross-node sandbox breach, host OOM, or data loss from simulated node failure.

**Phase 2: 20–50 Machine Federated Testnet.** Machines solicited from research groups and trusted contributors across at least 3 geographic regions, including low-end hardware, GPU-capable machines, and machines behind CGNAT. Over a 72-hour continuous run, 80% of submitted test jobs complete correctly with 30% simulated node churn. Network partition recovery: two segments that split for 30 minutes merge without duplicate job execution or data loss. Adversarial tests include Sybil simulation, Byzantine node detection, and flood protection. Kill condition: data loss from churn, a Byzantine node that is not detected, any host machine affected by a workload outside its scoped directory.

**Phase 3: 500–5000 Public Alpha.** Real volunteers, explicit consent, isolated synthetic or clearly-scoped scientific workloads. 90% job completion rate over a 30-day rolling window. Zero Principle I incidents — this is a hard binary gate, not a metric to balance against others. At least one independent security audit completed, with critical and high findings remediated. Energy and carbon footprint published. A red-team exercise (a security researcher attempts sandbox escape on a dedicated test machine with full cooperation) is required, not optional.

**Phase 4: General Availability.** Gated on Phase 3 metrics sustained for 30 days, security audit clearance, legal entity fully incorporated, TSC and board seated, and the incident-disclosure policy tested with at least one drill.

Failing a gate blocks promotion. There is no timeline pressure that overrides a failed gate.

---

## What We Are Not Doing in v1

Explicit scope clarity prevents scope creep and sets honest expectations.

**No MPI or tightly-coupled HPC.** World Compute targets embarrassingly parallel batch workloads, data-parallel ML training within a co-located cohort, and scientific simulation. MPI-style workloads requiring low-latency interconnects across donor home networks are architecturally incompatible with donor link variability.

**No long-running services.** Web servers, databases, and real-time inference endpoints cannot be preempted within the sub-second budget without violating their service-level agreements. Long-running services require a different SLA model; they are a v2 conversation at the earliest.

**No homomorphic computation.** Computation over encrypted data without decryption is an active research area but not viable at general-purpose scale. The confidential-compute path (SEV-SNP, TDX, H100 Confidential Compute) covers the practical need for data privacy during execution.

**No token or cryptocurrency.** NCU credits are an internal accounting unit. They are not traded on external markets. They have no market price. They do not appreciate or depreciate based on speculation. They decay on a fixed schedule. This is a deliberate governance decision: a tradeable token creates financial incentives misaligned with the public-good mission and regulatory complexity inconsistent with 501(c)(3) status.

**No mobile or browser donor mode at launch.** Mobile donor participation and browser-WASM donation are deferred to Phase 3. The browser sandbox is not equivalent to a hardware VM boundary; treating it as equivalent would be a Principle I violation. Mobile donors will be supported for monitoring and job submission (the management interface) before donation is enabled.

**No private federated learning coordinator.** Federated learning over private donor data may be a future workload class; it is not v1 scope.

---

## Open Questions and Honest Limitations

A whitepaper that does not acknowledge hard problems is a sales document. These are the genuinely difficult open questions:

**GPU kernel preemption.** The 10-millisecond SIGSTOP target applies to CPU resources. CUDA and ROCm kernels already in flight do not respond to SIGSTOP; they run to completion. For small kernels (under 200 milliseconds) this is tolerable. For large training kernels, it is not. CUDA MPS and GPU time-slicing APIs exist but are not universally available across GPU generations and driver versions. This requires a dedicated investigation before GPU donation can be broadly certified. The v1 approach — short kernel windows, explicit GPU certification with real hardware preemption-latency testing — is honest about the constraint rather than hiding it.

**Coordinator election.** Who exactly elects the 100–1,000 coordinator nodes that form the global control plane? A proof-of-uptime-plus-attestation mechanism is the leading candidate, but the detailed protocol — including rotation, slashing for misbehavior, and handling of coordinators that go offline — is not yet specified. This is a follow-on research stage.

**Empirical credit calibration.** The NCU benchmarks are defined against a reference platform (A10G), but the real-world normalization factors for heterogeneous hardware require empirical measurement. The ±15% tolerance for benchmark cross-validation is a design target, not a measured threshold. Phase 2 testnet is the first opportunity to calibrate against a realistic donor population.

**Sybil resistance at scale.** The network-layer Sybil resistance (IP diversity constraints in Kademlia routing tables, GossipSub peer scoring) is necessary but not sufficient against a well-resourced adversary with a large botnet spanning many /24 subnets. Full Sybil resistance without economic collateral is an open research problem. The 3% audit layer and Trust Score system provide probabilistic deterrence; they do not provide cryptographic guarantees against a determined state-level attacker.

**Supply chain.** The agent binary is reproducibly built and code-signed, but the build toolchain, the signing key infrastructure, and the reproducibility attestation pipeline are themselves attack surfaces. This requires a formal supply-chain security plan, ongoing transparency log monitoring, and an independent audit before Phase 3.

**Relay bandwidth at scale.** If 15–20% of donors are behind symmetric NAT (a realistic estimate from WebRTC production data), and average job data transfer is substantial, relay bandwidth becomes a real cost. The model — well-connected donors earning compute credits for relay bandwidth — needs empirical validation at scale.

**Long-running job key management.** Session keys for multi-day training runs need a rotation and escrow strategy. The current architecture specifies per-chunk keys wrapped by the submitter's public key; the handoff for a job that checkpoints and resumes across multiple sessions is not yet fully designed.

---

## Call to Action

World Compute is at the design and early prototype stage. Every contribution matters.

**Individuals with hardware to donate:** Install the agent when Phase 1 testing begins (announcement at [worldcompute.org]). A laptop that sits idle at night contributes meaningfully. A machine with a consumer GPU contributes substantially. You will earn NCU credits redeemable for compute you need, and you will see exactly what ran on your machine and what it produced.

**Researchers and scientists:** If you run computations that are embarrassingly parallel, checkpointable, and represent a public good — protein folding, climate modeling, signal processing, machine learning research — contact the project. Public-good jobs run on donated capacity with no credit cost after PGRB approval. Your participation in the Phase 2 testnet shapes the workload taxonomy.

**Security researchers:** The most important work we need is adversarial. If you find a sandbox escape, a protocol weakness, or a credit fraud mechanism we haven't anticipated, we want to hear about it before GA, not after. A responsible disclosure program will be in place before Phase 3.

**HPC centers and universities with idle cluster capacity:** The Slurm pilot-job adapter and Kubernetes operator described in `research/05-discovery-and-bootstrap.md` allow an existing cluster to contribute idle capacity as a first-class World Compute node. The adapter presents your entire allocation as a single logical peer. Your policies, quotas, and local priorities are respected. The contribution is auditable and credited.

**Philanthropists and foundations:** The project needs funding for security audits (minimum 20% of annual budget), core developer compensation, and the hardware required for Principle V's real-hardware test requirements. Financial donations are accepted as 501(c)(3) charitable contributions. A donation does not buy compute priority; it buys the project's independence to serve all of humanity's compute needs equitably. The quarterly financial reports are public. The spending is auditable.

**Engineers:** The full specification is public. The constitution governs contributions. If you want to build a component — the Rust agent, the Firecracker sandbox driver, the libp2p integration, the CLI, the Tauri GUI, the erasure-coding data plane, the credit ledger — start with `specs/001-world-compute-core/spec.md` and the research documents under `research/`. The code-review gates are explicit and tied directly to the five principles.

The largest compute cluster in history need not be owned by anyone. It can be a commons — built by anyone who opts in, governed by principles that protect everyone who participates, and operated permanently as a public good. The design is ready. The work begins now.

---

*The World Compute Project*
*worldcompute.org*
*2026-04-15*

---

## Document References

The following research documents provide full technical depth for each section of this whitepaper:

- `research/01-job-management.md` — Hierarchical hybrid scheduler, job model, R=3 replication, churn tolerance
- `research/02-trust-and-verification.md` — Layered trust stack, blockchain analysis, ledger architecture, Trust Score
- `research/03-sandboxing.md` — Per-platform VM sandboxing, GPU passthrough, attestation, red-team test plan
- `research/04-storage.md` — RS(10,18) erasure coding, CIDv1 addressing, geographic placement, encryption
- `research/05-discovery-and-bootstrap.md` — libp2p stack, mDNS, Kademlia, DCUtR, Circuit Relay, adapter architecture
- `research/06-fairness-and-credits.md` — NCU credits, scheduling hierarchy, preemption mechanics, credit decay
- `research/07-governance-testing-ux.md` — 501(c)(3) governance, staged release plan, CLI/GUI, public API
- `specs/001-world-compute-core/spec.md` — Full v1 feature specification, functional requirements, success criteria
- `.specify/memory/constitution.md` — The five ratified principles governing every design decision
