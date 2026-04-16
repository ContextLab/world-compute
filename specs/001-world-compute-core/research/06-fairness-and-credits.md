# 06 — Fairness, Job Prioritization, and Credit Economics

**Status**: Draft — 2026-04-15
**Scope**: World Compute Core Spec 001
**Constitution anchor**: Principle III (Fairness & Donor Sovereignty), Principle IV (Self-Improvement Budget)

---

## Prior Art Survey

This section distills the key lessons from eight prior systems that inform the World Compute design. All citations are to published papers or publicly documented system behavior.

### BOINC / CreditNew

BOINC's original credit system allowed clients to self-report floating-point operations, which was immediately gamed. The CreditNew overhaul (2008) introduced server-side cross-validation and exponential moving averages over a 7-day window (RAC — Recent Average Credit). Even so, per-project credit multipliers fragmented the economy, and credit inflation devalued contributions over time as hardware improved but per-WU credit stayed fixed. SETI@home and Einstein@home saw users running hundreds of fraudulent machines to top leaderboards. Einstein@home's quorum validation (requiring 3+ independent results to agree) was the most effective cheat-deterrent deployed at scale in volunteer computing.

[FINDING] Self-reported hardware benchmarks are ungameable only if cross-validated server-side with known hardware profiles; quorum result validation is the strongest known deterrent against result fraud in volunteer systems.
[EVIDENCE] Einstein@home quorum validation documented in Anderson et al. (2002), "BOINC: A System for Public-Resource Computing and Storage"; gaming history documented in BOINC developer mailing list archives 2004–2008.
[CONFIDENCE] High — these are well-documented failure modes in production systems.

### Folding@home Points

F@h assigns points per Work Unit based on a k-factor encoding deadline tightness, biological complexity, and device class (CPU vs. GPU vs. PS3). GPU contributions are worth roughly 100× CPU contributions, reflecting real throughput differences. A "time bonus" rewards donors who return results faster than the deadline, incentivizing reliability over raw donation quantity. Points are purely vanity — there is no credit redemption mechanism. This simplifies the system but removes any direct return to donors.

[FINDING] Device-class tiering (not raw FLOPS normalization) is sufficient to prevent low-tier hardware from appearing to match high-tier hardware in credit accounting.
[EVIDENCE] Folding@home points system documented at https://foldingathome.org/support/faq/points/
[CONFIDENCE] High — directly observable from public documentation.

### Dominant Resource Fairness (DRF, Mesos/Spark)

Ghodsi et al. (2011) proved that max-min fairness across multiple resource dimensions — where each user's "dominant share" is the maximum of their fractional usage across all resource types — is simultaneously Pareto efficient, envy-free, and strategy-proof under idealized conditions. DRF is implemented in Apache Mesos, Spark FAIR scheduler, and Hadoop YARN. Its key weakness for World Compute is that it assumes a homogeneous resource pool; it does not natively handle the case where a Raspberry Pi and an H100 are in the same pool.

[FINDING] DRF's multi-dimensional fairness is the right foundation for heterogeneous workload scheduling, but must be extended with hardware-tier tagging to prevent caliber class collapse.
[EVIDENCE] Ghodsi, A. et al. "Dominant Resource Fairness: Fair Allocation of Multiple Resource Types." NSDI 2011.
[CONFIDENCE] High — peer-reviewed, widely implemented.

### Lottery Scheduling (Waldspurger & Weihl 1994)

Proportional-share scheduling via random ticket draws is starvation-free by construction: any non-zero ticket count yields non-zero long-run allocation. Compensation tickets (given to jobs that were recently preempted) reduce short-term unfairness. The main weakness is high variance at small timescales.

[FINDING] Lottery scheduling's starvation-freedom property makes it a suitable mechanism for the PUBLIC_GOOD and SELF_IMPROVEMENT job classes, where strict ordering matters less than guaranteed forward progress.
[EVIDENCE] Waldspurger, C.A. & Weihl, W.E. "Lottery Scheduling: Flexible Proportional-Share Resource Management." OSDI 1994.
[CONFIDENCE] High — well-tested in research OS contexts.

