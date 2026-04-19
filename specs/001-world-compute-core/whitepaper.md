# World Compute: A Decentralized, Volunteer-Built Compute Public Good

**The World Compute Project**
**Date**: 2026-04-18
**Version**: Draft v0.4

**Changelog**:
- v0.4 (2026-04-18): Corrects v0.3 overstatement. Significant subsystems from specs 001–004 are implemented with 802 tests passing across 150+ source files and full CI green on Linux/macOS/Windows, but several critical paths ship with placeholders (see repository README and open GitHub issues). Fully in place: P2P daemon with full libp2p NAT-traversal stack (relay v2 + DCUtR + AutoNAT, validated end-to-end in-process), distributed job dispatch (request-response over CBOR), WASM execution, policy engine (artifact registry + egress allowlist), preemption supervisor (SIGSTOP via nix), scheduler matchmaking, credit decay, storage GC, confidential compute (AES-256-GCM + X25519), mTLS lifecycle, energy metering, all 8 adversarial tests. Scaffolded but placeholder-bearing: mesh LLM (orchestration real, `load_model()` placeholder — no real LLaMA inference yet), attestation root CA fingerprints (zero placeholders → bypass mode), Rekor pinned public key (zero placeholder), Firecracker rootfs (layers concatenated, not ext4), admin ban, platform adapters (Slurm/K8s/Cloud scaffolds not exercised against live systems), Tauri GUI (scaffold only, never built), Docker/Helm (files exist, never deployed), REST gateway (routing logic only, no HTTP listener). Cross-machine WAN mesh formation behind institutional firewalls is the next spec (#60).
- v0.3 (2026-04-17): Overstated completeness — see v0.4.
- v0.2 (2026-04-15): Replaced rigid priority hierarchy with open-access multi-factor scheduling (research/08-priority-redesign.md). Added "Democratic Access and Public Voting" section. Replaced vague self-improvement language with concrete mesh LLM architecture (research/09-mesh-llm.md, research/10-prior-art-distributed-inference.md). Updated "What We Are Not Doing" and FAQ accordingly.

---

## Abstract

The world's personal computers, laptops, servers, and workstations sit largely idle. This paper describes World Compute, a planetary-scale volunteer compute federation that turns that idle hardware into a public good — a single, self-organizing cluster that any person or institution can contribute to and draw from. World Compute borrows the volunteer-compute model pioneered by BOINC and Folding@home, extends it with modern cryptographic accounting, libp2p-based peer discovery, hypervisor-boundary sandboxing, and a three-tier hierarchical scheduler, and governs itself as a US 501(c)(3) public charity. Unlike prior systems, it requires no blockchain, no staking, no token, and no trusted central operator. Two machines on a LAN with the agent installed form a cluster in under two seconds with zero configuration; that cluster transparently merges into the global federation the moment internet connectivity is available. Donor machines are protected by hardware VM boundaries and sub-second preemption — if you touch your keyboard, cluster jobs freeze instantly.

---

## Introduction and Motivation

A laptop sitting idle on a desk delivers roughly the same throughput as a server rack did ten years ago. Most of that capacity goes unused. Globally, the underutilized compute on personal devices dwarfs the capacity of most national supercomputer centers. Volunteer compute projects recognized this fact in the late 1990s. BOINC and its affiliated projects — SETI@home, Einstein@home, Rosetta@home — demonstrated that ordinary people would donate idle cycles to science, and at peak ran on hundreds of thousands of machines. Folding@home exceeded 2.4 exaFLOPS during the COVID-19 pandemic, briefly surpassing every supercomputer on Earth.

These projects plateau for understandable reasons. Each runs its own centralized server infrastructure; joining one project's compute pool does not help another. Donors cannot easily redeem their contribution for compute they need. The sandboxing model — running unsigned native binaries in a process sandbox — has not kept pace with the threat landscape. Credit accounting is per-project and not auditable by donors. There is no mechanism for a donor's laptop and a university's idle cluster to discover each other and cooperate.

A 2026 redesign can address each of these. The libp2p networking stack, battle-tested at the scale of the Ethereum Beacon Chain (500,000+ nodes), provides zero-configuration LAN discovery and NAT traversal as primitives. Firecracker microVMs, Apple Virtualization.framework, and Hyper-V provide hardware-enforced VM isolation at near-native performance — the same technology AWS Lambda uses to run untrusted code safely. Reed-Solomon erasure coding with content-addressed storage (CIDv1) provides durable data storage across unreliable, churning donors. Threshold-signed, hash-chained append-only ledgers with external transparency anchors provide tamper-evident accounting without a blockchain. Modern CPU and GPU hardware increasingly includes hardware attestation roots (TPM 2.0, AMD SEV-SNP, Intel TDX) that let the system prove what code is running on a donor's machine.

World Compute assembles these components into a coherent system governed by five constitutional principles that are binding on every design decision.

---

## The Five Principles

The World Compute Constitution (`.specify/memory/constitution.md`, version 1.0.0, ratified 2026-04-15) defines five principles that override every other consideration. Understanding them is understanding the system.

**Safety First** is the precondition for the project's existence. Donors lend hardware they use for their lives and livelihoods. A single real breach of a donor machine would permanently destroy public trust. Consequently, every workload runs inside a hardware-enforced VM boundary — not a container, not a namespace, not a process sandbox. The agent never accesses host credentials, files, peripherals, or LAN state. Cryptographic attestation proves what is running before any job is dispatched. A discovered sandbox escape is a P0 incident that halts new dispatches cluster-wide until a fix is verified.

**Robustness** reflects the reality that a cluster of the general public's machines experiences churn rates orders of magnitude higher than a datacenter. Every node is assumed unreliable. Every job checkpoints to erasure-coded storage. Every in-flight task has pre-warmed replicas elsewhere. The control plane survives the loss of any region or coordinator. Network partitions are the normal operating condition, not an edge case.

**Fairness and Donor Sovereignty** is what keeps donors participating. Donors are not a resource to be exploited; they are sovereign owners generously sharing. Their local user always takes absolute priority over cluster workloads. If a donor touches their keyboard, cluster jobs freeze within 10 milliseconds. In exchange, donors earn credits they can redeem for compute of at least the same caliber class as what they donated — a hard contractual guarantee, not a hope. Paying sponsors never preempt donor-redemption jobs. Critically, the cluster is open: anyone on Earth can submit jobs, and donated hardware boosts scheduling priority rather than gating access. This makes the system a public good rather than a private club.

**Efficiency and Self-Improvement** recognizes that wasted cycles on donor machines are a real cost — to donors and to the planet. The scheduler is locality- and energy-aware. A permanently reserved 5–10% of cluster capacity runs self-improvement workloads: scheduling optimization, sandbox hardening, storage efficiency, and cluster governance. This capacity powers a distributed mesh LLM — an ensemble of small language models running across GPU donor nodes — that observes cluster metrics, proposes improvements, and validates changes before deployment. At planetary scale, the system publishes its aggregate energy and carbon footprint and is required to reduce joules-per-useful-result year over year.

**Direct Testing** is non-negotiable. No component ships until it has been tested on real hardware, with real sandboxes, returning real correct answers. Mocks may be used for regression speed but cannot be the sole evidence of correctness for any component entering production. Every release produces a published evidence artifact: the jobs that ran, the systems they ran on, the expected outputs, and the observed outputs. A failing direct test blocks deployment. There is no "we'll fix it next release" exception for Principles I, II, III, or V.

These principles are not aspirational. They are load-bearing for every requirement in the system.

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

The blockchain ecosystem's primitives are valuable and used throughout World Compute: Ed25519 signatures, Merkle DAGs, hash-chained logs, threshold signatures, and transparency logs. The chain itself is not needed.

### The Ledger Architecture

The credit ledger is a **CRDT-replicated, hash-chained, threshold-signed append-only log with periodic Merkle-root anchors to external transparency logs**.

Each ledger event (credit earned, credit spent, job result accepted, governance action taken, public vote cast) is a record containing: the hash of the preceding record, the event type, the node and job identifiers, the NCU amount, a timestamp, and a threshold-signed witness from at least 3 of the coordinator quorum. Records form a Merkle chain per coordinator, equivalent in structure to IETF Certificate Transparency (RFC 6962). The chain is content-addressed with CIDv1, giving it the same durability as the storage plane.

Every 10 minutes, the coordinator quorum computes the Merkle root of all per-coordinator log heads and anchors it to two external transparency logs: Sigstore Rekor (already operated for open-source supply-chain transparency) and a World Compute-operated CT-style log mirrored to a third-party operator. These anchors are public, permanent, and independent of the project. A compromised coordinator cannot rewrite history without producing a Merkle root that contradicts the already-published external anchors. Third parties can verify any credit event with no trusted intermediary.

Donor-facing balance views are derived from the append-only log via a CRDT OR-Map index. Reads are local and instant. Writes propagate via GossipSub across the coordinator quorum and are replicated with the same RS(10,18) durability as other cold data.

The same ledger records public vote tallies from the democratic scheduling system (described in the next section), giving voting history the same tamper-evidence guarantees as credit history.

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

The ledger gives every property that matters — tamper evidence, public auditability, cryptographic non-repudiation — at a fraction of the coordination cost. A donor can verify their own credit history locally in O(log n) time without trusting any server. For further detail, see `research/02-trust-and-verification.md`.

---

## Donor Sovereignty

Principle III is the commitment that makes donors willing to participate at all. It has concrete, measurable, testable requirements.

**Sub-second preemption.** The agent monitors keyboard and mouse activity, foreground app changes, thermal state, battery level, and memory pressure. When any trigger fires, SIGSTOP reaches all workload processes within 10 milliseconds. Within 500 milliseconds, the agent attempts a checkpoint; if that completes, the broker reschedules from it on another node. If not, SIGKILL fires and the broker reschedules from the last committed checkpoint in the RS(10,18) data plane.

In-flight CUDA kernels do not respond to SIGSTOP. In v1, GPU donors run workloads with kernel windows targeted at 200 milliseconds or less; GPU certification requires a real-hardware preemption-latency test. This is a documented limitation, not a glossed-over one.

**Same-caliber redemption guarantee.** Credits are tagged with the caliber class of the hardware that earned them: 0 (Raspberry Pi) through 4 (data center GPU). The scheduler places redemption jobs on hardware at or above the earned class. A high NCU balance provides strong scheduling priority (via S_ncu), ensuring fast placement. Donors willing to accept lower-caliber nodes receive a compensation credit multiplier.

**No "pay for priority."** Financial donations to the project do not purchase scheduling priority. Paying organizations that need compute can acquire NCU credits, which feeds into the same priority formula as any other submitter — they compete on equal terms. This is a bylaw-level commitment, not merely a scheduler policy.

**NCU credits.** One Normalized Compute Unit is 1 TFLOP/s of FP32 throughput for 1 second on a reference platform (NVIDIA A10G). Credit is multi-dimensional via Dominant Resource Fairness — compute, memory, storage, and network are each measured, with the dominant dimension determining credit earned and consumed. Credits decay with a 45-day half-life to prevent hoarding; if outstanding credits exceed 110% of trailing redemption demand, the decay rate increases automatically. This is the lesson of BOINC's credit inflation problem, applied concretely.

---

## Democratic Access and Public Voting

### Open Access

World Compute is not a members-only club. Anyone with a verified identity can submit jobs — no hardware donation required. Donating hardware earns NCU credits that provide a significant scheduling priority advantage, but NCU is a boost, not a gate. A non-donor's job is deprioritized, not blocked.

This is not charity toward non-donors. It is a Pareto improvement. A donor who contributes X NCU and consumes Y NCU (where Y < X due to quorum/replication overhead) experiences a net compute loss in a closed donor-only system. In an open system, that surplus capacity serves non-donor jobs, converting overhead into public benefit. The donor's individual experience is unchanged; the social outcome is strictly better.

### Multi-Factor Priority Score

Scheduling priority is a continuous composite score, not a rigid class hierarchy. Every job — regardless of submitter — competes via:

```
P = 0.35·S_ncu + 0.25·S_vote + 0.15·S_size + 0.15·S_age + 0.10·S_cool
```

Each signal is normalized to [0, 1]:

- **S_ncu** — saturating function of the submitter's NCU balance. Deliberately sublinear: a donor with 10× more NCU gains only ~2× the priority score, not 10×, preventing plutocratic dominance.
- **S_vote** — public importance signal from human-verified voting. No votes gives 0.5 (neutral); strong support approaches 1.0. Normalized by sqrt(voter population) so significance scales with electorate size.
- **S_size** — decaying function of job size. Small jobs score near 1.0; large jobs score lower, enabling backfill scheduling.
- **S_age** — monotonically increasing with time in queue. The starvation-prevention signal. Under default parameters, any job — regardless of NCU, votes, or size — reaches competitive priority within approximately 7 hours.
- **S_cool** — penalty for heavy recent consumption. Resets naturally as the 24-hour trailing window slides forward.

LOCAL_USER preemption remains an absolute override outside the formula — donor activity freezes all workloads within 10 milliseconds regardless of priority score.

Example: a donor with 200 NCU submitting a 1-hour job scores approximately P = 0.628. A non-donor submitting a large 100-hour job with strong community support after waiting 8 hours scores approximately P = 0.463. The donor's job runs first, but the non-donor's job runs too — its priority keeps climbing until it does. Weights are governance-configurable. For worked examples and the starvation-freedom proof, see `research/08-priority-redesign.md`.

### Human-Verified Voting and Sybil Resistance

The voting system uses a layered "humanity points" (HP) composite score. Tier 1 (email, phone, social account binding) provides a baseline; Tier 2 adds web-of-trust vouching and proof-of-personhood ceremonies; Tier 3 — active World Compute donor status — is the strongest signal, because a fake donor identity requires real hardware ($50+ per node), making it 3–4 orders of magnitude more expensive to fabricate than a fake email. A voter with 5 HP casts a full-weight vote; below that, vote weight scales proportionally.

Anti-gaming measures include quadratic vote budgets (3 votes on one proposal costs 9 budget units), a 20-vote cap per 30-day epoch, time-weighted voting, and automated detection of accounts voting in lockstep. Vote tallies and HP scores (not identities) are publicly auditable on the tamper-evident ledger. For the full Sybil resistance analysis, see `research/08-priority-redesign.md`.

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

## Safety Hardening and Admission Control

Following an independent red team review, the project added a comprehensive safety hardening layer addressing enforcement gaps in the original design. The full specification is in `specs/002-safety-hardening/`.

**Deterministic policy engine.** Every job submission passes through a 10-step deterministic evaluation pipeline before reaching the scheduler. The pipeline wraps the existing manifest validation with checks for submitter identity, cryptographic signature verification, artifact registry lookup, workload class approval (including quarantine status), resource quotas, endpoint allowlists, data classification compatibility, and ban status. Each evaluation produces an immutable `PolicyDecision` audit record with full reasoning. The LLM advisory layer may flag submissions but is explicitly non-authoritative — it cannot override the deterministic engine's verdict.

**Attestation enforcement.** The original design specified hardware attestation (TPM 2.0, SEV-SNP, TDX) but the verification functions accepted any non-empty quote. The safety hardening replaced these with real structural verification: TPM2 PCR measurements are validated against a `MeasurementRegistry` of known-good values per agent version; SEV-SNP reports are checked against expected guest measurements; TDX quotes are validated against expected MRTD values. Nodes presenting invalid (non-empty) attestation are rejected outright, not silently downgraded. Empty attestation classifies the node as T0 (WASM-only, public data, 5x replication).

**Default-deny network egress.** All sandbox drivers enforce default-deny outbound networking at the hypervisor/namespace level. RFC1918 private ranges, link-local addresses, cloud metadata endpoints (169.254.169.254), loopback, and multicast are blocked. Jobs requesting network access must declare approved endpoints in their manifest, validated by the policy engine against an approved endpoint list.

**Governance separation of duties.** No single identity can hold both the WorkloadApprover and ArtifactSigner roles, or ArtifactSigner and PolicyDeployer, within the same approval flow. Safety-critical governance proposals (EmergencyHalt, ConstitutionAmendment) require an elevated Humanity Points threshold (HP >= 5) for voters. ConstitutionAmendment proposals enforce a mandatory 7-day review period before votes can be tallied. The emergency halt function requires cryptographic proof of the OnCallResponder role.

**Incident response.** Containment primitives — FreezeHost, QuarantineWorkloadClass, BlockSubmitter, RevokeArtifact, DrainHostPool — allow authorized responders to contain security incidents within 60 seconds. Every containment action produces an immutable audit record with actor identity, justification, and reversibility status. Quarantined workload classes are rejected by the policy engine automatically.

**Supply chain.** Build provenance metadata (git commit, build timestamp) is embedded in every binary. An approved artifact registry enforces that the signer and approver of each artifact are different identities. Release channels (development, staging, production) enforce sequential promotion — direct development-to-production promotion is blocked.

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

**Financial donations do not confer compute scheduling priority** — a bylaw-level commitment in every major-donor agreement. Organizations that need compute acquire NCU credits and compete via the same multi-factor priority formula as all other submitters.

Governance proposals are submitted via CLI or web dashboard, discussed publicly, and voted on per the published rules. Outcomes are recorded on the tamper-evident ledger. TSC meeting minutes are published within 7 days.

A Public Good Review Board approves proposals for the public voting board: open-access results, non-harmful purpose, scientific validity, and alignment with donor opt-in classes. Standard approval requires a 14-day public comment period and 60% weighted vote; humanitarian emergencies use a 7-day track.

---

## Mesh LLM: Distributed Self-Improvement

Principle IV reserves 5–10% of cluster capacity for self-improvement workloads. The concrete mechanism for this reservation is a **distributed ensemble-of-experts language model** — the mesh LLM — that observes the cluster's own operation and proposes improvements to scheduling, security, governance, and infrastructure.

### Why a Distributed LLM

The self-improvement function requires open-ended reasoning: analyzing scheduling anomalies, drafting configuration changes, evaluating security findings, proposing governance motions. These tasks suit a language model operating as an autonomous agent. Running that model on a centralized server contradicts the project's decentralized architecture and creates a single point of failure. The mesh LLM distributes inference across GPU donors who already participate in the cluster.

### Ensemble-of-Experts Architecture

The mesh LLM is an **inter-model Mixture-of-Experts** system. Each participating GPU donor node runs a complete small language model locally — LLaMA-3-8B at 4-bit quantization requires approximately 4–6 GB of VRAM. A lightweight distributed router selects K-of-N expert nodes per output token, dispatches the input in parallel, receives sparse probability distributions back, and aggregates them to produce the next token.

This is architecturally distinct from pipeline-parallel distributed inference (as used by Petals), where tokens flow sequentially through layer-shards on different nodes. Pipeline parallelism compounds latency multiplicatively and requires all nodes to run the same model architecture — eliminating heterogeneous hardware participation. The ensemble approach needs only one parallel network round-trip per token and tolerates nodes running different model sizes and fine-tunes, provided they share the same tokenizer vocabulary.

At K=4 experts and 100ms inter-node latency, the system achieves approximately **3.2 tokens per second** — too slow for interactive chat, adequate for autonomous agents generating scheduling analyses and governance proposals on minute-to-hour timescales. Bandwidth is negligible: each expert returns its top-256 (token\_id, logit) pairs (1.5 KB) rather than a full 128K-vocab distribution (250 KB), a 99%+ reduction with under 0.1% quality loss.

### Tokenizer Standardization

For logit distributions to be aggregated across heterogeneous models, all experts must share a vocabulary. The mesh LLM standardizes on the **LLaMA-3 tokenizer** (128,256 tokens), covering the LLaMA-3/3.1/3.2 families and the largest actively developed open-source ecosystem. Cross-tokenizer support via vocabulary mapping is a future research direction. See `research/09-mesh-llm.md` for the full survey.

### The Self-Prompting Loop

The mesh LLM operates as an autonomous agent on a slow, deliberate cadence: observe cluster metrics, analyze, propose improvement actions, validate against a simulation harness, human-review non-trivial changes, apply, measure, and repeat. Cycle time is 1–24 hours depending on action class.

At sufficient scale (1,000+ nodes, yielding 3–7 parallel streams within the 5–10% SI budget), the mesh partitions into independent domain agents — scheduler efficiency, sandbox auditing, storage compaction, network topology — each running its own router and sharing results via GossipSub.

### Safety Architecture

All mesh LLM outputs are classified into action tiers:

| Tier | Examples | Approval required |
|-|-|-|
| Read-only | Analyze metrics, generate reports | None |
| Suggest | Draft config changes, governance motions | Human review |
| Sandbox-test | A/B experiment on 1% of traffic | Automated validation + spot-check |
| Deploy-minor | Update non-critical config within pre-approved bounds | 2-of-3 governance quorum |
| Deploy-major | Change scheduler algorithm, modify sandbox policy | Full governance vote + 24h review |

Proposed changes never touch the production cluster directly. They go through a staging environment, a simulation harness replaying the last 24 hours of traffic, and a 1% canary deployment before promotion.

A **governance kill switch** — a signed GossipSub message from any governance participant — immediately halts all inference streams, reverts the last N applied changes (default N=3), and enters read-only mode. The kill switch cannot be disabled or overridden by the mesh LLM itself.

### Phased Rollout

The minimum viable distributed mesh requires approximately 280 nodes (at 5% SI budget, 30% GPU donors) for one inference stream. Each phase transition requires demonstrated stability and a governance vote — no phase unlocks autonomously.

| Phase | Node count | Capability |
|-|-|-|
| 0–1 | 0–500 | Centralized project-operated model; read-only + suggest tiers only |
| 2 | ~280–1,000 | Distributed ensemble enabled; sandbox-test tier after 30-day stability |
| 3 | ~1,000 | 3–7 parallel domain streams; deploy-minor under governance quorum |
| 4 | ~5,000+ | 37+ parallel streams; deploy-major under full governance vote |

For the full architecture, latency model, heterogeneous node compatibility, router design, and prior art survey (Petals, Hivemind, SWARM, MoE literature), see `research/09-mesh-llm.md` and `research/10-prior-art-distributed-inference.md`.

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

**No fully autonomous mesh LLM in v1.** The mesh LLM's higher-tier action capabilities (deploy-minor, deploy-major) are phase-gated and require explicit governance votes at each phase transition. The system cannot self-authorize expanded autonomy. In Phase 0–1, the self-improvement function runs on a centralized project-operated model with read-only and suggest tiers only.

---

## Open Questions and Honest Limitations

A whitepaper that does not acknowledge hard problems is a sales document. These are the genuinely difficult open questions:

**GPU kernel preemption.** The 10-millisecond SIGSTOP target applies to CPU resources. CUDA and ROCm kernels in flight do not respond to SIGSTOP; they run to completion. For kernels under 200 milliseconds this is tolerable. For large training kernels, it is not. CUDA MPS and GPU time-slicing APIs are not universally available. The v1 approach — short kernel windows, real-hardware preemption-latency certification — is honest about the constraint.

**Coordinator election.** The protocol for electing the 100–1,000 global control-plane coordinators — including rotation, slashing, and offline handling — is not yet fully specified. A proof-of-uptime-plus-attestation mechanism is the leading candidate.

**Empirical credit calibration.** NCU normalization factors for heterogeneous hardware require empirical measurement. The ±15% benchmark cross-validation tolerance is a design target; Phase 2 testnet is the first opportunity to calibrate it.

**Priority weight tuning.** The composite formula's initial weights are estimates requiring empirical tuning against real queue dynamics and governance input.

**Sybil resistance at scale.** The layered humanity-points model raises the cost of fake identities substantially but does not eliminate the risk. A well-resourced adversary spanning many /24 subnets can threaten both network routing and voting. Full Sybil resistance without economic collateral remains an open research problem.

**Mesh LLM cold start and quality.** The distributed mesh requires approximately 280 nodes; below that threshold, self-improvement runs on centralized project infrastructure. An ensemble of quantized 7–8B models will not match frontier reasoning — tasks requiring deep novel analysis will need human expert involvement.

**Supply chain, relay bandwidth, and key management.** The build toolchain and signing infrastructure require a formal supply-chain security plan and independent audit before Phase 3. Relay bandwidth costs for the ~15–20% of donors behind symmetric NAT need empirical validation. Session key rotation and escrow for multi-day checkpointing jobs is not yet fully designed.

---

## Call to Action

World Compute is at the design and early prototype stage.

**Individuals with hardware to donate:** Install the agent when Phase 1 testing begins (announcement at [worldcompute.org]). A laptop that sits idle at night contributes meaningfully; a consumer GPU contributes substantially. You will earn NCU credits redeemable for compute you need and see exactly what ran on your machine.

**Researchers and scientists:** If you run embarrassingly parallel, checkpointable workloads that represent a public good — protein folding, climate modeling, ML research — contact the project. Public-good jobs run on donated capacity with no credit cost after review board approval.

**Security researchers:** The most valuable work is adversarial. A responsible disclosure program will be in place before Phase 3. Sandbox escapes, protocol weaknesses, and credit fraud vectors are the findings we most need before general availability.

**HPC centers and universities:** The Slurm pilot-job adapter and Kubernetes operator in `research/05-discovery-and-bootstrap.md` let an existing cluster contribute idle capacity as a first-class node. Your policies, quotas, and local priorities are respected.

**Philanthropists and foundations:** The project needs funding for security audits (minimum 20% of annual budget), core developer compensation, and real-hardware test infrastructure. Financial donations are accepted as 501(c)(3) charitable contributions and do not purchase scheduling priority.

**Engineers:** The full specification is public. Start with `specs/001-world-compute-core/spec.md` and the research documents under `research/`. The code-review gates are explicit and tied directly to the five principles.

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
- `research/06-fairness-and-credits.md` — NCU credits, preemption mechanics, credit decay
- `research/07-governance-testing-ux.md` — 501(c)(3) governance, staged release plan, CLI/GUI, public API
- `research/08-priority-redesign.md` — Open-access multi-factor priority formula, public voting, Sybil resistance, starvation-freedom proof
- `research/09-mesh-llm.md` — Distributed ensemble-of-experts mesh LLM, router design, safety architecture, phased rollout
- `research/10-prior-art-distributed-inference.md` — Prior art survey: Petals, Hivemind, Exo, SWARM, MoE literature, federated learning
- `specs/001-world-compute-core/spec.md` — Full v1 feature specification, functional requirements, success criteria
- `.specify/memory/constitution.md` — The five ratified principles governing every design decision
