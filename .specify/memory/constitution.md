<!--
SYNC IMPACT REPORT
==================
Version change: 0.0.0 (template placeholder) → 1.0.0 (initial ratification)
Bump rationale: First concrete ratification of the World Compute constitution;
all placeholder tokens replaced with governing content.

Modified principles (template → ratified):
- [PRINCIPLE_1_NAME] → I. Safety First (Sandboxing & Host Integrity)
- [PRINCIPLE_2_NAME] → II. Robustness & Graceful Degradation
- [PRINCIPLE_3_NAME] → III. Fairness & Donor Sovereignty
- [PRINCIPLE_4_NAME] → IV. Efficiency, Performance & Self-Improvement
- [PRINCIPLE_5_NAME] → V. Direct Testing (NON-NEGOTIABLE)

Added sections:
- Additional Constraints & Operating Requirements
- Development Workflow & Quality Gates
- Governance

Removed sections: none (all template placeholders replaced in-place)

Templates requiring updates:
- ✅ .specify/memory/constitution.md (this file)
- ⚠ .specify/templates/plan-template.md — "Constitution Check" section is
  a placeholder; should be populated by /speckit.plan runs to reference the
  five principles below. No edits needed to the template itself.
- ⚠ .specify/templates/spec-template.md — mandatory sections remain valid;
  ensure security/sandboxing + donor-impact requirements are captured in
  feature specs going forward (enforced by plan-time Constitution Check).
- ⚠ .specify/templates/tasks-template.md — existing task categories are
  compatible; feature plans should add explicit "direct test on real
  hardware" tasks per Principle V.
- ⚠ README.md / quickstart.md — not yet present; create when project work
  begins and link this constitution.

Deferred items / TODOs:
- TODO(RATIFICATION_DATE): set to 2026-04-15 as the project has not yet
  started and this is the first ratification. If the founding team adopts
  a different formal ratification date, amend via PATCH bump.
-->

# World Compute Constitution

World Compute is a voluntary, planet-scale, SETI@home-style compute federation
built from hardware donated by anyone on Earth who opts in. It is intended to
become a public good: the most powerful compute cluster on Earth, continually
growing, serving all of humanity. The principles below are binding on every
component, protocol, and deployment.

## Core Principles

### I. Safety First (Sandboxing & Host Integrity)

The safety of donor machines and the privacy of their owners is the single
highest priority and overrides every other concern in this document.

- All workloads MUST execute inside a hardened, defense-in-depth sandbox
  (minimum: OS-level isolation + process/namespace isolation + hypervisor or
  equivalent VM/MicroVM boundary) with no path to the host kernel, host
  filesystem, host network credentials, peripherals, or host user data.
- Private information belonging to the donor (files, credentials, browser
  state, keys, network identity, camera/mic/clipboard, LAN peers) MUST remain
  100% inaccessible to cluster workloads. There is no "mostly isolated".
- The client agent MUST drop privileges to the minimum required, MUST be
  reproducibly built, MUST be code-signed, and MUST be independently auditable.
- Cluster workloads MUST NOT be able to gain persistent state on the host
  beyond an explicitly scoped, size-capped working directory that is wiped on
  job completion or agent exit.
- Any discovered sandbox-escape, privilege-escalation, or host-data-exfiltration
  vulnerability is a P0 incident: affected client versions MUST be remotely
  disabled, and new jobs MUST halt cluster-wide until a fix is deployed and
  verified.
- Cryptographic attestation MUST be used to ensure only trusted, signed agent
  and workload images run on donor machines.

**Rationale**: Donors are lending real hardware they use for their lives and
livelihoods. A single breach would destroy public trust in the cluster
permanently. Safety is not a feature — it is the precondition for the project
existing at all.

### II. Robustness & Graceful Degradation

The cluster MUST treat every node and every storage device as fundamentally
unreliable and capable of disappearing at any instant, including mid-job.

- Scheduling MUST be Kubernetes-like: declarative workload specs, automatic
  rescheduling, health checking, self-healing, and progressive rollout.
- Storage MUST be RAID-like / erasure-coded across geographically and
  administratively independent donors; no single-node failure and no plausible
  correlated failure of a small subset MUST cause data loss for an active job.
- Every long-running job MUST support checkpointing and resume-from-checkpoint
  on arbitrary other nodes. "Restart from zero" is not an acceptable recovery
  strategy for non-trivial jobs.