### Slurm Fair-Share Scheduler

Slurm's multi-factor priority model combines fair-share factor (exponential decay of recent usage relative to allocation share), job size, job age (for backfill), and Quality of Service (QOS) class. The fair-share factor uses a configurable half-life (typically 7–14 days). National HPC centers run millions of jobs per month through this system. Backfill scheduling fills scheduling gaps with smaller jobs without indefinitely blocking larger jobs, directly addressing starvation.

[FINDING] Multi-factor priority with configurable decay half-life is proven at national HPC scale and provides a template for World Compute's fair-share accounting window.
[EVIDENCE] Slurm documentation: https://slurm.schedmd.com/priority_multifactor.html; deployed at NERSC, ORNL, NCAR.
[CONFIDENCE] High — documented production deployment at scale.

### Kubernetes Priority Classes and Preemption

Kubernetes PriorityClasses assign integer priority values; when a high-priority Pod cannot schedule due to resource constraints, the scheduler evicts lower-priority Pods. System-reserved classes (system-cluster-critical: 2,000,001,000) preempt everything. PodDisruptionBudgets (PDB) limit the rate of preemption to preserve availability. Graceful termination windows (default 30s) allow cleanup before hard kill. Crucially, Kubernetes priority is administrative (assigned by operators), not earned — there is no credit or fairness layer built in.

[FINDING] Kubernetes preemption mechanics (priority classes + graceful termination period + PDB rate limiting) provide a proven template for World Compute's job preemption implementation.
[EVIDENCE] Kubernetes documentation: https://kubernetes.io/docs/concepts/scheduling-eviction/pod-priority-preemption/
[CONFIDENCE] High — production-proven across thousands of clusters.

### Cloud Spot Markets (AWS Spot, GCP Preemptible)

AWS Spot provides a 2-minute warning before interruption. GCP Preemptible VMs have a 30s shutdown notice and a 24-hour maximum lifetime. Both place checkpointing responsibility entirely on the user. Neither provides fairness guarantees — spot price is determined by market clearing, creating a price-based access hierarchy.

[FINDING] 30-second checkpoint windows are feasible if jobs are designed for it; 2-minute warnings are insufficient for naive ML workloads without explicit checkpoint support. Market pricing is inappropriate for a fairness-first system.
[EVIDENCE] AWS Spot Instance documentation; GCP Preemptible VM documentation.
[CONFIDENCE] High for latency figures; Medium for generalizability of checkpoint feasibility to arbitrary workloads.

### Token/Credit Economies (Golem, iExec, Filecoin)

Golem uses GLM token; providers list price/hour; requestors bid via Dutch auction. iExec uses RLC token with Proof of Contribution validation; workers stake tokens as collateral (cheat-resistant but excludes resource-poor donors). Filecoin enforces cryptographic Proof-of-Spacetime with collateral slashing. All three use blockchain settlement, which adds significant latency; production deployments require off-chain payment channels (state channels) for micro-payments.

[FINDING] Token collateral as cheat resistance works but creates a capital barrier that excludes small donors — incompatible with World Compute's universal participation model. Cryptographic proofs of work/storage are the gold standard if feasible without capital requirements.
[EVIDENCE] Golem Network whitepaper; iExec technical whitepaper; Filecoin spec at spec.filecoin.io.
[CONFIDENCE] Medium — designs are public but operational experience is mixed.

### HTCondor Matchmaking / ClassAds

HTCondor's ClassAd matchmaking allows jobs and machines to publish arbitrary attribute advertisements, with the matchmaker finding compatible pairs. The donor's Startd policy language lets owners specify fine-grained availability rules. Flocking allows jobs to cross pool boundaries when the local pool is full. Preemption via "Vacate" signal gives jobs a configurable window to save state before the machine reclaims resources.

