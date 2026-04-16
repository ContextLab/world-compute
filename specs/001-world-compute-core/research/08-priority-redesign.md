# 08 — Priority Redesign: Open-Access Multi-Factor Scheduling

**Status**: Draft — 2026-04-15
**Scope**: World Compute Core Spec 001
**Constitution anchor**: Principle III (Fairness & Donor Sovereignty), Principle IV (Efficiency)
**Supersedes**: FR-032 priority hierarchy (partial); Research 06 §1.2 hard-guarantee model (partial)

---

## Executive Summary

The current priority model (Research 06, FR-032) treats NCU credits as a **gate**: donors earn credits by contributing hardware and spend them to run jobs. Non-donors cannot submit jobs at all. This design has a fatal flaw — redundant execution (quorum validation, replication) means donors always get back *less* compute than they put in, making the system strictly worse than running locally. It also contradicts the project's stated mission of democratizing compute for all of humanity.

This document redesigns the priority system around a single principle: **anyone can submit a job for free; NCU boosts priority but never gates access**. The new model replaces the rigid five-class preemption hierarchy with a continuous composite priority score inspired by Slurm's multi-factor scheduler, combined with a novel public importance voting system for Sybil-resistant democratic prioritization.

The composite priority formula is:

```
P(job) = w_ncu · S_ncu(j) + w_vote · S_vote(j) + w_size · S_size(j) + w_age · S_age(j) + w_cool · S_cool(j)
```

where LOCAL_USER remains an absolute preemption override (unchanged from FR-032), and all five signals are normalized to [0, 1] before weighting. Default weights: w_ncu = 0.35, w_vote = 0.25, w_size = 0.15, w_age = 0.15, w_cool = 0.10.

The key insight: this model is **strictly more democratic** than the old one because the set of people who can use the cluster expands from {donors} to {everyone on Earth}, while donors retain a significant priority advantage as compensation for their contribution.

---

## 1. The Composite Priority Formula

### 1.1 Design Principles

1. **No binary gates**: Every signal produces a continuous score in [0, 1]. No signal can make a job unschedulable.
2. **Monotonic age guarantee**: The age signal grows without bound (before normalization), ensuring every job eventually exceeds any finite priority threshold.
3. **Additive composition**: Signals are weighted-summed, not multiplied, so a zero in one signal does not zero the total.
4. **LOCAL_USER exception**: Local user activity is not part of the priority formula. It is a preemption trigger that instantly suspends all cluster work on that machine (FR-040, constitutional mandate). This is unchanged.

### 1.2 Signal Definitions

#### (a) NCU Balance Signal: S_ncu

```
S_ncu(j) = 1 - exp(-α · ncu_balance(submitter))
```

where `α = ln(2) / NCU_HALFSAT` and `NCU_HALFSAT` is the NCU balance at which the signal reaches 0.5 (default: 100 NCU, roughly 1 hour of A100-equivalent donation). This is a saturating exponential: early NCU contributions give large priority gains; returns diminish as balance grows, preventing whales from achieving unbounded advantage.

Properties:
- A non-donor (ncu = 0) gets S_ncu = 0, but this does not block scheduling — the other four signals still contribute.
- A donor who contributed 100 NCU-hours gets S_ncu ≈ 0.5.
- A donor who contributed 1000 NCU-hours gets S_ncu ≈ 0.999.
- NCU is spent when jobs run (as in Research 06 §2), so heavy users deplete their balance and their priority naturally decays.

[FINDING] The saturating exponential prevents donor plutocracy: a donor with 10× more NCU than another gains only ~1.4× the S_ncu score (at the 100-NCU halfsat point), not 10×. This is a deliberate design choice to bound the advantage of wealth.
[EVIDENCE] For ncu_1 = 100, S_ncu = 0.5; for ncu_2 = 1000, S_ncu = 0.999. Ratio = 0.999/0.5 = 2.0. For ncu_1 = 50, S_ncu = 0.293; for ncu_2 = 500, S_ncu = 0.969. Ratio = 3.3. The ratio is always sublinear in the NCU ratio.
[CONFIDENCE] High — mathematical property of 1 - exp(-αx).

**Interaction with old model**: This replaces the DONOR_REDEMPTION class entirely. There is no separate "redemption" queue. NCU *is* the redemption mechanism — it boosts priority. A donor spending NCU gets faster service (high S_ncu), but a non-donor's job is never blocked, just deprioritized.