- The control plane itself MUST be replicated and survive the loss of any
  individual region, provider, or coordinator.
- Network partitions, high churn, and Byzantine/faulty donors MUST be assumed
  as the normal operating condition, not as edge cases.
- Every component MUST define its failure modes explicitly and MUST degrade
  gracefully rather than fail catastrophically.

**Rationale**: A cluster composed of the general public's laptops, phones,
and home servers will experience churn rates orders of magnitude higher than
a datacenter. Robustness cannot be bolted on; it must be the default posture
of every subsystem.

### III. Fairness & Donor Sovereignty

Donors are first-class citizens. Their local experience MUST NEVER be degraded
by the cluster, and contribution MUST translate into proportional access.

- The local human user and their processes ALWAYS take absolute priority over
  cluster workloads on their own machine. If the user resumes activity
  (keyboard, mouse, foreground app, thermal/battery pressure, user-defined
  triggers), all cluster jobs on that machine MUST stop immediately and yield
  CPU, GPU, memory, disk I/O, network, and power.
- "Stop immediately" means within a bounded, published latency budget
  (target: sub-second yield of interactive resources) and MUST NOT rely on
  cooperative workload behavior — the agent MUST be able to forcibly suspend
  or terminate jobs.
- In exchange for donating hardware, a donor MUST be entitled to request
  cluster compute with a guaranteed MINIMUM allocation of resources of at
  least the same caliber and performance class as what they donated, averaged
  over a fair accounting window. "Same caliber" covers compute, memory,
  storage, and network tier — not just raw hours.
- The cluster MUST NOT silently downgrade donors, rate-limit them below their
  earned minimum, or prioritize paying/institutional users over donors' earned
  allocation.
- Accounting of contribution and consumption MUST be transparent, auditable,
  and inspectable by the donor at any time.
- Donors MUST be able to withdraw their hardware from the cluster at any time
  with no penalty and no residual cluster state left on their machine.

**Rationale**: The cluster exists only because people choose to donate. Any
system that treats donors as exploitable resources — rather than as
sovereign owners who are generously sharing — will hemorrhage participants
and collapse. Fairness is how the cluster stays alive.

### IV. Efficiency, Performance & Self-Improvement

World Compute is intended to become a globally significant compute resource
and a public good. It MUST treat efficiency as a core obligation, not an
optimization.

- The system MUST make efficient use of all available resources across
  compute, memory, storage, network, and energy. Wasted cycles on donor
  hardware are considered a real cost to donors and to the planet.
- Scheduling MUST be locality- and energy-aware: prefer warm data, prefer
  geographically nearby collaborators, prefer times and regions with lower
  marginal energy cost and lower carbon intensity when feasible without
  violating fairness or safety.
- A non-trivial, explicitly budgeted fraction of total cluster capacity MUST
  at all times be allocated to continually improving the cluster itself
  (scheduler quality, sandbox hardening, storage efficiency, protocol
  evolution, observability, and research into making the system cheaper,
  safer, and more useful). This self-improvement budget is a permanent line
  item, not a phase.
- As the cluster scales toward consuming a substantial fraction of world
  energy use, it MUST publish its aggregate energy and carbon footprint and
  MUST actively reduce joules-per-useful-result over time. Growth without
  efficiency improvement is a governance failure.
- Performance regressions in core subsystems MUST be treated as defects and
  MUST block release.

**Rationale**: A public-good cluster that squanders donated hardware and
donated power is not a public good. At planetary scale, inefficiency is
measured in gigawatts; the system has a duty to keep improving itself.

### V. Direct Testing (NON-NEGOTIABLE)

No component ships, deploys, or participates in production scheduling until
it has been directly tested by running real test jobs on real target systems
and verifying the returned values against known-correct expectations.

- "Directly tested" means end-to-end execution on representative real
  hardware (including real donor-class machines, real sandboxes, real
  networks), not in simulation alone and not with mocks standing in for the
  components under test.
- Mocks and simulators MAY be used for regression speed, but MUST NOT be the
  sole evidence of correctness for any component entering production.
- Every release candidate MUST produce a direct-test evidence artifact:
  the job(s) that ran, the systems they ran on, the inputs, the expected
  outputs, and the observed outputs, with a pass/fail determination.