[FINDING] ClassAd-style expressive matching is the right model for World Compute's donor policy layer (donors specifying which job classes, time windows, and resource limits they accept).
[EVIDENCE] HTCondor documentation: https://htcondor.readthedocs.io/
[CONFIDENCE] High — 30+ years of production deployment at research institutions.

---

## 1. Recommended Job Priority and Scheduling Model

### 1.1 Job Classes

Five job classes are defined, ordered by preemption priority (lower number = higher priority):

| Priority | Class | Description | Hard Guarantee |
|-|-|-|-|
| 0 | LOCAL_USER | Local human activity; triggers instant cluster yield | Absolute (constitutional) |
| 1 | DONOR_REDEMPTION | Donor spending earned NCU credits | Yes — Principle III |
| 2 | PAID_SPONSORED | External paying organizations | No |
| 3 | PUBLIC_GOOD | PGRB-approved research/humanitarian | No |
| 4 | SELF_IMPROVEMENT | Cluster self-optimization | Reserved capacity slice |

LOCAL_USER is not a schedulable class — it is a preemption trigger that supersedes all others by constitutional mandate (Principle III: "local human user and their processes ALWAYS take absolute priority").

### 1.2 Hard Guarantees vs. Soft Priorities

DONOR_REDEMPTION jobs carry a hard contractual guarantee: a donor who has earned N NCU credits is entitled to receive N NCU-equivalent compute at caliber class >= their donated tier, within a bounded wait (SLA: 95th percentile queue time < 2 hours). This is enforced by the scheduler holding a reserved capacity pool exclusively for donor redemption jobs proportional to current outstanding credit balances.

All other classes use weighted fair-share scheduling within their priority band, inspired by Slurm's multi-factor model: effective priority = f(fair_share_factor, job_age, job_size, QOS). The fair-share factor decays with a configurable half-life (default: 7 days) to prevent permanent accumulation of priority debt.

### 1.3 Preemption Hierarchy

```
LOCAL_USER  (absolute — SIGSTOP within 10ms, hard kill within 500ms)
  ↓ preempts
DONOR_REDEMPTION  (hard guarantee — served before paid users)
  ↓ preempts
PAID_SPONSORED  (funds operations — below donor minimum)
  ↓ preempts
PUBLIC_GOOD  (runs on unearned capacity — yields to all with claims)
  ↓ preempts
SELF_IMPROVEMENT  (permanent reserved slice — never starved, never prioritized)
```

This ordering directly implements Principle III's prohibition on "prioritizing paying/institutional users over donors' earned allocation." Paid jobs are explicitly below donor redemption jobs.

### 1.4 Starvation Prevention

- **DONOR_REDEMPTION**: Hard SLA with escalation. If a donor job cannot start within 4 hours, it is elevated to EMERGENCY_DONOR class, which preempts PAID_SPONSORED jobs. The 4-hour threshold is configurable per donor tier.
- **PUBLIC_GOOD**: Lottery scheduling within the class (ticket count proportional to approved allocation). Lottery scheduling guarantees forward progress for any non-zero ticket holder.
- **SELF_IMPROVEMENT**: Does not participate in the priority queue. It holds a permanently reserved 5–10% capacity slice. The scheduler never offers this slice to other classes.
- **General**: Backfill scheduling (Slurm-style) fills gaps with smaller jobs. No job with non-zero allocation can be blocked indefinitely.

---

## 2. Credit and Accounting Model

### 2.1 Unit: Normalized Compute Unit (NCU)

1 NCU = 1 TFLOP/s of FP32 throughput sustained for 1 second on a reference platform (NVIDIA A10G). Credit is multi-dimensional:

- **Compute NCU**: FP32 TFLOP-seconds
- **Memory NCU-s**: GB·s of active VRAM/RAM allocation
- **Storage NCU-s**: GB·s of active scratch storage I/O
- **Network NCU-s**: GB transferred

Credit earned = the dominant dimension (per DRF), preventing donors from over-claiming credit on a dimension they minimally stress. Credit consumed by jobs = likewise the dominant dimension.

### 2.2 Hardware Normalization