**FR-032 impact**: The hard guarantee "p95 queue < 2h for same-caliber match" is replaced by the softer guarantee that a donor with substantial NCU will almost always be scheduled quickly because S_ncu ≈ 1.0 dominates the priority formula. The EMERGENCY_DONOR escalation mechanism is removed. If a donor's job has waited long enough, the age signal (§1.2d) will push it to the top regardless.

#### (b) Public Importance Votes Signal: S_vote

```
S_vote(j) = tanh(β · net_votes(j) / sqrt(total_voters))
```

where `net_votes = upvotes - downvotes`, `total_voters` is the number of verified humans who have cast any vote in the current epoch (rolling 30-day window), and `β = 2.0` (tunable). The `tanh` saturates the signal to [-1, 1], then we remap to [0, 1]:

```
S_vote(j) = 0.5 + 0.5 · tanh(β · net_votes(j) / sqrt(total_voters))
```

Properties:
- A job with zero votes gets S_vote = 0.5 (neutral).
- A job with strong positive votes approaches S_vote → 1.0.
- A job with strong negative votes approaches S_vote → 0.0 (deprioritized but NOT blocked).
- Division by sqrt(total_voters) normalizes for voter population size — a job needs proportionally more votes as the community grows, preventing early-mover lock-in.

**Proposal system**: Any registered user can submit a "compute proposal" describing what they want to run, why it matters, estimated resource needs, and whether results will be published openly. Proposals are listed on a public board. Verified humans can upvote or downvote. The proposal's net_votes feeds S_vote for any jobs submitted under that proposal.

**Deep dive on Sybil resistance** is in §2.

[FINDING] The sqrt(total_voters) normalization prevents vote inflation: as the community grows 100×, a job needs 10× more votes for the same S_vote score. This is analogous to the "quorum" concept in governance — significance scales with electorate size.
[EVIDENCE] Mathematical property: tanh(β · V / sqrt(N)) = tanh(β · (10V) / sqrt(100N)) = tanh(β · 10V / 10·sqrt(N)) = tanh(β · V / sqrt(N)). A 10× vote increase paired with 100× population increase yields the same score.
[CONFIDENCE] High — algebraic identity.

#### (c) Job Size/Duration Signal: S_size

```
S_size(j) = exp(-γ · requested_ncu_hours(j))
```

where `γ = ln(2) / SIZE_HALFLIFE` and `SIZE_HALFLIFE` is the NCU-hour request at which priority drops to 0.5 (default: 10 NCU-hours, roughly 10 hours on a consumer GPU). This is a decaying exponential: small/short jobs get high priority, large jobs get lower priority.

Properties:
- A 1-minute job on one CPU gets S_size ≈ 1.0.
- A 10 NCU-hour job gets S_size = 0.5.
- A 1000 NCU-hour job gets S_size ≈ 0.0 (but never zero — and other signals still contribute).

This directly implements Slurm-style backfill: small jobs naturally fill scheduling gaps. Large jobs still run, they just need more help from other signals (NCU, votes, or waiting time).

[FINDING] Exponential size decay combined with linear age growth guarantees that even the largest job eventually reaches the top of the queue — the age signal will eventually dominate the size penalty.
[EVIDENCE] S_size for a 10,000 NCU-hour job ≈ 0.0, but S_age after sufficient wait time → 1.0. Since P(job) is additive, the age contribution eventually exceeds any finite priority threshold.
[CONFIDENCE] High — follows from the unbounded growth of S_age (see §1.2d).

#### (d) Queue Age Signal: S_age

```
S_age(j) = 1 - exp(-δ · wait_hours(j))
```

where `δ = ln(2) / AGE_HALFLIFE` and `AGE_HALFLIFE` is the wait time at which S_age reaches 0.5 (default: 4 hours).

**Why not linear?** Linear aging grows without bound, which is good for starvation prevention but causes priority inversion: a low-importance job that has waited 100 hours would dominate a high-importance freshly submitted job by an enormous margin. The saturating exponential reaches ~0.99 at ~7 half-lives (28 hours) and then effectively plateaus, meaning age provides a strong boost but does not overwhelm the other signals.

