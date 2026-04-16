# Research 01 — Job Management, Scheduling, and Execution Architecture

**Stage**: 1 (Job Management) of the World Compute core research series
**Date**: 2026-04-15
**Author**: Scientist agent
**Status**: Draft for synthesis review
**Constitutional anchors**: Principles I (Safety), II (Robustness), III (Fairness/Sovereignty), IV (Efficiency), V (Direct Testing)

---

## 1. Executive Summary

World Compute should adopt a **hybrid hierarchical scheduler** built around an **immutable, content-addressed declarative job manifest** ("WCJob") whose unit of work is the **task** (a single deterministic, sandboxed, checkpointable execution unit), grouped into **DAGs** ("workflow") that can express embarrassingly parallel, batch, MPI-like, and ML-training workloads. Long-running services are **explicitly out of scope** for v1.

Concretely the recommendation is:

1. **Job model**: BOINC-style work units redesigned for modern containerized/WASM workloads, expressed as a CIDv1-addressed manifest. Tasks are the unit of scheduling; workflows are the unit of submission. Default replication factor **R=3** with **2-of-3 quorum agreement** on result hashes for untrusted workloads, escalating to R=5 (3-of-5) for high-value scientific results, and R=1 with deterministic re-execution audit sampling for self-improvement / non-correctness-critical work.
2. **Scheduler topology**: **Three-tier hybrid** — (a) a **global federated control plane** of ~100–1000 elected coordinator nodes running Raft-per-shard for the job catalog, (b) **regional brokers** that own subsets of tasks and match them to nearby donors via libp2p gossip, (c) **per-host local agents** that own sub-second preemption and donor-sovereignty enforcement entirely autonomously. No global scheduling decision is on the critical path of preemption.
3. **Workload packaging**: **Two co-equal first-class formats** — OCI container images (Tier 1/2 sandboxes) and WASM modules (Tier 3 / browser / mobile). Both are content-addressed, signed, attested, and pulled through the storage plane (stage 4) using CIDv1.
4. **Execution model**: Kubernetes-like declarative reconciliation loops at the regional broker layer, BOINC-like pull-based work fetch at the agent layer, with mandatory **periodic checkpointing to the RS(10,18) erasure-coded storage plane** every N seconds (default 60s, manifest-overridable) so any task can resume on any other compatible node without restart-from-zero.
5. **Result verification**: BOINC-style **redundant computation + canonicalized result hashing** as the default; **zero-knowledge / trusted-execution attestation** as an optional escalation for jobs whose results are not bitwise-deterministic (e.g., floating-point ML).

This recommendation is anchored in BOINC's volunteer-compute scar tissue, Kubernetes' declarative reconciliation, HTCondor's matchmaking, and Nomad's small-binary operational simplicity — adapted to a fully decentralized, libp2p-based control plane.

[FINDING:F1] No existing system in the surveyed prior art simultaneously satisfies (a) sub-second preemption for donor sovereignty, (b) Byzantine-tolerant result verification, and (c) operation across both 3-machine LAN micro-clusters and a planet-scale superset. World Compute MUST compose elements from at least three of them. [CONFIDENCE:HIGH]

---

## 2. Prior Art Survey

Each entry: one-paragraph characterization, **what to steal**, **what to avoid**, and a confidence note.

### 2.1 BOINC (volunteer compute, ~2002–present)
The reference implementation of internet-scale volunteer compute. Centralized per-project servers hand out work units to a fleet of untrusted clients; clients pull, compute, return, and the server validates by quorum across replicated executions.

