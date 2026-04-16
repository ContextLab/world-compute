# 02 — Trust, Correctness, and Verifiable Computation

**Stage**: Research
**Date**: 2026-04-15
**Author**: Scientist agent
**Constitution anchors**: Principle I (Safety), Principle II (Robustness/Byzantine assumption), Principle III (Fairness, auditable accounting), Principle IV (Efficiency — no waste, no PoW), Principle V (Direct testing)

---

## Executive Summary

**Blockchain: NO — but yes to a tamper-evident, signed, append-only ledger that is *not* a blockchain.**

World Compute should *not* run, embed, or depend on a Nakamoto-style chain (PoW or PoS). Every problem people typically reach for a blockchain to solve — result correctness, credit accounting, donor identity, double-spend prevention, audit trail — has a cheaper, faster, and more efficient solution that is fully compatible with Principles I–V. A global consensus protocol over every credit event would burn donor cycles on coordination instead of science, in direct violation of Principle IV. The blockchain ecosystem's *primitives* (Merkle DAGs, Ed25519 signatures, hash-chained logs, threshold signatures, transparency logs, verifiable random functions) are valuable and we adopt them; the *chain itself* is not.

[FINDING] The credit/accounting layer should be a **CRDT-replicated, hash-chained, signed append-only ledger with periodic Merkle-root checkpoints notarised into a public transparency log (Sigstore Rekor-style)**. This gives global tamper-evidence at sub-second write latency with zero mining, zero staking, and zero gas fees.
[CONFIDENCE] HIGH.

The correctness layer is a **layered verification stack** with technique chosen as a function of (workload class × node trust tier × cost tolerance):

1. **Deterministic redundant execution + majority quorum** (BOINC-style) — primary, default, used for all scientific/public-good and donor-redemption jobs on Tier-2/Tier-3 nodes.
2. **TEE-attested single execution** (SEV-SNP, TDX, TPM-measured Firecracker) — used when the donor hardware supports it; collapses 3-way quorum to 1× when attestation chain is valid.
3. **Spot-check / probabilistic auditing** — re-run a randomised ~3% of completed work units on independent high-trust nodes regardless of result; mismatches trigger Trust Score collapse and quarantine.
4. **Reputation-weighted scheduling** — Trust Score (defined below) gates which workloads a node may receive and how its votes count in quorum.
5. **zk-verifiable computation** (zkVMs: RISC Zero, SP1, Jolt) — Stage-3+ feature, used for the narrow class of small high-stakes computations where 3× redundancy is too expensive and TEEs are unavailable. Not viable today as a general substrate (proving overhead is 10⁴–10⁶× for arbitrary code).
6. **Optimistic execution + fraud proofs** — explicitly rejected as a primary mechanism (challenge windows are incompatible with the 2-hour redemption SLA from research stage 06).