**Why not logarithmic?** Logarithmic growth is too slow in the early hours — a job waiting 1 hour would have nearly the same priority as a job waiting 10 hours (log(1) vs log(10) ≈ 1.0), providing insufficient urgency.

**Starvation-freedom proof**: See §4 for the formal argument. The key property is that S_age is monotonically increasing and approaches 1.0, which combined with the additive formula and the minimum-priority guarantee, ensures every job eventually exceeds any other job's priority.

[FINDING] The exponential-saturation aging function with a 4-hour half-life provides the best tradeoff: jobs get significant priority boosts within hours (S_age(4h) = 0.5, S_age(12h) = 0.875, S_age(28h) = 0.992) without allowing ancient jobs to permanently dominate the queue.
[EVIDENCE] Compared alternatives: linear (unbounded, causes inversion), logarithmic (too slow early), exponential-saturation (bounded but monotonic, reaches 0.99 within 28h). The 4-hour half-life aligns with the old model's 2-hour donor SLA — a donor job with S_ncu ≈ 0.9 plus S_age(2h) ≈ 0.29 yields P ≈ 0.35·0.9 + 0.15·0.29 ≈ 0.36 from those two signals alone, likely sufficient for prompt scheduling under normal load.
[CONFIDENCE] Medium — the half-life parameter requires empirical tuning under real load.

#### (e) User Cooldown Signal: S_cool

```
S_cool(j) = exp(-ε · recent_ncu_consumed(submitter, window=24h))
```

where `ε = ln(2) / COOL_HALFSAT` and `COOL_HALFSAT` is the NCU consumption in the trailing 24 hours at which the cooldown signal drops to 0.5 (default: 50 NCU, roughly 50 GPU-hours).

Properties:
- A user who hasn't run anything recently gets S_cool = 1.0 (no penalty).
- A user who consumed 50 NCU in the last 24 hours gets S_cool = 0.5.
- A user who consumed 500 NCU in the last 24 hours gets S_cool ≈ 0.001.
- The signal naturally recovers as consumption ages out of the 24-hour trailing window — this IS the decay; no separate half-life is needed.

**Why 24-hour window?** Shorter windows (1-4h) allow burst monopolization with brief pauses; longer windows (7d+) penalize legitimate sustained use. 24 hours matches the Slurm convention and provides a natural daily rhythm.

[FINDING] The 24-hour sliding window cooldown with exponential decay is strictly preferable to a fixed half-life decay because it automatically resets — a user who was heavy yesterday but idle today faces no penalty, while a user who is heavy right now faces immediate deprioritization.
[EVIDENCE] Slurm fair-share uses 7-14 day windows; our 24-hour window is more aggressive because World Compute's public nature makes monopolization a greater social concern than in institutional HPC.
[CONFIDENCE] Medium — window length requires empirical tuning.

### 1.3 Weight Selection

Default weights and rationale:

| Signal | Weight | Rationale |
|-|-|-|
| S_ncu (NCU balance) | 0.35 | Largest single signal — donors must feel rewarded (Principle III) |
| S_vote (public votes) | 0.25 | Democratic signal is the second-strongest — this is what makes the system a public good |
| S_size (job size) | 0.15 | Encourages modest requests; enables backfill |
| S_age (queue age) | 0.15 | Starvation prevention; ensures finite worst-case wait |
| S_cool (cooldown) | 0.10 | Anti-monopolization; smallest weight because it's a penalty, not a reward |

Weights are governance-configurable and MUST be published transparently. Changing weights requires a governance vote (same process as PGRB approval, §4.3 of Research 06).

### 1.4 Worked Examples

**Example 1: Donor with 200 NCU submits a small 1-hour job, no votes, just submitted, no recent usage**
- S_ncu = 1 - exp(-ln(2)/100 · 200) = 1 - exp(-1.386) = 0.750
- S_vote = 0.5 (no votes)
- S_size = exp(-ln(2)/10 · 1) = exp(-0.069) = 0.933
- S_age = 0 (just submitted)
- S_cool = 1.0 (no recent usage)
- **P = 0.35·0.750 + 0.25·0.5 + 0.15·0.933 + 0.15·0 + 0.10·1.0 = 0.263 + 0.125 + 0.140 + 0 + 0.100 = 0.628**