- **Steal**: pull-based work fetch (donors are behind NAT, push doesn't work); replicated execution as the default trust model; explicit work-unit replication factor in the manifest; credit accounting per validated result; client-side throttling and quiet-hours; the operational lesson that "trickle messages" and "result canonicalization" are mandatory, not optional. [EVIDENCE:F1] BOINC's `validate_state` machinery is the single best body of evidence we have that quorum verification works at planetary scale.
- **Avoid**: per-project siloed servers (we want one cluster, not 50); the Berkeley Open Infrastructure assumption that workloads are scientific batch only; the assumption that the local user's only intervention is "pause when active" rather than sub-second adversarial preemption; the entirely centralized control plane.
- [CONFIDENCE:HIGH] for the lessons; BOINC's design is extensively documented and battle-tested over 20+ years.

### 2.2 Folding@home
Similar to BOINC but tighter coupling between the project and the workload (molecular dynamics). Demonstrated >2 exaFLOPS during COVID, the largest single compute system in history at the time.

- **Steal**: GPU-first design assumptions; demonstration that volunteer compute *can* outscale top-500 supercomputers when the workload fits; the concept of "work server tiers" with regional caching.
- **Avoid**: workload-specific monoculture; lack of generalized job model; closed work-server software historically.
- [CONFIDENCE:HIGH]

### 2.3 HTCondor (opportunistic computing, U. Wisconsin, ~1988–present)
The intellectual ancestor of opportunistic / cycle-scavenging schedulers. ClassAd-based bilateral matchmaking: jobs advertise requirements, machines advertise capabilities, the negotiator pairs them.

- **Steal**: **the ClassAd matchmaking abstraction** — this is the right way to express "this task wants a machine with >=8GB RAM, AVX2, NVIDIA GPU with >=12GB VRAM, and is in EU for data-sovereignty reasons"; the explicit notion of "machine owner policy" that overrides job preferences; checkpointing as a first-class concept.
- **Avoid**: the central negotiator (single point of coordination, doesn't survive partitions); the assumption that the pool is mostly long-lived workstations; the operational complexity (HTCondor has dozens of daemons).
- [EVIDENCE:F2] HTCondor ClassAds are the only matchmaking DSL in the survey that has demonstrably handled heterogeneous opportunistic resources at scale for >30 years. [CONFIDENCE:HIGH]

### 2.4 Kubernetes / k3s
Declarative reconciliation: desired state in etcd, controllers continuously drive observed state toward desired state. k3s is the lightweight single-binary distribution.

- **Steal**: **declarative reconciliation as the core control loop**; CRDs / controller pattern; readiness/liveness probes; admission controllers as a security boundary; k3s's single-binary deployability for the local-agent footprint.
- **Avoid**: etcd as the source of truth (too heavyweight, single Raft group, bad WAN behavior, unsuited for adversarial donors); the assumption of trusted nodes; the assumption of stable IP / DNS; the lack of any preemption budget tighter than tens of seconds; the operational complexity of a full K8s cluster on a donor laptop.
- [CONFIDENCE:HIGH]

### 2.5 HashiCorp Nomad
Single-binary, multi-region scheduler with a simpler model than Kubernetes. Supports containers, raw exec, Java, QEMU, and crucially has a "system" job type and federation between regions.

- **Steal**: **single-binary agent**; multi-region federation via gossip + Raft per region; bin-packing scheduler with preemption; pluggable task drivers (we can have firecracker / vf / wasm / hyperv drivers); the fact that Nomad agents can run as both client and server.
- **Avoid**: trusted-cluster assumption; lack of result verification; the Raft-per-region assumes regions are reasonably stable.
- [CONFIDENCE:HIGH] Nomad is the closest existing OSS scheduler in *operational shape* to what World Compute needs.

### 2.6 Apache Mesos
Two-level scheduler: Mesos offers resources to frameworks, frameworks accept and run tasks. Largely superseded by Kubernetes in industry but the architectural idea is still relevant.

- **Steal**: the **two-level scheduling** idea — the global control plane offers resource leases, regional/job-specific schedulers decide what to do with them. This maps extremely well to the donor-sovereignty constraint where the local agent is the *only* entity that can offer (or instantly revoke) a lease.
- **Avoid**: Mesos masters as a centralized fleet; the operational complexity; framework proliferation problem.
- [CONFIDENCE:MEDIUM] Mesos's relevance is conceptual, not operational; few people deploy it now.

### 2.7 Ray, Dask Distributed, Apache Spark
Three different takes on data-parallel and task-parallel distributed computing inside a *trusted* cluster. Ray emphasizes actor model + fine-grained tasks; Dask emphasizes Python-native lazy graphs; Spark emphasizes RDD/DataFrame batch.

- **Steal**: **fine-grained task graphs / DAGs as the workflow representation**; lineage-based recovery (Spark RDDs reconstruct lost partitions by replaying lineage — this is *exactly* the right pattern for volunteer churn, and it composes with checkpointing); the futures/object-store abstraction (Ray's Plasma) which maps onto our libp2p-CIDv1 storage plane.
- **Avoid**: the trusted-cluster assumption (none of them sandbox); the Python-only assumption (Dask/Ray); the assumption of low-latency intra-cluster network; centralized head node / driver.
- [EVIDENCE:F3] Spark RDD lineage recovery is the most successful "recompute on failure" pattern in the survey and is directly applicable to high-churn volunteer compute. [CONFIDENCE:HIGH]

### 2.8 Slurm
Dominant HPC batch scheduler. Hierarchical: controller, backup controller, slurmd per node, partitions, QoS, fair-share accounting.

- **Steal**: **fair-share accounting algorithm** (this is directly relevant to stage 6's NCU credit unit and the donor-redemption guarantee); job arrays for embarrassingly parallel; reservations; QoS classes.
- **Avoid**: MPI / tightly-coupled assumption (we cannot guarantee low-latency interconnects across donor homes); single controller; the operational model of an HPC sysadmin team.
- [CONFIDENCE:HIGH] for the fair-share lessons specifically.

### 2.9 Bacalhau
Compute-over-data on IPFS. Jobs are specified declaratively, executed near the data, results returned as CIDs. Built on libp2p.

- **Steal**: **content-addressed jobs and results** (CIDv1 in/out); the libp2p-native control plane (validates that this is feasible); the "compute near data" locality principle; the explicit decision to NOT try to be a general orchestrator.
- **Avoid**: the small operator set (only ~hundreds of nodes, mostly trusted); the lack of strong sandboxing guarantees; the limited workload types.
- [CONFIDENCE:MEDIUM] Bacalhau is the closest spiritual predecessor but is still small and early.

### 2.10 Golem Network, iExec
Blockchain-anchored decentralized compute marketplaces. Tasks paid via tokens, providers stake, results verified via redundancy + on-chain dispute resolution.

- **Steal**: economic incentive design as a backstop for fairness; on-chain (or cryptographic-ledger) accounting as a tamper-evident audit log; result-dispute protocols.
- **Avoid**: token speculation as the primary user motivation; on-chain settlement on the critical path of every task (latency, cost, ecological footprint); the assumption that providers are profit-motivated rather than altruistic donors.
- [CONFIDENCE:MEDIUM] These systems have not achieved planetary scale and the token-economy framing is in tension with Principle III's "donors are sovereign citizens, not vendors."

---

## 3. Recommended Job Model

### 3.1 Unit of work and granularity

- **Task**: a single deterministic execution unit, scheduled atomically to one sandbox on one host. Bounded by an explicit **resource envelope** (CPU cores, RAM, GPU class, disk, network, wall-clock budget) and an **input set** (list of CIDs) and **output schema** (declared CIDs with size bounds). Tasks SHOULD target 1 minute–4 hours of work on the median donor; tasks longer than that MUST checkpoint at least every 60s.
- **Workflow** (DAG of tasks): the unit of *submission*. Workflows declare task templates, fan-out parameters, dependency edges, replication policy, verification policy, and a desired-completion deadline.
- **Job**: an instance of a workflow with concrete inputs.

[FINDING:F2] Targeting 1m–4h tasks balances scheduling overhead against churn loss. At 1h median task length and a 1h median donor session, expected loss to churn is bounded; below 1m, control-plane overhead dominates; above 4h without checkpointing, churn loss becomes unacceptable. [EVIDENCE:F2] BOINC empirically converged on 30m–8h work units; SETI@home used ~few-hour units. [CONFIDENCE:MEDIUM] Exact thresholds need empirical confirmation per Principle V.

### 3.2 Supported workload types (v1 scope)

| Type | Supported v1? | Notes |
|-|-|-|
| Embarrassingly parallel batch | YES (primary) | The bread and butter; matches BOINC heritage. |
| ML training (data-parallel) | YES | Via parameter-server or all-reduce *within* a co-located cohort; cross-region all-reduce is too latency-bound for donor links. |
| ML inference (batch) | YES | Excellent fit. |
| Scientific batch (sim, render, fold) | YES | Direct BOINC analog. |
| MPI-tightly-coupled HPC | NO (v1) | Latency assumptions are incompatible with donor links; revisit if/when bonded micro-clusters become common. |
| Long-running services (web, DB) | NO (v1) | Violates donor sovereignty (can't be preempted within sub-second budget without violating SLA); explicitly out of scope. |
| Stateful streaming | NO (v1) | Same reason. |

### 3.3 Checkpointing and resume

- **Mandatory**: every task SHOULD checkpoint, every workflow MUST. Checkpoints are written to the storage plane (stage 4) as RS(10,18) erasure-coded CIDv1 objects.
- **Mechanism**: tiered. Tier A — cooperative checkpointing via a small WCJob SDK callback (`wc_checkpoint(state_blob)`); Tier B — sandbox-level snapshot (Firecracker `CreateSnapshot`, Apple VF equivalent, WASM module memory snapshot) for non-cooperative workloads; Tier C — replay from input-CID + lineage if checkpoint cost > recompute cost.
- **Resume invariant**: any task with a valid checkpoint CID MUST be resumable on any sandbox of the same tier and resource class within 30s of being scheduled. This is a release-blocking direct test (Principle V).

[FINDING:F3] Checkpoint-to-storage-plane is the single mechanism that lets us tolerate continuous churn without restarting from zero. It also gives us free migration (preempt-and-resume-elsewhere is just preempt + reschedule + resume-from-checkpoint). [CONFIDENCE:HIGH]

### 3.4 Replication and result verification

| Workload class | Replication | Verification | Rationale |
|-|-|-|-|
| Public-good scientific (default) | R=3 | 2-of-3 canonical hash quorum | BOINC default; well-tested. |
| High-value / paid sponsored | R=5 | 3-of-5 hash quorum | Defense in depth. |
| ML training (non-deterministic FP) | R=2 + spot audit | bitwise-tolerant range check + periodic R=3 audit (~5% of tasks) | Bitwise quorum impossible; cost-controlled. |
| Self-improvement / system internal | R=1 + audit | 1% audit replication | Trusted-ish, low cost. |

Result canonicalization is mandatory: floating point pinned via deterministic libm builds inside Tier 1/2 sandboxes, or declared non-deterministic and routed to range-check verification.

### 3.5 Job specification format

- **Format**: a signed CBOR (or equivalent) manifest, content-addressed as a CIDv1, referencing OCI image CIDs and/or WASM module CIDs. JSON projection for human authorship.
- **Schema** (sketch):

```yaml
wcjob: 1
workflow:
  id: cid:bafy...
  submitter: did:wc:...
  signature: ...
tasks:
  - name: render-tile
    image: oci+cid:bafy... | wasm+cid:bafy...
    args: [...]
    inputs: [cid:..., cid:...]
    outputs: { tile: { max_bytes: 4MB, schema_cid: ... } }
    resources: { cpu: 4, ram_gb: 8, gpu: { class: rtx30+, vram_gb: 8 } }
    walltime_budget_s: 1800
    checkpoint_interval_s: 60
    replication: 3
    verification: hash-quorum
    locality_hint: { prefer_near: cid:input1 }
    preempt_class: yieldable     # vs. checkpointable, vs. restartable
acceptable_use_classes: [scientific, ml-training]
deadline: 2026-05-01T00:00:00Z
priority_tier: PUBLIC_GOOD
```

The matching ClassAd-style requirements language is embedded in `resources` and `locality_hint`; the broker layer evaluates the bilateral fit.

### 3.6 Tolerating churn

- **Pull-based dispatch**: agents pull work when they have spare cycles; never pushed.
- **Lease-based ownership**: a task lease is short (default 5min) and must be heartbeat-renewed; on lease loss the broker reschedules.
- **Speculative re-execution**: if a task lease expires or progresses too slowly, an additional replica is dispatched without canceling the slow one (Spark-style straggler mitigation).
- **Lineage fallback**: if all replicas of a task are lost AND no checkpoint exists, recompute from input CIDs; inputs are durable in the RS(10,18) storage plane.

---

## 4. Recommended Scheduler Architecture

### 4.1 Topology (three tiers)

```
                     +--------------------------------------+
                     |   GLOBAL FEDERATED CONTROL PLANE     |
                     |   ~100-1000 elected coordinators     |
                     |   Sharded Raft (job catalog,         |
                     |   workflow metadata, credit ledger)  |
                     |   Tier-1 hardened nodes only         |
                     +------------------+-------------------+
                                        |
                          libp2p gossip + pubsub
                                        |
              +-------------------------+--------------------------+
              |                         |                          |
   +----------v-----------+  +----------v----------+  +-----------v---------+
   |  REGIONAL BROKER     |  |  REGIONAL BROKER    |  |  REGIONAL BROKER    |
   |  (geographic shard)  |  |                     |  |                     |
   |  Owns a slice of     |  |  ClassAd matching   |  |  Local discovery    |
   |  the task queue;     |  |  Lease management   |  |  via mDNS for LAN   |
   |  Kademlia DHT for    |  |  Speculative exec   |  |  micro-clusters     |
   |  task lookup         |  |                     |  |                     |
   +----------+-----------+  +----------+----------+  +-----------+---------+
              |                         |                          |
              |  pull lease  +----------+----------+  pull lease   |
              +------------->|                     |<--------------+
                             |    LOCAL AGENTS     |
                             |   (donor machines)  |
                             |                     |
                             |  - Sandbox driver   |
                             |  - Preemption MUST  |
                             |    be local-only,   |
                             |    sub-second,      |
                             |    autonomous       |
                             |  - Heartbeat lease  |
                             |  - Checkpoint to    |
                             |    storage plane    |
                             +---------------------+
```

### 4.2 Why three tiers, and what each is responsible for

- **Global control plane**: holds the durable workflow catalog, credit ledger, and acceptable-use policy. Replicated via sharded Raft (one Raft group per shard of the catalog, ~64 shards initially). Coordinator nodes are elected from the highest-attestation, highest-uptime donors and are themselves redundant. The global plane is **not** on the critical path of any single task execution; it is the system of record.
- **Regional brokers**: own task queues for a geographic / network region. Match tasks to local agents using ClassAd-style requirements. Manage leases, speculative execution, and lineage tracking. Communicate via libp2p gossip and the Kademlia DHT (stage 5). Brokers are themselves drawn from the donor pool but with a much lower bar than coordinators — any well-behaved Tier 1 host can serve as a broker for a few hundred local agents.
- **Local agents**: the only entity that touches the donor's actual hardware. **Owns donor sovereignty entirely** — preemption, throttling, quiet hours, SIGSTOP delivery, sandbox lifecycle. The local agent does NOT consult any remote service to preempt; it acts autonomously on local signals (input device activity, thermal, power, user policy) within the sub-second budget mandated by Principle III.

[FINDING:F4] Putting preemption authority exclusively in the local agent is the only design that can meet the sub-second yield budget. Any architecture that requires a network round-trip to preempt fails Principle III by construction. [EVIDENCE:F4] Stage 6's recommendation of `SIGSTOP < 10ms` is achievable only with local-only enforcement. [CONFIDENCE:HIGH]

### 4.3 LAN micro-clusters and the global superset, simultaneously

A 3-machine LAN should be able to function as a self-contained micro-cluster *and* as part of the global federation, both at once. The mechanism:

- mDNS discovery (stage 5) elects one of the three local agents as a **transient regional broker** for the LAN.
- That broker registers itself with the global control plane as a regional shard with `shard_size=3, gateway=true`.
- Tasks submitted by local users to "this LAN only" stay within the LAN broker; tasks submitted to the global pool are visible to both the LAN broker and the wider federation, with locality hints preferring LAN execution when inputs are local.
- If WAN connectivity drops, the LAN broker continues serving local jobs from a cached subset of the catalog (graceful degradation per Principle II).

### 4.4 Donor-sovereignty preemption (Principle III)

- Local agent runs a kernel-level (or near-kernel) **sovereignty monitor** observing: HID activity, foreground process changes, CPU/GPU contention, thermal headroom, battery state, user-defined triggers.
- On any sovereignty event with severity above the configured threshold, the agent immediately:
  1. **t+0ms**: fires `SIGSTOP` (or sandbox-driver-equivalent freeze) to all running task sandboxes. This is the < 10ms commitment from stage 6.
  2. **t+0–500ms**: depending on event, either (a) holds frozen pending recheck, (b) issues a checkpoint-and-evict request to the sandbox driver, or (c) terminates and discards based on the task's `preempt_class`.
  3. **t+500ms–5s**: if checkpoint requested, sandbox writes incremental snapshot to storage plane; agent reports lease release to broker.
  4. **t≥5s**: broker reschedules elsewhere from checkpoint CID.
- The sub-second yield is achieved at step 1; everything later is bookkeeping.

---

## 5. Job Lifecycle State Machine

```
        SUBMITTED
            |
            v
        VALIDATED  <-- acceptable-use, signature, resource feasibility
            |
            v
        QUEUED     <-- in regional broker queue, awaiting matchmaking
            |
            v
        LEASED     <-- agent has accepted; lease heartbeating
            |
            v
        RUNNING    <-- sandbox executing; checkpoints to storage plane
        /  |  \
       /   |   \
      v    v    v
PREEMPTED CHECKPOINTED COMPLETED
   |        |              |
   |        +--> RESCHEDULED -> LEASED (resume from checkpoint CID)
   |                              |
   v                              v
 LOST -> SPECULATED -> ...   VERIFYING <-- broker collects R replicas
                                 |
                                 v
                    +------------+------------+
                    |                         |
                    v                         v
                 VERIFIED                 DISPUTED
                    |                         |
                    v                         v
               DELIVERED              ESCALATED (re-execute, audit)
```

State transitions are durable in the broker's local store and replicated to the global catalog at workflow-completion granularity (not per-task, to keep the global plane lightweight).

---

## 6. Tradeoffs Explicitly Considered and Rejected

### 6.1 Fully peer-to-peer scheduler (gossip-only, no coordinators)
Considered. Rejected. **Why**: Byzantine donors can flood gossip with fake task offers; without an attestation root anchored in the global control plane, there is no way to decide which broker to trust for matchmaking. Gossip-only schedulers (e.g., Sparrow research designs) work in trusted clusters but fail under adversarial conditions. Hybrid wins because the global plane is small enough to harden Tier-1 and large enough to survive regional failure.

### 6.2 Fully central scheduler (BOINC project model writ large)
Considered. Rejected. **Why**: violates Principle II (single point of failure), violates the "no organization should own this" public-good ethos, doesn't scale to planetary work-unit volumes without becoming a distributed system anyway.

### 6.3 Container-only OR WASM-only
Considered each. Rejected both. **Why**: containers can't run inside browsers and most mobile devices (Tier 3); WASM can't yet efficiently express GPU compute (Tier 1 requirement). Both are needed.

### 6.4 No replication, attestation only
Considered. Rejected for v1. **Why**: TEE attestation (SGX, SEV-SNP, TDX, Apple Secure Enclave) is not universally available on donor hardware and has a long history of side-channel breaks. Replication is hardware-agnostic. Attestation will be added as an *option* (R=1+TEE) in a later phase.

### 6.5 Long-running services in v1
Considered. Rejected. **Why**: incompatible with sub-second preemption guarantees. Donor sovereignty wins. Services are a v2 conversation that requires either dedicated always-on donor pools (consenting to no preemption) or a fundamentally different SLA model.

### 6.6 etcd / single Raft for the global catalog
Considered. Rejected. **Why**: etcd is one Raft group; doesn't shard naturally; bad WAN behavior; assumes trusted nodes. Sharded Raft (one group per catalog shard) gives us horizontal scalability and partition tolerance while keeping per-shard linearizability where it matters (credit ledger).

### 6.7 Blockchain-anchored job ledger
Considered. Rejected as the primary mechanism (kept as optional audit anchor). **Why**: latency, cost, ecological footprint on the critical path of every task is unacceptable for a system whose Principle IV explicitly demands joules-per-result improvement.

---

## 7. Direct-Test Plan (Principle V)

All of the following are release-blocking:

1. **Sub-second preemption test**: on a real macOS laptop (Apple VF Tier 2), real Linux desktop (Firecracker Tier 1), and real Android phone (WASM Tier 3), launch a CPU-bound task that ignores `SIGTERM`, then trigger sovereignty events (mouse move, foreground app, thermal). Measure t-from-event-to-CPU-yield. Pass criterion: 95th percentile < 1000ms, 99th < 2000ms, no observed CPU contention with the local user's test workload after t+1s.
2. **Checkpoint-resume across hosts**: run a 30-minute scientific batch task on host A, kill A at t=15min, observe automatic resume on host B from the most recent checkpoint, verify final result bitwise-identical to a non-interrupted control run.
3. **Quorum verification with adversarial worker**: submit 1000 tasks with R=3, inject a malicious worker that returns plausible-but-wrong results for 30% of its assignments, verify (a) the malicious results are caught by quorum mismatch, (b) the malicious worker is downgraded in the credit ledger, (c) end-to-end correctness of all 1000 tasks.
4. **LAN micro-cluster + WAN federation simultaneously**: 3-laptop LAN, with one laptop also bridging to a 50-node global testbed. Submit jobs to both and verify both proceed without the LAN cluster being starved when WAN connectivity flaps.
5. **Sustained 24h churn test**: 100-node testbed with simulated donor churn (median session 90min, exponential), submit a workflow of 10000 tasks, measure end-to-end completion time and waste ratio (compute-spent / compute-credited). Pass criterion: waste < 25% (a deliberately loose initial bound to be tightened over time per Principle IV).
6. **Sandbox escape adversarial suite**: per Principle I, run the standard sandbox-escape test corpus (kernel exploits, side channels, IPC fuzzing) inside the agent's sandbox tier on each supported OS. Any escape is a release blocker.

Every release MUST produce a direct-test evidence artifact for tests 1–6.

---

## 8. Open Questions

1. **Coordinator election**: how exactly is the ~100-1000 coordinator set elected and rotated? Proof-of-uptime + attestation is the leading idea but needs a sibling research stage. Likely a stage 8 or 9 topic.
2. **Cross-region all-reduce for ML**: should we attempt latency-tolerant all-reduce (e.g., gradient compression, async SGD) for cross-region ML training, or restrict ML training to within-region cohorts in v1? Recommendation: within-region only for v1.
3. **Self-improvement budget enforcement**: where exactly does the Principle IV self-improvement budget live in the scheduler? Suggested: a fixed reserved priority class `SELF_IMPROVEMENT` per stage 6, with a global broker policy that ensures it always gets ≥X% of total scheduled cycles. Needs synthesis with stage 6.
4. **Acceptable-use classification**: who decides whether a submitted workflow is "scientific" vs. "ML training" vs. "rendering" vs. "abusive"? Needs an admission-control protocol; likely stage 7 governance.
5. **Confidential-compute integration**: when (not if) do we add TEE attestation as an alternative to redundant execution? Needs hardware survey; probably v2.
6. **Task-language bindings**: do we ship a Python/Rust/Go SDK in v1, or only the manifest format and let users wrap their own? Recommendation: ship Python + Rust SDKs in v1, manifest is the source of truth.
7. **Empirical validation of 1m–4h task granularity**: needs Principle V direct measurement on realistic donor population.

---

## 9. Coherence with Sibling Stages

- **Stage 3 (sandboxing)**: the local agent's sandbox driver layer is exactly where the Tier 1/2/3 split lives. The job manifest's `image` field accepts both OCI (Tier 1/2) and WASM (Tier 3) and the broker's matchmaking respects tier capability. Coherent.
- **Stage 4 (storage)**: every input, output, checkpoint, and image is a CIDv1 in the RS(10,18) storage plane. The scheduler never moves bytes itself; it moves CIDs. Coherent.
- **Stage 5 (discovery)**: regional brokers announce themselves via Kademlia, LAN micro-clusters bootstrap via mDNS, broker-to-agent and broker-to-broker traffic uses DCUtR / Circuit Relay v2 fallback. Coherent.
- **Stage 6 (fairness/credits)**: the lease layer reports validated work to the credit ledger in the global control plane; preemption respects the LOCAL_USER > DONOR_REDEMPTION > PAID_SPONSORED > PUBLIC_GOOD > SELF_IMPROVEMENT hierarchy directly; the sub-10ms SIGSTOP commitment is honored by the local-agent-only preemption design. Coherent.

[FINDING:F5] The recommended architecture is consistent with all four sibling stages without requiring renegotiation of any of their primitives. [CONFIDENCE:HIGH]

---

## 10. Summary Table of Tagged Findings

| Tag | Claim | Evidence | Confidence |
|-|-|-|-|
| F1 | No prior art simultaneously meets sub-second preemption + Byzantine verification + LAN-and-global. | Survey above. | HIGH |
| F2 | 1m–4h task granularity is the right operating range. | BOINC empirical history. | MEDIUM (needs direct test) |
| F3 | Checkpoint-to-storage-plane is the foundational churn-tolerance mechanism. | Spark RDD + BOINC checkpoint history. | HIGH |
| F4 | Preemption authority MUST live exclusively in the local agent. | Sub-10ms requirement is unachievable across a network. | HIGH |
| F5 | The recommendation is coherent with stages 3–6 with no renegotiation. | Cross-stage trace above. | HIGH |