- Safety-critical paths (sandboxing, privilege boundaries, resource yield to
  local users, data-loss prevention, attestation) MUST be directly tested on
  every release and MUST include adversarial test cases, not just happy path.
- A failing or unverifiable direct test MUST block deployment. There is no
  "we'll fix it in the next release" exception for Principles I, II, III,
  or V.

**Rationale**: A cluster this safety-critical and this widely distributed
cannot rely on "it compiles and the unit tests pass." The only thing that
proves the system works is the system actually working, on real machines,
returning real correct answers.

## Additional Constraints & Operating Requirements

- **Open and auditable**: The agent, sandbox, scheduler, and protocols MUST
  be open-source and independently auditable. Closed-source binaries MUST
  NOT run on donor machines.
- **Consent and transparency**: Donors MUST give informed, granular, and
  revocable consent to what their hardware will be used for. Job classes
  (e.g., scientific, ML training, rendering, indexing) MUST be declarable
  and donors MUST be able to opt in or out per class.
- **Abuse resistance**: The system MUST refuse jobs that would harm donors,
  third parties, or the network (e.g., unauthorized scanning, malware
  distribution, illegal content, targeted surveillance). Acceptable-use
  enforcement is a first-class system concern, not an afterthought.
- **Privacy of users of the cluster**: Job submitters' data and code MUST be
  protected from donor nodes to the extent compatible with the workload
  (e.g., via confidential computing, encryption, and result verification).
- **No lock-in**: Donors and job submitters MUST be able to leave the system
  cleanly, taking their data with them.
- **Incident disclosure**: Security incidents affecting donor machines MUST
  be disclosed publicly within a bounded, pre-committed timeframe after
  mitigation.

## Development Workflow & Quality Gates

- **Constitution Check at planning time**: Every `/speckit.plan` run MUST
  evaluate proposed work against Principles I–V and document any tension
  or tradeoff explicitly in the plan's Constitution Check section. Plans
  that conflict with a principle MUST be revised or MUST include an
  explicit, justified complexity/exception entry that is reviewed.
- **Feature specs MUST address**:
  1. Host integrity and data-isolation impact (Principle I),
  2. Failure modes and recovery behavior (Principle II),
  3. Donor-experience and fairness impact (Principle III),
  4. Resource, energy, and self-improvement implications (Principle IV),
  5. Direct-test plan on real hardware (Principle V).
- **Tasks lists** generated by `/speckit.tasks` MUST include at least one
  direct-test task executed on real representative hardware for every user
  story that touches cluster execution, scheduling, storage, or sandboxing.
- **Review gates**: Code review MUST verify principle compliance. Reviewers
  MUST block merges that regress sandbox strength, donor-yield latency,
  data-durability guarantees, or direct-test coverage.
- **Observability**: Every production component MUST emit structured logs,
  metrics, and traces sufficient to investigate principle violations after
  the fact. "We don't know what happened" is not an acceptable post-mortem
  conclusion.

## Governance

- **Supremacy**: This constitution supersedes all other practices, style
  guides, and conventions. Where a lower-level document conflicts with this
  constitution, this constitution wins.
- **Amendments**: Amendments MUST be proposed as pull requests that modify
  this file, MUST include a Sync Impact Report, MUST enumerate affected
  downstream templates and docs, and MUST be reviewed by the project's
  designated governance group (to be formally named at project start).
- **Versioning policy**: Semantic versioning applies to the constitution:
  - MAJOR: Backward-incompatible governance or principle removal/redefinition.
  - MINOR: New principle or materially expanded section.
  - PATCH: Clarifications, typo fixes, non-semantic refinements.
- **Compliance review**: A formal compliance review of the live system
  against Principles I–V MUST be performed at least quarterly once the
  cluster is serving real jobs, and the results MUST be published.
- **Emergency powers**: In response to an active Principle I (safety) or
  Principle II (data-loss) incident, designated on-call responders MAY
  halt cluster operations without prior governance approval; such actions
  MUST be reviewed retroactively within 7 days.
- **Runtime guidance**: Day-to-day development guidance (coding standards,
  review checklists, on-call runbooks) lives in separate documents under
  `docs/` and MUST be kept consistent with this constitution.

**Version**: 1.0.0 | **Ratified**: TODO(RATIFICATION_DATE): project has not
yet started; provisional ratification date 2026-04-15, to be confirmed or
amended by the founding governance group at project kickoff. | **Last
Amended**: 2026-04-15