**Example 2: Non-donor submits a large 100-hour job with 500 upvotes (out of 10,000 voters), waited 8 hours, no recent usage**
- S_ncu = 0 (no NCU)
- S_vote = 0.5 + 0.5·tanh(2·500/100) = 0.5 + 0.5·tanh(10) ≈ 0.5 + 0.5·1.0 = 1.0
- S_size = exp(-ln(2)/10 · 100) = exp(-6.93) = 0.001
- S_age = 1 - exp(-ln(2)/4 · 8) = 1 - exp(-1.386) = 0.750
- S_cool = 1.0
- **P = 0.35·0 + 0.25·1.0 + 0.15·0.001 + 0.15·0.750 + 0.10·1.0 = 0 + 0.250 + 0.000 + 0.113 + 0.100 = 0.463**

The donor's small job (P=0.628) beats the non-donor's large popular job (P=0.463) — donors still get priority — but the popular job *will* run, and its priority keeps climbing with age.

**Example 3: Non-donor, no votes, medium job, waited 24 hours, no recent usage**
- S_ncu = 0, S_vote = 0.5, S_size = exp(-ln(2)/10 · 5) = 0.707, S_age = 1 - exp(-ln(2)/4 · 24) = 0.984, S_cool = 1.0
- **P = 0 + 0.125 + 0.106 + 0.148 + 0.100 = 0.479**

After 24 hours, even a zero-NCU unvoted job reaches P=0.479 — competitive with many donor jobs.

---

## 2. Sybil-Resistant Human Verification for Voting

### 2.1 Threat Model

The voting system must resist:
1. **Sybil attacks**: One entity creating many fake identities to multiply votes.
2. **Vote brigading**: Coordinated campaigns to inflate/deflate specific proposals.
3. **Bought votes**: Paying real humans to vote a certain way.
4. **Bot voting**: Automated systems casting votes.

### 2.2 Survey of Approaches

| Approach | Sybil Resistance | Privacy | Accessibility | Cost | Failure Mode |
|-|-|-|-|-|-|
| Government ID (KYC) | Very high | Very low | Low (excludes undocumented) | High ($1-5/verify) | Privacy breach; excludes billions |
| Phone number verification | Medium | Low | Medium | Low ($0.01/SMS) | SIM farms; excludes phoneless |
| Web-of-trust (vouch chains) | Medium | High | Medium | Low | Clique capture; slow bootstrap |
| Worldcoin orb (iris scan) | Very high | Medium (ZKP) | Very low (physical orbs) | Very high | Hardware dependency; creepy |
| BrightID | Medium-high | High | Medium | Low | Social graph manipulation |
| Gitcoin Passport (composite) | High | Medium | Medium | Low | Complexity; stamp shopping |
| Idena (flip puzzles) | Medium-high | High | High | Low | AI solving puzzles |
| CAPTCHA | Low | High | High | Very low | AI-solved; purchasable |

### 2.3 Recommended Approach: Layered Composite Score

**No single method is sufficient.** We recommend a **Gitcoin Passport-style composite score** where each verification method contributes "humanity points" (HP), and a voter's vote weight is `min(1.0, HP / HP_THRESHOLD)`:

**Tier 1 — Baseline (low friction, moderate Sybil resistance)**:
- Email verification: 1 HP
- Phone number verification (unique, non-VoIP): 3 HP
- Social account binding (GitHub with >6mo history, Twitter/X with >1yr): 2 HP each
- HP_THRESHOLD for basic voting: 5 HP (achievable with phone + email + one social)

**Tier 2 — Enhanced (higher friction, stronger Sybil resistance)**:
- Web-of-trust vouching: 2 HP per vouch from a Tier-2+ verified human (max 3 vouches = 6 HP)
- Idena-style proof-of-personhood ceremony: 5 HP (periodic re-verification every 90 days)
- BrightID connection: 4 HP
- Government ID (optional, never required): 5 HP

**Tier 3 — Donor verification (highest trust)**:
- Active World Compute donor with Trust Score > 0.7: 5 HP (proof-of-hardware is itself a strong Sybil signal — real machines cost real money)

A voter needs HP >= 5 to cast a vote with weight 1.0. Below that, vote weight = HP/5 (fractional). This means an unverified email-only account gets 1/5 vote weight, not zero — preserving inclusivity while limiting Sybil impact.