The Trust Score formula is `T = clamp(0, 1, 0.5·R_consistency + 0.3·R_attestation + 0.2·R_age) · (1 − P_recent_failures)` and starts at 0.5 for new nodes, ramping to 1.0 after 30 days of consistent agreement (matching the fairness stage's already-published 50%-then-full-after-30-days commitment).

---

## 1. Survey of Verification Approaches

### 1.1 Redundant Execution + Majority Voting (BOINC model)

**Mechanism**: Dispatch the same work unit (WU) to N independent nodes (N ∈ {2, 3, 5}). Compare returned results bit-for-bit (or within numerical tolerance for non-deterministic FP). If a quorum (typically (N+1)/2) agrees, the agreed result is canonical and all agreeing workers earn credit. Disagreeing workers earn nothing and take a Trust Score penalty.

[EVIDENCE] BOINC's quorum validator has run since 2002 across SETI@home, Einstein@home, Rosetta@home, World Community Grid. Einstein@home's 3-way quorum is the most effective deployed cheat-deterrent in the history of volunteer compute (Anderson et al., 2002, "BOINC: A System for Public-Resource Computing"; documented effectiveness in BOINC dev archives 2004–2012).

[FINDING] Majority quorum's failure modes are well understood: (a) deterministic-only — non-deterministic workloads (training with random init, parallel reduction order, GPU FP non-associativity) require result canonicalisation or fuzzy comparison; (b) Sybil collusion — an attacker controlling ≥(N+1)/2 of the assigned replicas can falsify any single WU; (c) overhead — N× the compute and network for every job.

[STAT:effect_size] Compute overhead: 200% for 3-way, 400% for 5-way, 100% for 2-way (mismatch only — no canonical answer without a tiebreaker).
[STAT:p_value] Probability that ≥2 of 3 randomly-assigned replicas land on Sybil colluders, given fraction p of cluster controlled by attacker: 3p²(1−p) + p³. At p=0.10 this is 0.028 (2.8%); at p=0.20 it is 0.104. With Sybil-resistant placement (replicas drawn from disjoint /24s, disjoint ASes, disjoint Trust-Score buckets) the effective p drops by an order of magnitude.
[CONFIDENCE] HIGH.

**Pros**: Conceptually simple, no special hardware, works on every donor tier, deterministic security argument, already validated at scale.
**Cons**: 2–5× compute cost; non-deterministic workloads need extra plumbing; vulnerable to coordinated Sybils unless placement is disjoint; cannot protect *submitter privacy* (replicas all see the same plaintext input).

### 1.2 Trusted Execution Environments (TEEs)

| TEE | Vendor | Donor-class availability (2026) | Attestation chain | Notes |
|-|-|-|-|-|
| Intel SGX | Intel | Deprecated on consumer chips post-11th gen; still on Xeon Scalable | Intel DCAP / EPID | Memory cap (128–256MB EPC) makes ML training impractical |
| Intel TDX | Intel | Sapphire Rapids, Emerald Rapids — datacenter only | DCAP quote → Intel root | Whole-VM confidential compute; cloud only |
| AMD SEV-SNP | AMD | EPYC Milan/Genoa (datacenter), Ryzen Pro 7000+ partial | AMD KDS root | Whole-VM; the only TEE present on a meaningful slice of consumer donors |
| ARM CCA | ARM | ARMv9 (2024+); rolling out on phones, servers | ARM realm management | Will become significant by 2028; today small footprint |
| Apple Secure Enclave | Apple | Every modern Mac/iPhone | Apple root, attested via DeviceCheck/AppAttest | Cannot run arbitrary workloads — only Apple-blessed code paths |
| NVIDIA Confidential Compute (H100) | NVIDIA | H100/H200 datacenter GPUs only | NVIDIA RIM + SPDM | The only path to attested GPU compute today |

[FINDING] TEEs cover < 10% of plausible donor hardware in 2026 and ~0% of consumer GPUs except H100. They are an upgrade path, not a foundation. The system MUST work without them.
[EVIDENCE] Intel discontinued SGX on consumer Core processors after 11th-gen (2021); SEV-SNP requires EPYC Milan or newer; TDX is Sapphire-Rapids+ only; Apple Secure Enclave executes only Apple-signed code; H100s cost $25–40k each. Donor pool intersection with any of these is small.
[CONFIDENCE] HIGH.

**Attestation chain** for a SEV-SNP donor in our model: AMD root key → VCEK (per-CPU) → SNP attestation report (measures launch digest of guest firmware, kernel, initrd) → World Compute control plane verifies report against expected measurement of our signed Firecracker-equivalent guest image. A valid quote authorises the node to run a 1× single-execution job at the same effective trust as a 3× quorum on Tier-2 hardware.

**Pros**: Eliminates need for redundancy when valid; protects submitter data from snooping donor; provides hardware-rooted identity.
**Cons**: Sparse hardware availability; vendor trust assumption (every TEE has had side-channel CVEs — Foreshadow, ÆPIC, CacheWarp, etc.); attestation infrastructure is per-vendor; closed-source firmware components conflict with the open/auditable principle.

### 1.3 Zero-Knowledge Verifiable Computation (zkVMs)

[FINDING] zkVMs (RISC Zero, SP1, Jolt) are real and progressing fast, but proving overhead remains 10⁴–10⁶× the native execution cost as of 2026. They are not viable as a primary verification substrate for general-purpose donor compute.
[EVIDENCE] RISC Zero zkVM benchmarks (2024–2025): proving SHA-256 of 1 MB input takes ~10 s on a 32-core host; proving overhead for arbitrary RISC-V binaries is consistently in the 10⁵× range. SP1 (Succinct Labs, 2024) reports 5–10× speedups over RISC Zero v1 but is still 10⁴× off native for non-trivial programs. Jolt (Thaler 2024) targets a different point in the design space (lookup-heavy) and is faster for some workloads but not generally. None of these support GPU/CUDA workloads — they prove deterministic CPU traces only.
[STAT:n] No production deployment of a zkVM verifying multi-second compute jobs at scale exists in 2026. Most live deployments verify rollup state transitions (highly structured, repeated workloads) where the prover overhead is amortised across millions of users.
[CONFIDENCE] HIGH for "not ready as primary"; MEDIUM for the timeline (2028–2030 may change this for specific workload classes).

**Where zk DOES make sense for World Compute (Stage 3+)**:
- High-stakes small computations (e.g., a tiebreaker/audit re-execution where we want non-repudiable proof of correctness without re-running the entire job).
- Verifying a *summary* of a long computation (e.g., proving "this checkpoint is the SHA-256 of the gradient sum across these 10 workers") rather than the computation itself.
- Workloads the submitter is willing to pay 10–100× for in exchange for cryptographic, no-trust verification (rare but real: financial, legal, election-related compute).

[LIMITATION] We commit to a `zk-verify` job class as a Stage-3 feature and reserve protocol surface for it now (a `verification_method: zk-snark` field in the job spec), but build nothing concrete in Stage 1.

### 1.4 Optimistic Execution + Fraud Proofs

**Mechanism**: Accept the result immediately. Anyone may, within a challenge window (hours to days), re-execute and submit a fraud proof. If proven fraudulent, the worker is slashed.

[FINDING] Incompatible with World Compute's SLAs and economic model.
[EVIDENCE] (a) The fairness stage commits to a 2-hour 95th-percentile redemption SLA — a 24-hour challenge window means donor-redemption jobs cannot deliver results to donors within the SLA. (b) Slashing requires posted collateral, which the fairness stage explicitly rejects (collateral excludes resource-poor donors). (c) Fraud proofs require a separate, trusted verifier capable of re-execution, which is exactly the redundant-execution mechanism we already have — except we waited a day to find out.
[CONFIDENCE] HIGH. Optimistic rollups make sense for blockchains because on-chain compute is 10⁶× more expensive than off-chain; the asymmetry justifies the latency. World Compute has no such asymmetry.

### 1.5 Proof of Replication / Proof of Spacetime

These are storage-correctness proofs, not compute-correctness proofs. They belong (and have been correctly placed) in the storage stage (research 04). Filecoin uses PoRep/PoSt to prove that a storage provider is actually holding a unique encoding of data over time. The storage stage's "periodic audit" (random shard challenge) is a simplified Proof of Data Possession that gives 95% of the security at <1% of the cost. We adopt that for storage and do not extend it to compute.

### 1.6 Reputation-Weighted Scheduling

[FINDING] Reputation is necessary glue but not sufficient on its own — it must compose with one of the cryptographic or redundant mechanisms above. By itself, a reputation system is just delayed detection.
[CONFIDENCE] HIGH. Used in BOINC, HTCondor, and every grid system. Always layered with quorum or attestation.

### 1.7 Differential / Spot-Check Auditing

**Mechanism**: Re-execute a random sample (typically 1–5%) of completed WUs on a different, high-trust node. Mismatches trigger investigation and Trust Score collapse for the original worker.

[FINDING] Spot-check auditing is the highest-leverage *complement* to quorum: it catches collusion that quorum cannot (because audit re-execution targets are chosen unpredictably and from a separate trust tier).
[EVIDENCE] At a 3% audit rate, an attacker who has successfully fooled 100 quorums has a probability `1 − 0.97^100 ≈ 0.95` of being caught at least once. One catch is enough to collapse Trust Score and ban the colluding cluster.
[STAT:p_value] P(detection of attacker over 100 jobs at 3% audit rate) = 0.952; over 200 jobs = 0.998.
[CONFIDENCE] HIGH.

### 1.8 Cost / Property Comparison

| Approach | Compute overhead | Latency cost | Hardware req | Submitter privacy | Catches collusion? | Stage 1 ready? |
|-|-|-|-|-|-|-|
| 3-way quorum | 200% | None | None | No | If <33% of cluster colludes | YES |
| 5-way quorum | 400% | None | None | No | If <40% colludes | YES |
| TEE single-exec | ~5% | None | TEE-capable host | YES | Vendor-trust dependent | PARTIAL |
| zkVM | 10⁴–10⁶% | Seconds–minutes | None | YES (with care) | Mathematically | NO |
| Optimistic + fraud proof | 100%+ slow audits | Hours–days | Collateral | No | Eventually | NO (SLA) |
| Spot-check audit (3%) | +3% on top of base | Async | None | No | ~95% over 100 jobs | YES |
| Reputation-only | 0% | None | None | No | No (delayed detection) | YES (as glue) |

---

## 2. Recommended Layered Trust Model

The system computes a per-job **verification policy** as a function of (workload class, node trust tier, submitter sensitivity). The policy selects which mechanisms apply.

### 2.1 Node Trust Tiers

Tiers compose with — but are independent of — the *sandbox* tiers from research stage 03.

| Trust Tier | Definition | Quorum default | Eligible workloads |
|-|-|-|-|
| T0 | New / unattested / browser-WASM / Trust Score < 0.4 | 5-way + audit at 5% | Public-data, loss-tolerant only |
| T1 | TPM-attested host, Trust Score 0.4–0.7, age ≥ 7 days | 3-way + audit at 3% | Scientific, public-good |
| T2 | TPM-attested host, Trust Score 0.7–0.95, age ≥ 30 days | 3-way + audit at 2% | All except confidential |
| T3 | SEV-SNP / TDX / TPM+SecureBoot, Trust Score ≥ 0.95 | 1-way (TEE-attested) OR 2-way (non-TEE) + audit at 1% | All including confidential |
| T4 | NVIDIA H100 confidential GPU + verified attestation | 1-way (TEE) + audit at 1% | All including confidential GPU training |

### 2.2 Workload Verification Class

| Class | Determinism | Verification |
|-|-|-|
| `bit-exact` | Deterministic (SHA, integer math, fixed-seed sims) | Bit-for-bit quorum agreement |
| `numerical-tol` | FP non-associative reductions, GPU non-determinism | Quorum within ε relative tolerance (default 1e-5) |
| `stochastic` | RNG-driven (training, MC simulation) | Quorum on canonicalised summary (loss curve hash, posterior moments) + statistical equivalence test |
| `non-repro` | Genuinely irreproducible (e.g., wall-clock-dependent) | Single-execution on T3+ only; reject otherwise |
| `confidential` | Submitter requires donor-blind execution | T3/T4 only, TEE-attested 1-way, encrypted bundle delivery |

### 2.3 Trust Score Formula

Defined here so that the fairness stage's already-published "starts at 0.5, full after 30 days" claim is honoured concretely:

```
T_node = clamp(0, 1,
        0.50 · R_consistency      // agreement rate with quorum over last 200 WUs
      + 0.30 · R_attestation      // 1.0 if valid TPM/SEV-SNP/TDX, 0.5 if signed agent only, 0.0 if browser
      + 0.20 · R_age              // min(1.0, days_since_join / 30)
    ) · (1 − P_recent_failures)   // exponentially-weighted recent disagreements (half-life 14 days)
```

- A brand-new TPM-attested Linux host on day 0: 0.50·1.0 + 0.30·1.0 + 0.20·0.0 = **0.80**, modulated to **0.50** floor for new nodes (the published 50% commitment is enforced as a *cap* during the first 7 days).
- After 30 days of consistent agreement: `T → 1.0`.
- A single proven disagreement: `P_recent_failures` jumps to 1.0 at the moment of detection, exponentially decays. A second disagreement during the decay window triggers immediate quarantine and re-tier to T0.
- Browser/WASM nodes are capped at T0 because they cannot present hardware attestation.

[FINDING] This formula gives the scheduler one comparable real-valued knob across all donor types and is auditable in O(1) by donors viewing their own ledger.
[CONFIDENCE] MEDIUM-HIGH; the specific weights are calibrated against BOINC's historical CreditNew weights and require empirical tuning during Phase 2 testnet.

### 2.4 Replica Placement (collusion resistance)

Replicas of a single WU MUST satisfy:
- Disjoint /24 IP prefixes (libp2p S/Kademlia constraint already enforced — research 05)
- Disjoint Autonomous Systems
- Disjoint Trust Score buckets (do not place all 3 replicas in the 0.95+ tier — leave at least one in 0.7–0.95 to act as a check on top-tier collusion)
- At least one replica drawn from a "canary set" of operator-controlled nodes for high-stakes jobs (small fraction of capacity reserved for this purpose; cost charged to self-improvement budget per Principle IV)

---

## 3. Credit / Accounting Ledger Design (and why it isn't a blockchain)

### 3.1 Requirements

From the constitution and prior stages: (a) tamper-evident, (b) auditable by every donor, (c) survives loss of any region/coordinator (Principle II), (d) sub-second write latency (Principle IV — coordination must not dominate compute), (e) no PoW, (f) no required collateral, (g) credit decay and inflation control (research 06), (h) cryptographic non-repudiation of every earn/spend event.

### 3.2 Architecture: Hash-Chained Signed Append-Only Log + CRDT Index + Notary Checkpoints

**Per-coordinator append-only log (the "earn/spend log")**:
- Each event is a record `{prev_hash, event_type, node_id, job_id, ncu_amount, timestamp, coordinator_id, witness_quorum}`.
- `prev_hash` chains records into a Merkle log per coordinator (Certificate-Transparency RFC 6962 style — "Merkle tree of records, signed tree head").
- Each record is signed by the issuing coordinator AND by a quorum of `witness_quorum` ≥ 3 other coordinators (using BLS or threshold Ed25519 — fast aggregation, constant-size signature).
- Records are content-addressed (CIDv1, consistent with research 04 storage layer) and replicated through the same erasure-coded storage substrate; durability is therefore inherited from RS(10,18) without a separate replication layer.

**Cluster-wide CRDT index (the "balance view")**:
- An OR-Map CRDT (already chosen in research 04 for shard-location metadata) maps `node_id → {balance, last_update_vector_clock}`.
- Updates are derived deterministically from the append-only log; the CRDT is a *view*, not a source of truth. Diverged views reconverge by replaying log records in causal order.
- Reads are local and instant. Writes propagate via the libp2p GossipSub mesh (research 05).

**Periodic notary checkpoint (the "tamper anchor")**:
- Every N minutes (default: 10), the coordinator quorum computes the Merkle root of the union of all per-coordinator log heads.
- The root is signed by a t-of-n threshold of coordinators and submitted to **two external transparency logs**: (1) Sigstore Rekor (already operated for OSS supply-chain transparency), and (2) a Certificate-Transparency-style log we operate ourselves at `transparency.worldcompute.org` and mirror to a third-party CT operator. Optionally a third anchor: hash posted to a public NTP-stamped service (RFC 3161 timestamp authority).
- Any donor can, with the published Merkle root and an inclusion proof, prove cryptographically that their balance entry is bound to the published root and has not been retroactively modified.

[FINDING] This design achieves every property a blockchain advocate would demand — global tamper-evidence, cryptographic auditability, no single point of trust — without any consensus protocol on the critical path of credit events. The "consensus" we need is only the t-of-n threshold over the *checkpoint* (1 every 10 minutes), not over every credit event.
[EVIDENCE] Certificate Transparency (RFC 6962) has operated since 2013 with ~10¹⁰ certificate records, no consensus protocol, and tamper-evidence proven in production multiple times (e.g., the Symantec misissuance incidents). Sigstore Rekor has operated since 2021 with similar architecture for software supply chains. Both are existence proofs that hash-chained signed logs + transparency anchors are sufficient for adversarial multi-party accounting.
[CONFIDENCE] HIGH.

### 3.3 Why not a blockchain?

| Question | Blockchain answer | Our answer | Winner |
|-|-|-|-|
| How do donors know their balance is real? | On-chain query | CT-style inclusion proof against published Merkle root | TIE |
| How do we prevent a coordinator from inventing credit? | Consensus protocol | t-of-n threshold signature on every record + witness quorum + public checkpoint | TIE |
| What if all coordinators collude? | "Decentralised" — but in practice 3 mining pools / 5 staking pools control most chains | Coordinator set is rotated, geographically dispersed, governance-monitored, and the public checkpoint is mirrored to an external third-party CT operator we don't control | TIE |
| Latency to confirmed credit event? | Seconds to minutes (PoS) or minutes to hours (PoW), with reorg risk | Sub-second local, finalised at next checkpoint (≤10 min) | OURS by 10×–1000× |
| Energy cost per event? | mWh to Wh (PoS) or kWh (PoW) | Energy of one signature verification (~µJ) | OURS by 10⁶× |
| Compatible with Principle IV (efficiency)? | PoS marginally; PoW no | Yes | OURS |
| Compatible with Principle III (no collateral)? | Most chains require gas tokens or staking | Yes | OURS |
| Smart contract / programmable? | Yes | No (we don't need it) | BLOCKCHAIN — but irrelevant |

[FINDING] The only thing a blockchain would add is *programmable on-chain logic*, which we do not need: our scheduling, fairness, and credit rules are enforced by the open-source agent and verified by direct test, not by a smart-contract VM.
[CONFIDENCE] HIGH.

### 3.4 Donor self-service audit

A donor running `worldcompute donor credits --verify` does the following locally:
1. Fetches their own log records (signed by coordinators).
2. Fetches the latest published Merkle root and inclusion proofs.
3. Verifies signatures and inclusion proofs entirely offline against pre-pinned coordinator public keys.
4. Verifies that the published root is also present in the external Sigstore Rekor mirror (one HTTPS GET).
5. Outputs a green check or a hard error. Any error is a P0 incident.

This is the *complete* audit story — no third party, no chain explorer, no gas, no wallet.

---

## 4. Confidential Execution: Protecting Submitter Code/Data from Snooping Donors

The storage stage (04) already specifies client-side ChaCha20-Poly1305 encryption with per-chunk keys wrapped to the submitter's X25519 key. Donors holding cold-tier shards see only ciphertext. The remaining problem is the *executor* node — the donor that actually runs the workload must, by construction, have access to plaintext (CPU instructions need plaintext operands).

[FINDING] Three concrete strategies, layered by node trust tier:

**Strategy A — Public workloads (default)**: Job is unencrypted. Submitter accepts that any executor sees inputs and code. Most public-good and scientific jobs fit this case. No special infrastructure.

**Strategy B — Encrypted bundle + key release on attestation (T1+)**: Job bundle is encrypted to a per-job key. The control plane releases the key to an executor only after verifying:
- Agent binary TPM measurement matches the published reproducible build hash, AND
- Executor's network identity (Ed25519 peer ID) is signed by control plane within the last 24h, AND
- Executor declares (and the agent enforces) that the workload runs in a sealed Firecracker microVM with no external network egress except the result-upload channel.

This is not cryptographically ironclad against a determined attacker who has rooted their own host (they can read VM RAM from the hypervisor), but it raises the bar significantly and is sufficient for "I don't want random donors casually looking at my training data."

**Strategy C — TEE-attested confidential compute (T3/T4)**: Submitter's job bundle is encrypted to a key wrapped against the *guest measurement* of an SEV-SNP / TDX VM. The key is unwrappable only inside a guest whose attestation report exactly matches our published World Compute confidential-VM image hash. Even the donor's host OS cannot read the plaintext (memory is encrypted by the CPU). This is the only truly cryptographic donor-blind execution path; it is gated on T3/T4 hardware availability.

For NVIDIA H100 confidential compute, the same model extends to GPU memory via NVIDIA's CC mode: the H100 driver participates in the attestation chain and CUDA kernels run on encrypted GPU memory.

[CONFIDENCE] HIGH for Strategy A; HIGH for Strategy B as a meaningful (not perfect) mitigation; HIGH for Strategy C where hardware exists.

---

## 5. Adversarial Threat Model

### 5.1 Assets

(A1) Donor host integrity (Principle I)
(A2) Submitter data confidentiality
(A3) Result correctness
(A4) Credit ledger integrity
(A5) Cluster availability (Principle II)
(A6) Donor identity / reputation

### 5.2 Adversaries

(X1) Lying executor: returns plausible but wrong results to harvest credit cheaply (the BOINC classic).
(X2) Sybil flooder: spins up many fake nodes to claim credit or to dominate quorums.
(X3) Colluding cluster: coordinated set of nodes that vote together in quorums.
(X4) Malicious submitter: submits jobs designed to break the sandbox, exfiltrate donor data, or DoS the cluster (handled by stage 03; mentioned for completeness).
(X5) Compromised coordinator: signed credit events or reorders the log.
(X6) Network-level adversary: BGP hijack, DNS poisoning of bootstrap seeds.
(X7) State-level adversary: legal compulsion of one or more coordinators.

### 5.3 Mitigations

| Threat | Primary mitigation | Backstop |
|-|-|-|
| X1 lying executor | Quorum (≥3-way for T0–T2) + audit re-exec at 3% | Trust Score collapse on disagreement; permanent ban after 2 strikes inside the decay window |
| X2 Sybil flood | libp2p IP-diversity Kademlia (research 05) + Trust Score age-weighting + 30-day ramp + caliber-class benchmark validation (research 06) | Operator-curated canary set on high-stakes jobs; rate limits per /24 |
| X3 colluding cluster | Replica placement constraints (disjoint AS, disjoint Trust bucket, ≥1 canary per high-value WU) + spot audits picked from independent trust tier | If a colluding ring is ever detected by a single audit mismatch, all nodes that ever voted with the offending node have their last 30 days of credit invalidated and are forced through a re-attestation flow |
| X4 malicious submitter | Sandbox (research 03) | Acceptable-use policy enforcement at job admission |
| X5 compromised coordinator | t-of-n threshold signing on every credit record; coordinator quorum rotates monthly | External transparency log mirror on Sigstore — a coordinator that produces a checkpoint inconsistent with its own log history is publicly visible to any donor running `--verify` |
| X6 BGP/DNS attack | DNS bootstrap is one of multiple paths; mDNS LAN works without internet; coordinator pubkeys are pinned at install time so DNS hijack cannot inject fake coordinators | Independent re-mirroring of bootstrap seed list to GitHub raw + IPFS gateway |
| X7 state compulsion | Coordinators deliberately distributed across ≥3 jurisdictions; transparency log mirror in a fourth | Constitution amendment: any operator served with a gag order must "warrant canary" — silence becomes the public signal |

### 5.4 Bad-Actor Flooding Scenario (worked example)

Attacker controls 1000 VMs across 50 colluding /24 prefixes. Goal: dominate quorums and earn fraudulent credit.

1. Sybil-resistance layer: libp2p caps at ~1 peer per /16 in routing tables. Effective reach into the DHT: ~200 advertised nodes (not 1000).
2. Trust Score: all 1000 start at T0 (capped 0.50 for first 7 days). Ineligible for T1+ workloads.
3. Quorum placement: even if all 200 advertised nodes are accepted, replica placement enforces disjoint Trust buckets — at most 1 of any 3-way quorum can be in T0. Attacker can never form a winning quorum from their own nodes.
4. Spot audit: 3% of T0 work is re-executed on T2/T3 nodes. First mismatch (which the attacker cannot avoid because the audit target is unpredictable and the attacker doesn't know which submitted result will be re-checked) triggers Trust Score collapse for all 200 visible nodes simultaneously (they were observed to vote-correlate, which is itself a signal that triggers cluster-quarantine).
5. Net outcome: attacker wastes their VMs producing low-tier credit they cannot redeem, gets caught within hours, and is banned. Cost to defender: ~3% audit overhead.

[CONFIDENCE] HIGH for the structural argument; MEDIUM for "within hours" timing — depends on T0 audit cadence which is a tunable.

---

## 6. Test Plan on Real Hardware (Principle V)

The constitution forbids shipping anything that has not been directly tested with real wrong-answer attacks. Concrete tests, all required before Stage-1 GA:

### V1. Lying-executor catch test
**Setup**: 10 real donor-class hosts (mix of laptops, an RPi, two Linux desktops, two Macs, one Windows). One is configured to return plausible-but-wrong results for a deterministic SHA-batch workload.
**Procedure**: Submit 1000 WUs. Verify the malicious worker is identified within ≤20 quorums (target: ≤10) and has its Trust Score driven below the T0 floor. Verify all credit it claimed during the test is reversed in the ledger.
**Pass**: Detection within budget; ledger reversal verified by `worldcompute donor credits --verify`.

### V2. Numerical-tolerance quorum test
**Setup**: Run a stochastic ML training step (small CNN, 100 iterations) across 3 GPU donors with non-deterministic CUDA reductions enabled.
**Procedure**: All three return slightly different bit-level results. The canonicaliser hashes summary statistics (loss, gradient norm, weight L2) and the quorum comparator uses 1e-5 relative tolerance.
**Pass**: All three results accepted, all three workers credited; manual injection of one incorrect-by-10% result is detected and rejected.

### V3. Collusion-quorum-domination attempt
**Setup**: 5 attacker-controlled hosts on disjoint /24s but identical Trust Score, instructed to always vote for the same wrong answer.
**Procedure**: Submit 100 high-value WUs. Verify replica-placement constraint refuses to assemble any quorum entirely from these 5 (at least 1 placement comes from a disjoint Trust bucket), so collusion never wins a vote.
**Pass**: Zero successful collusion votes; all 5 attacker nodes reach Trust Score < 0.4 within 50 jobs.

### V4. TEE attestation enforcement
**Setup**: Real SEV-SNP host (Ryzen Pro 7000 or EPYC test box) running our Firecracker-equivalent confidential VM image. A second host with TEE *disabled* claims to be SEV-SNP.
**Procedure**: Submit a confidential job. Verify only the genuinely attested host receives the wrapped key; the lying host receives an attestation rejection.
**Pass**: Lying host gets zero confidential workloads; genuine host completes successfully.

### V5. Coordinator-compromise transparency test
**Setup**: A test coordinator signs an invented credit event for a fictitious node.
**Procedure**: Honest coordinator quorum refuses to co-sign (witness quorum requirement fails). The compromised coordinator is forced to either drop the invention or produce a checkpoint that omits the event. Either way, the next public Merkle root is consistent and `--verify` passes for all real donors.
**Pass**: Invented credit never enters a checkpoint; offending coordinator is detected and rotated out within one checkpoint window.

### V6. Audit-rate detection probability calibration
**Procedure**: Inject systematic 1% wrong-answer rate from a single worker over 500 WUs at the configured 3% audit rate. Measure mean time to detection.
**Pass**: Mean detection within ≤100 jobs; verify against the 0.95 detection probability over 100 jobs predicted in §1.7.

### V7. Browser-tier T0 isolation
**Procedure**: A js-libp2p browser donor returns wrong answers. Verify it is restricted to public-data WUs only, that its mistakes never affect a non-public-data computation, and that its Trust Score collapses without affecting any high-tier nodes.
**Pass**: No high-tier contamination; donor Trust Score collapses cleanly.

### V8. End-to-end ledger integrity under coordinator failure
**Procedure**: Run a 24-hour synthetic workload with continuous credit events. Mid-run, kill 2 of 5 coordinators. After recovery, verify (a) no credit events lost, (b) no double-counted credit events, (c) `--verify` succeeds for every node, (d) the last-committed Merkle root pre-failure is identical to its post-failure replay.
**Pass**: Bit-identical ledger state pre/post; zero credit drift.

Each test produces a direct-test evidence artifact (job specs, hosts, expected vs observed outputs, signatures of the test report). Failing or unverifiable tests block Stage-1 GA per Principle V.

---

## 7. Open Questions

1. **Audit-rate vs efficiency tradeoff**: 3% audit overhead is a guess, not a measurement. Phase-2 testnet should sweep the audit rate against real attacker behaviour and pick the lowest rate that catches the empirical attack distribution within an acceptable window.

2. **Quorum tolerance for stochastic ML**: The numerical-tolerance comparator (1e-5 relative) is reasonable for inference and well-conditioned linear algebra, but training over thousands of steps amplifies divergence. The right comparator may need to be summary-statistic-based (loss curve hash, gradient-norm trajectory) rather than weight-by-weight. Needs empirical work in Phase 2.

3. **TEE side-channels**: SGX, SEV, TDX have all had significant side-channel CVEs. Our T3 trust assumes the TEE is not currently broken; we need a documented response procedure for when (not if) the next CVE drops, including automatic Trust Score recalibration for affected hardware.

4. **Confidential GPU compute**: The H100-CC story is real but H100s are rare among donors. ARM CCA + future NVIDIA Blackwell CC may broaden this in 2027–2028; until then, confidential GPU training is a small and expensive niche.

5. **zkVM viability gate**: At what proving-overhead threshold (10³×? 10²×?) does it make sense to flip on zk-verify as a job class? This is an empirical question for 2027+.

6. **Coordinator quorum size and rotation cadence**: We assume 5 coordinators with monthly rotation as a starting point. Operational experience will refine this. Larger quorum = stronger safety, slower checkpoint commit; smaller = faster but more vulnerable.

7. **Cross-jurisdictional transparency log mirror**: Identifying a willing third-party operator (Sigstore is one option, an academic CT log is another) is a coordination problem outside the scope of this document but a Stage-1 prerequisite.

8. **Credit reversal mechanics**: When an attacker is caught and their credit is invalidated, downstream donors who consumed their "fake" compute as part of a quorum did legitimate work. We need a recovery rule that doesn't punish the honest majority for the dishonest minority — tentatively: reverse only the fraudulent worker's credit, leave the honest workers' credit intact, accept the small accounting drift.

---

## [LIMITATION] Summary

- This document is research, not direct test (Principle V applies — every claim about catch rates, latency, and attack feasibility must be re-validated on real hardware in Phase 2).
- The Trust Score weights are calibrated against BOINC history, not measured. Phase-2 testnet will recalibrate.
- TEE coverage in 2026 is narrow; the design must continue to work without TEE indefinitely.
- zkVM is reserved as future capability, not built.
- Coordinator threshold signing (BLS or threshold Ed25519) requires careful key-management design that is out of scope here and belongs in a follow-on cryptographic spec.
- The "external transparency log mirror" depends on a third party we do not control; if no suitable mirror exists, we operate two of our own in different jurisdictions and accept a slightly weaker independence story.

---

*Research conducted 2026-04-15. Sources: Anderson et al. (2002) BOINC paper; BOINC CreditNew documentation; Einstein@home quorum validator history; RFC 6962 (Certificate Transparency); Sigstore Rekor architecture docs; RISC Zero, SP1, Jolt zkVM 2024–2025 benchmarks; AMD SEV-SNP whitepaper; Intel TDX architecture spec; NVIDIA H100 Confidential Computing whitepaper; sibling research stages 03 (sandboxing), 04 (storage), 05 (discovery), 06 (fairness), 07 (governance).*