On node join, the agent runs standardized microbenchmarks (FP32 GEMM, INT8 inference, memory bandwidth, disk IOPS, network uplink). Results are cryptographically signed and submitted to the cluster. Server-side cross-validation against a hardware profile database rejects outliers (> ±15% from expected range for the detected hardware model). Trust scores from Stage 2 weight new-node benchmarks at 50%; weight grows toward 100% with consistent validated results over 30 days.

Hardware tiers:

| Tier | Examples | NCU/hr | Caliber Class |
|-|-|-|-|
| 0 — Embedded | Raspberry Pi, phones | 0.01 | 0 |
| 1 — Consumer CPU | Laptop i7, Desktop i9 | 0.10 | 1 |
| 2 — Consumer GPU | RTX 3080, RTX 4070 | ~30 | 2 |
| 3 — Prosumer GPU | RTX 4090, A5000 | ~82 | 3 |
| 4 — Data Center | A100, H100 | ~312 | 4 |

### 2.3 The "Same Caliber" Guarantee

Donors earn credits tagged with their **caliber class**. When redeeming credits, the scheduler MUST match the job to a node with `caliber_class >= donor_caliber_class`. Specifically:

- An H100 donor (class 4) can demand class 4 nodes. If none are available within the SLA window, the scheduler escalates (EMERGENCY_DONOR).
- Voluntary downgrade: if a donor accepts a lower-tier node, they receive a **caliber compensation refund** of `1 + (donated_class - used_class) * 0.3` NCU multiplier on the consumed job credits (e.g., accepting class 3 when class 4 was owed returns 30% more credits).
- Upward substitution is always allowed (running on a better node than donated is fine; no additional credit charged).

This enforces Principle III's requirement that the guarantee covers "compute, memory, storage, and network tier — not just raw hours."

### 2.4 Credit Decay and Inflation Control

Credits decay with a 45-day half-life:

```
C_remaining(t) = C_earned × exp(−ln(2)/45 × age_days)
```