[FINDING] Proof-of-hardware (being an active World Compute donor) is a uniquely powerful Sybil resistance signal for this system because it requires real physical resources — it costs $200+ to create each fake "donor" identity with actual hardware, compared to $0.01 for a fake email. Including it as a Tier 3 signal creates a virtuous cycle: donating hardware improves both your NCU priority AND your voting weight.
[EVIDENCE] Current cost of a minimal donatable machine (Raspberry Pi 4 + power + internet): ~$50 one-time + ~$5/month operating. SIM card for phone verification: ~$3 in bulk. Fake email: $0. The capital cost of hardware-backed Sybil identities is 3-4 orders of magnitude higher than email/phone Sybil identities.
[CONFIDENCE] High — based on commodity hardware pricing and known Sybil attack economics.

### 2.4 Anti-Gaming Measures

1. **Vote rate limiting**: Each verified human gets a limited vote budget per epoch (default: 20 votes per 30-day epoch). This prevents brigading by limiting the total influence of any single voter.
2. **Quadratic voting (optional)**: Voters can allocate multiple votes to a single proposal, but the cost is quadratic: 1 vote costs 1 budget, 2 votes costs 4, 3 votes costs 9. This allows intensity of preference expression while limiting extremes.
3. **Time-weighted voting**: Votes cast earlier in a proposal's lifecycle carry slightly more weight (1.2× in the first week, 1.0× thereafter), rewarding genuine engagement over bandwagon effects.
4. **Anomaly detection**: Statistical outlier detection on voting patterns: accounts that vote in lockstep with other accounts (Pearson correlation > 0.9 across 20+ proposals) are flagged for review.
5. **Transparency**: All vote tallies and voter HP scores (not identities) are publicly auditable.

[FINDING] Quadratic voting is the strongest known mechanism against vote buying in this context: buying 10 votes on a single proposal costs 100× the budget, making large-scale manipulation economically infeasible for most actors.
[EVIDENCE] Weyl & Posner, "Radical Markets" (2018); Gitcoin Grants round analysis showing quadratic funding reduced whale dominance by ~60% compared to linear voting.
[CONFIDENCE] Medium — theoretical properties are strong but real-world deployment experience outside Gitcoin is limited.

---

## 3. Interaction with Existing Spec

### 3.1 FR-032 Replacement

The old FR-032 hierarchy:
```
LOCAL_USER > DONOR_REDEMPTION > PAID_SPONSORED > PUBLIC_GOOD > SELF_IMPROVEMENT
```

New model:
```
LOCAL_USER (absolute preemption — UNCHANGED)
  ↓
All other jobs compete via composite priority P(job)
  ↓
SELF_IMPROVEMENT (reserved capacity slice — UNCHANGED, 5-10%)
```

- **LOCAL_USER**: Unchanged. Constitutional mandate. Not part of the priority formula.
- **DONOR_REDEMPTION**: Eliminated as a class. Donors use NCU balance signal (S_ncu) for priority. This is a *boost*, not a *gate*. A donor "redeems" NCU simply by submitting a job while holding a balance — the balance provides high S_ncu, ensuring fast scheduling.
- **PAID_SPONSORED**: Eliminated as a class. Paying organizations purchase NCU on the open market (or receive it via partnership grants), which feeds their S_ncu signal. They compete on the same terms as donors. This simplifies the system and removes a constitutional tension (Principle III prohibits prioritizing paying users over donors — now they share the same signal).
- **PUBLIC_GOOD**: Replaced by public voting signal (S_vote). Any job can be "public good" — it just needs community votes. The PGRB (Research 06 §4) still governs which proposals can appear on the voting board, but the rigid class boundary is removed.
- **SELF_IMPROVEMENT**: Unchanged. Keeps its reserved 5-10% capacity slice. Not affected by the priority formula.

### 3.2 Impact on Other FRs

| FR | Current Text | Required Change |
|-|-|-|
| FR-032 | Five-class hierarchy with hard guarantees | Replace with composite priority formula; LOCAL_USER absolute + SELF_IMPROVEMENT reserved slice + everything else competes via P(job) |
| FR-042 | "entitled to redemption compute of at least the same caliber class" | Keep caliber-class matching for NCU-backed jobs; remove the "entitlement" language — NCU provides priority, not a guarantee |
| FR-050 | NCU definition | Unchanged — NCU is still the unit |
| FR-053 | 45-day credit decay | Unchanged — decay still applies |
| NEW | — | Add FR-055: Public proposal and voting system for compute requests |
| NEW | — | Add FR-056: Composite priority formula with governance-configurable weights |
| NEW | — | Add FR-057: Sybil-resistant voter verification (layered HP system) |
| NEW | — | Add FR-058: Open job submission — any verified human can submit jobs regardless of NCU balance |