At 90 days, 25% of earned credits remain. This prevents hoarding and aligns credit value with current hardware availability (lesson from BOINC's credit inflation problem).

**Minimum balance floor**: A donor always retains credits equal to their trailing 30-day contribution rate, regardless of decay. This protects donors who take brief breaks.

**Inflation control**: Total NCU credits in circulation are monitored weekly. If outstanding credits exceed 110% of trailing 30-day redemption demand, the decay rate increases by 10% until the ratio drops below 100%.

### 2.5 Cheat Resistance

- **Benchmark validation**: Server cross-checks benchmark results against device fingerprint and known hardware profiles. Nodes reporting > 15% above expected profile are flagged and assigned Trust Score penalty.
- **Result quorum**: Scientific/public-good jobs require ≥ 3 independent results to agree (within numerical tolerance) before accepting output and crediting workers. This is the Einstein@home model — the strongest known defense against result fraud in volunteer computing.
- **Collateral-free design**: Unlike Golem/iExec, World Compute does not require capital collateral. Instead, cheat resistance is probabilistic (quorum validation) combined with Trust Score reputation loss that reduces scheduling priority.
- **Credit audit trail**: Every credit earn and spend event is logged with node ID, job ID, timestamp, benchmark snapshot, and result hash. The audit log is publicly inspectable by donors (Principle III: "accounting MUST be transparent and auditable").

---

## 3. Local Preemption Mechanics

### 3.1 Preemption Triggers

| Trigger | Latency Target | Mechanism |
|-|-|-|
| Keyboard / mouse | 100ms | libinput / XInput / WinHook event subscription |
| Foreground app change | 200ms | NSWorkspace / GetForegroundWindow / _NET_ACTIVE_WINDOW |
| CPU/GPU thermal > 80°C | 500ms | sysfs / IOKit / WMI poll at 1Hz with hysteresis |
| AC power lost | 500ms | upower / IOKit / WMI power event |
| RAM > 80% | 200ms | /proc/meminfo or vm_stat poll at 2Hz |
| User-defined rules | 1000ms | YAML rule file evaluated by agent loop |
| Screen lock (resume signal) | 500ms | Platform screensaver/lock event subscription |

All thresholds are configurable by the donor.

### 3.2 Preemption Protocol

**Phase 1 — Signal detection (< 10ms)**: Agent event loop detects trigger. Sends `SIGSTOP` (Unix) or `SuspendThread` (Win32) to all worker processes. CPU and GPU timeslice yield immediately. Memory footprint is retained (pages allocated but not actively scheduled).

**Phase 2 — Graceful checkpoint window (0–500ms)**: Agent sends `SIGTERM` + checkpoint request. Job has 500ms to flush checkpoint to local scratch. On success, the agent notifies the scheduler with the checkpoint location.

**Phase 3 — Hard kill (500ms deadline)**: If the checkpoint is not complete, `SIGKILL` / `TerminateProcess`. Partial checkpoints are discarded. The scheduler marks the job for restart from the last validated checkpoint.

**Phase 4 — Resume (idle re-detection)**: After no keyboard/mouse events for N seconds (default: 30s, configurable), or when screen lock is active, the agent restores cluster jobs. Restoration is gradual: nice/ionice priority restored first, full CPU/GPU allocation restored after 5s stability.

This protocol meets the constitutional requirement: "Stop immediately means within a bounded, published latency budget (target: sub-second yield of interactive resources)." Phase 1 achieves sub-100ms for interactive resources; the full protocol completes within 500ms for memory and storage.

### 3.3 Interaction with Job Replication

When a replica is preempted:

1. Scheduler detects the preemption notification within 1s.
2. A pre-warmed hot standby node (from a pool maintained at ≥ 120% of active replica count) is immediately assigned as a replacement replica.
3. The replacement starts from the latest committed checkpoint of any surviving replica — not the preempted node's potentially-partial state.
4. The preempted node's progress is discarded (safe: surviving replicas have the committed state).
5. Job liveness is maintained: preemption of one replica does not stall the overall job, satisfying Principle II ("automatic rescheduling, health checking, self-healing").

---

## 4. Governance: Public Good Job Eligibility

### 4.1 Public Good Review Board (PGRB)

| Constituency | Vote Weight | Selection |
|-|-|-|
| Active donors (weighted by 90-day NCU) | 40% | Direct participation |
| Technical steering committee | 30% | Elected, 5 members, 1-year terms |
| Independent ethics advisors | 20% | Appointed, rotating |
| Foundation / operators | 10% | Designated |

### 4.2 Eligibility Criteria

A job class qualifies as "public good" if it meets all of:

1. **Open access**: Results published openly within 12 months of cluster use.
2. **Non-harmful**: No surveillance, weapons research, biohazard design, or targeted data collection on individuals.
3. **Scientific validity**: Peer-reviewed project plan, or equivalent review by the technical steering committee.
4. **Opt-in alignment**: Donors who opted out of a job class (e.g., ML training) are excluded from having their hardware used for public good jobs of that class.

### 4.3 Approval Process

Standard track: 14-day public comment period → weighted vote → 60% approval threshold → annual re-approval.

Emergency track (humanitarian crisis response): 7-day review, 70% threshold.

Revocation: Any constituency bloc holding ≥ 30% can trigger a revocation vote; revocation requires simple majority.

---

## 5. Test Plan: Real Hardware Validation

### 5.1 Benchmark Normalization Tests

| Test | Method | Pass Criterion |
|-|-|-|
| FP32 GEMM throughput | Run on known hardware (RTX 3080, A100, RPi 4); compare to manufacturer spec | Within ±15% of spec |
| Benchmark replay attack | Submit pre-recorded benchmark results from different hardware | Server rejects mismatched fingerprint |
| Trust Score convergence | New node runs 100 WUs; measure credit weight growth | Weight reaches 90% of target within 30 days |
| Cross-validation rejection rate | Inject 5% malicious nodes with inflated benchmarks | Rejection rate ≥ 95% within 3 validation rounds |

### 5.2 Preemption Latency Tests

| Test | Method | Pass Criterion |
|-|-|-|
| Keyboard trigger latency | Measure wall time from keypress to SIGSTOP delivery under active GPU job | p99 < 100ms |
| CPU yield under load | 100% CPU load; measure time from SIGSTOP to CPU utilization < 5% | p99 < 200ms |
| GPU yield under load | Active CUDA kernel; measure time from signal to GPU utilization < 10% | p99 < 500ms |
| Memory reclaim | 8GB active allocation; measure time from SIGKILL to memory reclaim | p99 < 2s |
| Checkpoint on preempt | PyTorch training job; preempt at random step; restart; verify loss continuity | Resumed loss within 0.1% of uninterrupted run |
| Thermal trigger | Stress-test to 85°C; verify preemption fires | Preempts before sustained 90°C |

### 5.3 Fairness Over Time Tests

| Test | Method | Pass Criterion |
|-|-|-|
| Donor minimum guarantee | Donor with H100 submits 10 jobs; measure wait times | p95 queue time < 2h |
| Priority inversion prevention | Mix of all 5 job classes under 90% load; run for 24h | No DONOR_REDEMPTION job starved > 4h |
| Credit decay accuracy | Earn 1000 NCU; wait 45 days; verify balance | Balance = 500 ± 10 NCU |
| Caliber class enforcement | Class-4 donor requests class-4 node; verify scheduler never assigns class-3 without consent | 100% compliance |
| Paid job cannot crowd out donors | Fill cluster 80% with PAID_SPONSORED; submit DONOR_REDEMPTION job | DONOR_REDEMPTION job preempts PAID job within SLA |

---

## Summary of Key Design Decisions

[FINDING] The recommended scheduling hierarchy is LOCAL_USER > DONOR_REDEMPTION > PAID_SPONSORED > PUBLIC_GOOD > SELF_IMPROVEMENT, with DONOR_REDEMPTION carrying a hard contractual guarantee and SELF_IMPROVEMENT holding a permanently reserved capacity slice rather than competing in the priority queue.

[FINDING] The recommended credit unit is the Normalized Compute Unit (NCU), defined as 1 TFLOP/s FP32 for 1 second on a reference platform (A10G), with multi-dimensional accounting (compute, memory, storage, network) using DRF-style dominant-dimension consumption.

[FINDING] Hardware normalization should use server-side validated microbenchmarks with ±15% tolerance, caliber class tags (0–4) for the "same caliber" guarantee, and Trust Score weighting to resist benchmark gaming.

[FINDING] The preemption protocol should use SIGSTOP within 10ms of trigger detection, a 500ms graceful checkpoint window, then hard SIGKILL — meeting the constitutional sub-second yield target for interactive resources while allowing checkpointing for recovery.

[FINDING] Credit decay with a 45-day half-life, a minimum balance floor, and inflation monitoring addresses the credit inflation and hoarding failure modes observed in BOINC and SETI@home.

**Blockers / Open Questions**:

1. **GPU preemption granularity**: SIGSTOP does not preempt running CUDA kernels on some GPU drivers — a kernel launched before SIGSTOP may run to completion (typically < 10ms for small kernels, up to seconds for large ones). The spec needs a Stage 2 investigation into CUDA/ROCm preemption APIs (CUDA MPS, GPU time-slicing) to determine whether sub-second GPU yield is achievable without driver-level support.

2. **Quorum cost for fast jobs**: 3-way result quorum triples compute cost for small public-good jobs. A risk-tiered quorum policy (1-way for trusted nodes, 3-way for new/low-trust nodes) would reduce overhead but requires the Trust Score system from Stage 2 to be in place first.

3. **Caliber class for exotic hardware**: TPUs, FPGAs, and neuromorphic chips do not map cleanly to TFLOP/s FP32. A hardware taxonomy extension is needed before the cluster can ingest these node types.

4. **Credit portability on withdrawal**: Principle III guarantees donors can withdraw with no residual cluster state. The protocol for credit balance cashout or transfer on withdrawal is not yet specified.