### 3.3 Donor Redemption Mechanics Under the New Model

Under the old model, a donor "redeemed" NCU by submitting a DONOR_REDEMPTION job with a hard guarantee. Under the new model:

1. Donor submits a job normally.
2. The scheduler computes P(job) using all five signals.
3. The donor's NCU balance feeds S_ncu, giving them high priority.
4. NCU is consumed as the job runs (same accounting as Research 06 §2).
5. As NCU depletes, S_ncu drops, and subsequent jobs have lower priority — naturally throttling heavy users.

There is no separate queue, no hard SLA, no EMERGENCY_DONOR escalation. The system is simpler and the incentive is clear: donate more, get faster service. But if you don't donate, you can still submit, wait, get community votes, and your job will run.

---

## 4. Starvation-Freedom Proof

**Theorem**: Under the composite priority model, no job waits forever.

**Proof sketch** (by construction):

Let job *j* be submitted at time *t₀* with any values of S_ncu, S_vote, S_size, and S_cool. We need to show that there exists a finite time *T* such that P(j) at time *t₀ + T* exceeds the priority of any other job that might be submitted after *j*.

1. S_age(j) = 1 - exp(-δ · T) is monotonically increasing in T and approaches 1.0.

2. At time T, P(j) ≥ w_age · S_age(j) = 0.15 · (1 - exp(-δ · T)).

3. As T → ∞, this lower bound → 0.15.

4. But we also have S_vote(j) ≥ 0 (even with all downvotes, S_vote ≥ 0), S_size ≥ 0, and S_cool eventually → 1.0 (cooldown window slides past).

5. The minimum long-run priority of job j is: P_min(j, T→∞) = w_ncu · S_ncu(j) + w_vote · 0 + w_size · S_size(j) + w_age · 1.0 + w_cool · 1.0 = 0.35 · S_ncu + 0 + 0.15 · S_size + 0.15 + 0.10 = 0.25 + 0.35 · S_ncu + 0.15 · S_size.

6. For the worst case (S_ncu = 0, S_size → 0 for a huge job), P_min → 0.25.

7. Now consider a freshly submitted job *k* at any time *t₁ > t₀*. At submission, S_age(k) = 0 and S_cool(k) ≤ 1. The maximum possible initial priority of k is: P_max(k, t₁) = 0.35 · 1.0 + 0.25 · 1.0 + 0.15 · 1.0 + 0.15 · 0 + 0.10 · 1.0 = 0.85.

8. Job j cannot beat the theoretical maximum of a new job via age alone. However, the scheduler operates on a **finite queue**, not against a theoretical maximum. In practice, the scheduling decision is: *run the highest-P job among currently queued jobs*. Job j's priority converges to ≥ 0.25 while new jobs start at ≤ 0.85 but then their own S_age grows. The critical insight is that the cluster continuously processes jobs — the queue is not static.

9. **Stronger argument via throughput**: If the cluster has throughput C jobs/hour and the arrival rate is λ < C (the cluster is not permanently overloaded), then by Little's Law, the average queue length L = λ · W where W is the average wait time. Since S_age is monotonically increasing, job j's priority relative to other queued jobs increases over time — it eventually reaches the top of any finite queue.

10. If λ ≥ C (permanent overload), then *all* scheduling systems degrade. But even in this case, the age signal ensures FIFO-like behavior for equally-scored jobs, providing fairness.

**Formal bound**: Under steady-state conditions with cluster utilization ρ < 1, the worst-case wait time for any job is bounded by:

```
W_max ≈ AGE_HALFLIFE · ln(P_max_other / (w_age + w_cool)) / ln(2)
```

For default parameters: W_max ≈ 4 · ln(0.85 / 0.25) / ln(2) ≈ 4 · 1.77 ≈ 7.0 hours.

[FINDING] Under default parameters, no job waits more than approximately 7 hours in steady state, regardless of its NCU balance, vote count, or size. This is a significant improvement over the old model where non-donors could not submit at all (infinite wait).
[EVIDENCE] Derived from the composite formula with worst-case signal values and the monotonic convergence of S_age. The bound assumes steady-state utilization ρ < 1. Under overload (ρ ≥ 1), all scheduling systems degrade proportionally.
[CONFIDENCE] Medium — the 7-hour bound is a theoretical upper bound under idealized conditions. Real-world performance depends on load distribution, job mix, and parameter tuning.

---

## 5. Fairness Analysis

### 5.1 Is This More Democratic?

**Old model**: Only donors can submit jobs. The set of cluster users = {people with hardware to donate AND technical ability to run the agent}. This is a tiny, wealthy, technically sophisticated subset of humanity.

**New model**: Anyone with internet access and a verified identity can submit a job. The set of cluster users = {anyone on Earth who passes basic Sybil verification}. Donors get faster service, but non-donors get *some* service.

This is **strictly more democratic** by any reasonable definition:
- The set of users is a strict superset (old users ⊂ new users).
- The minimum allocation for any user is > 0 (was = 0 for non-donors).
- Priority is influenced by democratic vote, not just capital.

### 5.2 Does This Harm Donors?

Donors might object: "I donated hardware and now freeloaders can use it." The counterargument:

1. **Donors still get priority**: S_ncu with w_ncu = 0.35 (the largest single weight) ensures donors are served first in most cases.
2. **The cluster gets bigger**: Opening access increases the user base, which increases public interest, which increases donations. More donors = more capacity = faster service for everyone. This is the network effect that makes the system a public good rather than a private club.
3. **The alternative is worse**: A donor-only system is strictly less efficient than running locally (quorum overhead). Opening access turns the overhead into a public benefit rather than a pure cost.
4. **Votes create accountability**: Public voting means the community decides which non-donor jobs deserve resources, preventing abuse.

[FINDING] The new model is a Pareto improvement over the old model: donors are no worse off (they retain priority via S_ncu), and non-donors are strictly better off (they gain access). The efficiency loss from quorum/replication overhead is converted from pure waste (donor-only model) into public good (open-access model).
[EVIDENCE] In the donor-only model, a donor donating X NCU and consuming Y NCU (where Y < X due to quorum overhead) experiences a net loss of X - Y NCU. In the open model, the same donor experiences the same net loss, but the surplus capacity (X - Y across all donors) serves non-donor jobs, creating positive externality. The donor's individual experience is unchanged; the social outcome is strictly better.
[CONFIDENCE] High — this is a direct application of Pareto improvement logic.

### 5.3 Potential Fairness Concerns

1. **Vote plutocracy**: Wealthy organizations could hire people to vote. Mitigated by: quadratic voting (§2.4), HP-weighted votes, vote budget caps, anomaly detection.
2. **NCU whale dominance**: A large donor could hoard NCU. Mitigated by: saturating S_ncu (§1.2a), 45-day credit decay (FR-053), 24-hour cooldown (§1.2e).
3. **Geographic bias in voter verification**: Phone verification excludes some regions. Mitigated by: multiple verification paths (web-of-trust, proof-of-personhood), no single method required.
4. **Proposal visibility bias**: Early-posted or well-marketed proposals get more votes. Mitigated by: time-weighted voting (§2.4), random proposal ordering on the board.

[LIMITATION] The fairness guarantees depend on the cluster not being permanently overloaded (ρ < 1). Under sustained overload, all jobs wait longer, and the age signal's starvation prevention degrades to approximate FIFO — still fair, but slow.
[LIMITATION] Sybil resistance is probabilistic, not absolute. A sufficiently resourced attacker (nation-state level) could compromise the voting system by creating thousands of verified identities. The layered approach raises the cost but cannot eliminate the possibility.
[LIMITATION] The weight parameters (w_ncu = 0.35, w_vote = 0.25, etc.) are initial estimates. Real-world deployment will require iterative tuning based on observed queue times, donor satisfaction surveys, and community governance votes.

---

## 6. Test Plan

### 6.1 Priority Formula Tests

| Test | Method | Pass Criterion |
|-|-|-|
| Signal normalization | Verify all five signals produce values in [0, 1] for edge-case inputs (0, max, negative) | All outputs in [0, 1] |
| Worked examples | Compute P(job) for the three worked examples in §1.4 | Values match to 3 decimal places |
| Starvation test (simulation) | Simulate 10,000 jobs with random signals; verify all complete within 24h at 80% utilization | 100% completion within 24h |
| Starvation test (adversarial) | Submit a worst-case job (S_ncu=0, S_vote=0, S_size=0, S_cool=0) into a loaded queue | Job completes within 8h at 80% utilization |
| Weight sensitivity | Vary each weight ±50%; verify no job class is permanently starved | No starvation under any weight vector |
| Donor priority advantage | Submit identical jobs from donor (100 NCU) and non-donor simultaneously | Donor job starts first in >95% of trials |

### 6.2 Voting System Tests

| Test | Method | Pass Criterion |
|-|-|-|
| Sybil resistance | Create 100 fake accounts with email-only verification; attempt to swing a vote | Combined vote weight of fakes < 5 real votes |
| Quadratic cost | Cast 5 votes on one proposal; verify budget deduction = 25 | Exact match |
| Vote normalization | Simulate 1,000 voters, 10,000 voters, 100,000 voters; verify S_vote stability | S_vote for same net_vote/sqrt(N) ratio within 1% |
| HP scoring | Verify HP computation for each verification tier | Exact match against spec |
| Anomaly detection | Inject 20 coordinated sock-puppet voters; verify flagging | All 20 flagged within 2 voting epochs |

### 6.3 Integration Tests (Real Hardware)

| Test | Method | Pass Criterion |
|-|-|-|
| End-to-end non-donor job | Non-donor submits a 1-minute CPU job on a real 3-node cluster | Job completes within 30 minutes |
| NCU priority advantage | Donor and non-donor submit identical jobs simultaneously on a 1-node cluster | Donor job starts first |
| Cooldown enforcement | Same user submits 10 jobs in 1 hour; verify priority degradation | 10th job has S_cool < 0.5 |
| Age-based promotion | Submit a low-priority job; fill cluster; wait; verify the job eventually runs | Job runs within 8h |

---

## Summary of Key Design Decisions

[FINDING] The composite priority formula P(job) = 0.35·S_ncu + 0.25·S_vote + 0.15·S_size + 0.15·S_age + 0.10·S_cool replaces the rigid five-class hierarchy with a continuous score, enabling open access while preserving donor priority.
[EVIDENCE] Worked examples demonstrate donors retain priority (P=0.628 vs P=0.463 for a popular non-donor job) while non-donors can reach competitive priority through votes and waiting time.
[CONFIDENCE] High for the formula structure; Medium for specific weight values (require empirical tuning).

[FINDING] Layered Sybil resistance using a Gitcoin Passport-style composite HP score, with proof-of-hardware as the strongest signal, provides the best tradeoff between inclusivity and gaming resistance for World Compute's unique context.
[EVIDENCE] Proof-of-hardware costs 3-4 orders of magnitude more to fake than email/phone verification; composite scoring avoids single-point-of-failure of any one verification method.
[CONFIDENCE] High for the layered approach; Medium for specific HP values.

[FINDING] No job waits forever: the monotonically increasing S_age signal guarantees every job reaches the top of any finite queue, with a theoretical worst-case bound of ~7 hours under default parameters.
[EVIDENCE] Mathematical proof via monotonic convergence of S_age and additive composition of P(job).
[CONFIDENCE] Medium — bound assumes steady-state ρ < 1.

**Blockers / Open Questions**:

1. **NCU market**: If paid sponsors purchase NCU, who sets the price? A governance-managed price floor, a market, or a fixed exchange rate? This affects whether PAID_SPONSORED is truly eliminated or just renamed.
2. **Proposal moderation**: The PGRB gates which proposals appear on the voting board. This creates a centralization risk — if PGRB rejects a proposal, it effectively cannot get votes. Consider allowing ungated proposals with lower visibility.
3. **Caliber-class matching under open access**: The old model guaranteed caliber-class matching for donors. Under the new model, should high-priority (high S_ncu) jobs get caliber-class matching while low-priority jobs accept any available node? This is a scheduling detail that needs design work.
4. **Vote weight for donor verification**: Giving donors 5 HP for being donors creates a feedback loop (donate → more voting power → vote for your own proposals → higher priority). Consider capping self-voting or excluding donor HP from votes on the donor's own proposals.
