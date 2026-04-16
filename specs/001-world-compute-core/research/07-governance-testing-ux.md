# 07 — Governance, Testing Strategy, and User-Facing UX

**Research Date**: 2026-04-15
**Constitution Version**: 1.0.0
**Stage**: Pre-implementation research

---

## 1. Governance and Funding

### Surveyed Models

[FINDING] The most durable public-good compute and infrastructure projects share a small set of structural patterns: nonprofit legal entity with a clear mission lock, diversified revenue across many small donors plus a few institutional grants, and a technical steering committee that separates day-to-day decisions from constitutional changes.
[EVIDENCE] Survey of nine comparable projects: Mozilla Foundation, Wikimedia Foundation, Let's Encrypt/ISRG, Linux Foundation, Apache Software Foundation, Folding@home, CERN/WLCG, Tor Project, and blockchain DAOs (Gitcoin, Optimism Collective).
[CONFIDENCE] High — these are well-documented, multi-decade governance records.

**Mozilla Foundation**: 501(c)(3) holding entity with a for-profit subsidiary (Mozilla Corporation) that generates revenue via search-engine contracts. The separation allowed commercial activity while keeping the mission locked. Weakness: revenue concentration — when Google search deals compress, the entire foundation is at risk. Mozilla employs ~700 people; the for-profit subsidiary requires complex conflict-of-interest management.

**Wikimedia Foundation**: 501(c)(3), funded almost entirely by small individual donations (avg ~$15), augmented by institutional grants. Extremely mission-stable because no single donor exceeds ~1–2% of revenue. Governance via elected board plus appointed seats. Strong transparency (public financial reports, Wikimedia Foundation Annual Report). Weakness: historically slow technical execution; governance overhead is high.

**Let's Encrypt / ISRG**: 501(c)(3) with a small board of directors and a clearly scoped technical mission (free TLS). Funding is diversified across ~50 corporate sponsors, from $1M/yr Platinum to $10K Bronze tiers. Mission lock is tight — ISRG can only do public-key infrastructure work. This focus is a strength: no mission drift. Technical decisions made by a small core engineering team; the board sets financial policy, not technical policy.

**Linux Foundation**: 501(c)(6) trade association — not a charity. Members pay dues tiered by company size ($20K–$500K/yr). Governance is explicitly member-weighted: larger dues buyers get more board seats. This model funds staff well but creates structural tension between large-donor influence and community interests. Not recommended for World Compute — donor-sovereignty (Principle III) is incompatible with a dues-weighted board.

**Apache Software Foundation**: 501(c)(3) with a flat membership model (invited individuals, not companies). Decisions are made at the project level by project management committees (PMCs). The "Apache Way" — consensus, transparency, community over code — is genuinely mission-protective. Weakness: very conservative; innovation is slow. The governance model is excellent for software stewardship but was not designed for operating infrastructure.

**Folding@home**: Academic consortium hosted at Washington University in St. Louis. No independent legal entity; the project rides on the university's nonprofit status and grant-funding infrastructure. This is workable for a small research project but fragile at scale — a change in university policy or principal investigator can end the project. Not recommended for a project intending planetary-scale permanence.

**CERN / WLCG**: Inter-governmental organization (IGO) with member-state funding. Not replicable for a volunteer project without state backing. The WLCG grid uses a federated resource-sharing model (pledged CPU-years per institution) that is structurally similar to what World Compute will need, but the political and legal machinery is inaccessible to a grassroots project.

**Tor Project**: 501(c)(3), historically overly concentrated on US government grants (60–70% revenue from DARPA/NSA/State Dept at peak). This created mission-credibility problems. Has since diversified toward individual donations and corporate sponsors. Lesson: government grant concentration is a strategic risk for a privacy/trust-critical project.

**Blockchain DAOs (Gitcoin, Optimism Collective)**: Marketing promises of "decentralized governance" have largely not survived contact with reality. Gitcoin's quadratic funding experiments are genuinely interesting for grant distribution but the governance token model concentrates voting power in early holders. Optimism Collective's bicameral Token House / Citizens' House is the most sophisticated attempt to date. Practical lesson: on-chain governance works for distributing grants from a treasury; it does not replace a legal entity, cannot hold employment contracts, cannot be sued, and cannot own infrastructure. DAOs are a funding/grant tool, not a governance replacement.
[EVIDENCE] Public financial disclosures, IRS Form 990 filings, and published post-mortems for each organization.
[CONFIDENCE] High for structural analysis; medium for exact financial figures which shift year to year.

### Legal Entity Recommendation

[FINDING] A US 501(c)(3) public charity is the recommended legal entity for World Compute, structured with a tightly scoped mission statement ("operate and improve a volunteer decentralized compute cluster as a public good") and bylaws that prevent mission drift.
[EVIDENCE] 501(c)(3) provides: US tax-deductibility for domestic donors (largest donor pool globally), access to Google Ad Grants (~$120K/yr free advertising), GitHub and AWS nonprofit credits, eligibility for most US and international science foundations, and established legal precedent for technology nonprofits. The ISRG model (a 501(c)(3) with a narrow technical mission and corporate sponsor tiers) is the closest analogue and has proven durable.
[CONFIDENCE] High for US-based founding team. If the founding team is European, a Dutch Stichting or Swiss Verein is the alternative — both are respected internationally and compatible with GDPR by default.

**Why not Swiss Verein or Stichting as primary?** These are excellent structures but require European founding presence, are less familiar to US institutional donors, and do not provide US tax deductibility without a parallel US entity. The added complexity is not warranted unless the founding team is European or expects primary funding from European institutions.

### Donations and Financial Transparency

[FINDING] Donations should be accepted in fiat currency by default, with crypto accepted as a secondary channel with immediate conversion to fiat — not held as a treasury asset.
[EVIDENCE] Wikimedia, EFF, and Internet Archive all accept crypto but convert immediately. Holding crypto creates volatility risk in operating budget and introduces regulatory complexity. Crypto donations are a meaningful supplemental channel (~5–15% for comparable orgs) but should not drive financial planning.
[CONFIDENCE] High.

Recommended donation structure:
- Individual donations: Open Collective or Stripe (low overhead, transparent public ledger)
- Corporate sponsors: Tiered (Sustaining $100K+/yr, Supporting $25K+/yr, Contributing $5K+/yr), structured as charitable donations not membership dues (avoids 501(c)(6) creep)
- Grants: Apply to NSF, NIH (for scientific workloads), Mozilla Foundation, Open Technology Fund, Alfred P. Sloan Foundation
- Tax deductibility: 501(c)(3) status provides this automatically for US donors; use fiscal sponsorship via Software in the Public Interest or Open Collective Foundation during the pre-incorporation period

All income and expenditure must be published quarterly in a machine-readable format (CSV + human-readable PDF). This is non-negotiable for donor trust.

### Fund Allocation Priorities

Recommended priority ordering for fund allocation:
1. Security audits (external, independent, scope: sandbox, agent binary, scheduler, network protocols) — minimum 20% of annual budget until post-GA
2. Core developer compensation (2–4 engineers at market-rate; underpaying burns out critical contributors)
3. Infrastructure for CI/testing (the direct-test requirement of Principle V is expensive; testnet hardware, cloud bursting for integration tests)
4. Hardware for testing (donor-class machines: budget laptops, Raspberry Pis, older phones, diverse GPU classes)
5. Legal / compliance (~5% of budget; ongoing)
6. Community and outreach (~10%)

### Decision-Making Structure

[FINDING] The recommended governance structure is a Technical Steering Committee (TSC) of 5–7 individuals for day-to-day technical decisions, a Board of Directors of 5 for financial and legal decisions, with constitutional amendments requiring a 2/3 supermajority of both bodies.
[EVIDENCE] This two-body model is used by Node.js Foundation, OpenJS Foundation, and CNCF projects. It separates technical merit decisions from fiduciary ones, preventing either a financial donor or a technical faction from controlling the whole organization.
[CONFIDENCE] High.

TSC membership: merit-based, elected by active contributors (defined as anyone who has merged code, filed verified bug reports, or donated verified compute-hours above a threshold in the past 12 months). No company may hold more than 2 of 7 TSC seats. No TSC member may simultaneously be a board member.

Board membership: 5 directors — 2 elected by TSC, 2 elected by individual-donor membership (donors above $100 cumulative), 1 independent director selected by unanimous board vote.

Conflict of interest when a major donor wants priority compute: this is explicitly prohibited by the mission statement and the fairness principle (Principle III). The bylaws must state that compute allocation is governed by the technical scheduler, not by donation tier. Donors may purchase compute via the submitter pathway like any other user; their donation does not confer scheduling priority. This must be stated in every major-donor agreement.

### Transparency

- Quarterly financial reports published within 30 days of quarter end
- Annual IRS Form 990 published immediately upon filing
- TSC meeting minutes published within 7 days of meeting
- All security incident disclosures per the constitution's pre-committed timeframe
- Open Collective public ledger (real-time income/expense visibility) strongly recommended

---

## 2. Testing Strategy Before Public Release

[FINDING] A five-phase staged rollout with explicit metric gates and mandatory kill conditions is required by Principle V. Each phase must produce a direct-test evidence artifact before the next phase begins. No phase may be skipped.
[EVIDENCE] Principle V of the constitution: "A failing or unverifiable direct test MUST block deployment." The phase structure below operationalizes this requirement on real hardware.
[CONFIDENCE] High for the structure; specific metric thresholds are estimates calibrated against comparable distributed-systems projects (IPFS, libp2p, Tor Browser) and should be validated against actual measurements.

### Phase 0: Single-Machine Smoke Tests (Laptop)

**Goal**: Verify that the agent installs, sandboxes a trivial workload, returns a correct result, and cleans up — all on one machine, no networking.

**Hardware**: Any developer laptop (macOS, Linux, Windows via WSL2). At least one test on a machine with no admin/sudo to verify privilege-drop.

**Gate metrics**:
- Agent installs from source and reproducible binary in < 5 min
- Sandbox starts and stops within 2 seconds on a cold machine
- Trivial workload (SHA-256 of a known file) returns correct result 100/100 runs
- No files left outside scoped working directory after job completion (verified by filesystem snapshot diff)
- Agent process drops to unprivileged UID after initialization

**Adversarial tests required at this phase**:
- Workload attempts to read /etc/passwd (must be blocked, not just invisible)
- Workload attempts to write to /tmp outside its scoped directory (must fail)
- Workload exits with non-zero code; verify agent reports failure and cleans up

**Kill conditions**: Any sandbox escape, any host-file read, any privileged-process persistence.

### Phase 1: 3–5 Machine LAN Testnet (Founder Home Network)

**Goal**: Verify peer discovery, job scheduling across nodes, fault recovery when one node disappears mid-job.

**Hardware**: 3–5 physical machines (not VMs of VMs — the sandbox runs inside a VM; the host must be bare metal or real hardware). Diverse OS and CPU architecture preferred (at least one ARM machine, e.g., Raspberry Pi 5 or Apple Silicon laptop).

**Gate metrics**:
- Peer discovery succeeds on a NAT'd home network (no manual IP configuration)
- Job completes successfully when submitted from a non-participating machine
- Node failure mid-job: job reschedules and completes within 2x expected wall-clock time
- Checkpointing: job killed at 50% completion, resumed from checkpoint on a different node, produces identical result to uninterrupted run
- Resource yield: cluster workload pauses within 1 second of simulated user activity (keyboard event injection)
- No cross-node data leakage: node A cannot read workload data from node B via any discovered path

**Adversarial tests required**:
- Malicious peer (one machine configured to send malformed protocol messages): cluster must isolate it, not crash
- Replay attack: resubmit completed job ID, verify scheduler detects and rejects
- Large-input job: workload requests more memory than scoped cap; agent must kill workload, not OOM the host

**Kill conditions**: Any cross-node sandbox breach, host OOM on any machine, data loss from simulated node failure.

### Phase 2: 20–50 Machine Federated Testnet (Early Collaborators)

**Goal**: Verify behavior across heterogeneous hardware, diverse network conditions, and non-cooperating administrators (each testnet participant controls their own machine; they are not all on the same LAN).

**Hardware**: Solicited from university research groups, trusted open-source contributors, and team members' personal hardware. Must include: low-end machines (4GB RAM, spinning disk), high-end machines (GPU), machines behind CGNAT, machines in at least 3 geographic regions. All participants give explicit written consent with full disclosure of what workloads will run.

**Gate metrics**:
- Cluster sustains > 80% job completion rate over a 72-hour continuous run
- Churn tolerance: cluster survives 30% simultaneous node departure without data loss
- Scheduler places jobs correctly by capability (GPU workload goes to GPU node, not CPU node)
- Contributor accounting: each node's contribution is recorded correctly; verified by cross-checking node-local logs against coordinator log
- Network partition recovery: two network segments merge after 30-minute split; no duplicate job execution, no data loss

**Adversarial tests required**:
- Sybil attack simulation: one participant registers 10 fake node identities; verify they are rate-limited or require proof-of-hardware
- Flooding: one participant submits 1000 jobs simultaneously; verify scheduler queues them without crashing or starvation of other submitters
- Byzantine node: one node returns incorrect results; verify the verification layer detects it within N re-runs

**Kill conditions**: Data loss from churn, verified Byzantine node that is not detected, any host machine affected by a workload outside the scoped directory.

### Phase 3: 500–5000 Public Alpha (Explicit Consent, Isolated Workloads)

**Goal**: Stress-test at scale with real volunteers. All workloads are isolated, synthetic or clearly-scoped scientific tasks (no sensitive data). All alpha participants sign a clear consent form disclosing the experimental nature.

**Gate metrics**:
- Job completion rate > 90% over 30-day rolling window
- P99 job latency within 3x expected wall-clock time
- Zero security incidents (Principle I violations) — this is a hard binary gate
- Resource yield latency < 1 second P99 across all reported donor machines
- At least one independent security audit completed and critical/high findings remediated
- Energy and carbon footprint published (Principle IV)

**Adversarial tests required**:
- External penetration test of the scheduler API surface
- Red-team exercise: a security researcher attempts sandbox escape on a dedicated test machine (with full cooperation and explicit consent)
- Denial-of-service against the control plane: verify graceful degradation, not total outage

**Kill conditions**: Any real-world sandbox escape, any host-data exfiltration, failure of independent security audit to clear critical findings.

### Phase 4: General Availability

GA is gated on: Phase 3 metrics sustained for 30 days, external security audit clearance, legal entity fully incorporated, governance structure operational (TSC and board seated), public-facing incident-disclosure policy published and tested with at least one drill.

---

## 3. CLI and GUI (User-Facing)

### CLI Framework

[FINDING] Rust with the `clap` crate is the recommended CLI framework for the single World Compute binary (`wc` or `worldcompute`).
[EVIDENCE] The agent binary must be code-signed and reproducibly built (Principle I). Rust produces small, statically-linked binaries with no runtime dependency, making reproducible builds straightforward (cargo's lock files + `cargo auditable`). `clap` is the dominant Rust CLI framework (>50M downloads/month on crates.io), with excellent subcommand ergonomics, shell-completion generation, and man-page generation. Go + cobra is a strong alternative — similar binary properties, slightly larger output — but Rust's memory-safety guarantees are directly relevant to a security-critical agent binary. Python click is ruled out: a Python runtime dependency on donor machines is a significant attack surface and installation complexity.
[CONFIDENCE] High for Rust/clap; Go/cobra is an acceptable alternative if the core team has stronger Go expertise.

One binary serves donors, submitters, and administrators via subcommand groups:
```
worldcompute donor join|status|pause|resume|withdraw|logs
worldcompute job submit|status|cancel|results|list
worldcompute admin propose|vote|report|audit
worldcompute config set|get|validate
```

### Desktop GUI Framework

[FINDING] Tauri is the recommended desktop GUI framework.
[EVIDENCE] Tauri uses the OS's native WebView (WebKit on macOS, WebView2 on Windows, WebKitGTK on Linux) rather than bundling Chromium. This means: (1) installer is ~3–10 MB vs. Electron's ~80–150 MB; (2) memory footprint is ~50–150 MB vs. Electron's ~200–400 MB; (3) critically, no bundled browser engine means a dramatically reduced attack surface on donor machines (Principle I direct implication — an Electron install ships a full browser runtime including V8, which has a historically rich CVE history). The Tauri backend is Rust, consistent with the CLI recommendation. The frontend is standard HTML/CSS/JS (React or Svelte), enabling web skills reuse.
[CONFIDENCE] High. The security surface argument is particularly strong given Principle I.

Qt (native per-platform): superior native feel and no WebView dependency, but requires Qt licensing consideration (LGPL is usable but has constraints), and the C++ build complexity increases the reproducible-build burden. Ruled out as primary recommendation; consider for a later "native" variant.

Flutter: single codebase for desktop + mobile is appealing, but Flutter's desktop rendering is not native — it draws its own widgets — and the Dart/Flutter binary is not as straightforward to audit as Rust. Acceptable if the team has Flutter expertise and mobile is a priority from day one.

### Mobile App

[FINDING] Mobile donor participation should be a Phase 3 feature, not a launch requirement, due to thermal and battery constraints. When built, Flutter is recommended for iOS+Android from a single codebase.
[EVIDENCE] Principle III requires that cluster workloads pause immediately on battery/thermal pressure. Mobile operating systems (iOS, Android) enforce aggressive thermal throttling and background process killing that makes sustained compute donation unreliable — the agent would be constantly suspended. The correct model for mobile is monitoring/management (pause, resume, view credits, submit jobs) not donation. Flutter provides one codebase for both platforms with good accessibility support. Native per-platform (Swift/Kotlin) would be higher quality but doubles the maintenance burden for a small team.
[CONFIDENCE] High for deferral; medium for Flutter vs. native (depends on team skills).

### Web UI and Browser Donor Mode

[FINDING] A hosted web dashboard is a launch requirement; browser-based compute donation via WASM + WebGPU is a Phase 3 stretch goal.
[EVIDENCE] The web dashboard (React + TypeScript, served as a static SPA from a CDN) requires no special capabilities — it's a management interface. Browser compute donation is technically feasible: WASM provides near-native CPU performance, WebGPU provides GPU access in Chrome/Edge/Firefox (with caveats), and WebRTC + WebTransport can handle peer communication. However, the sandbox isolation model in a browser context is fundamentally different from the OS-level + hypervisor sandbox required by Principle I. Browser workers are isolated from the host DOM but share the browser process's memory space in ways that are harder to audit. This must be treated as a distinct security model with its own audit, not as equivalent to the native agent sandbox.
[CONFIDENCE] High for dashboard launch priority; medium for browser-donate feasibility (WebGPU compatibility is still uneven across platforms as of 2026).

### Unified API

[FINDING] gRPC for internal/programmatic clients + a REST/HTTP JSON API for web and third-party integrations is the recommended API strategy. Both expose the same operations; gRPC is primary for the CLI and agent-to-agent communication; REST is primary for the web dashboard and external integrations.
[EVIDENCE] gRPC provides: strong typing via protobuf (single source of truth for API schema), efficient binary serialization (important for result download), bidirectional streaming (job log tailing, real-time status), and auto-generated clients in every major language. REST/HTTP JSON provides: universal accessibility (curl, browser fetch, any HTTP library), simpler debugging, and compatibility with standard API gateways. Maintaining both is feasible via gRPC-gateway (a protobuf plugin that generates a REST reverse-proxy from the same .proto definitions), keeping the schema authoritative in one place.
[CONFIDENCE] High.

### Accessibility and i18n

All UI surfaces must meet WCAG 2.1 AA as a launch gate. i18n must be built into the web dashboard from day one (react-i18next or equivalent); retrofitting i18n is significantly more expensive than building it in. The CLI should support UTF-8 output and localized error messages in at least English and one other language at launch, with a clear contribution pathway for community translations. Mobile apps must follow platform accessibility guidelines (iOS Accessibility, Android Accessibility).

---

## 4. Public API Sketch

### Donor API

```
POST   /v1/donors/join              # Register a new donor node; returns node_id + keypair
GET    /v1/donors/{node_id}/status  # Current status: online, paused, resource utilization
PATCH  /v1/donors/{node_id}/config  # Update preferences: job_classes, cpu_cap, gpu_cap, schedule
POST   /v1/donors/{node_id}/pause   # Pause donation (cluster clears active jobs gracefully)
POST   /v1/donors/{node_id}/resume  # Resume donation
DELETE /v1/donors/{node_id}         # Withdraw: wipe node state, revoke keypair, close account
GET    /v1/donors/{node_id}/credits # Contribution and consumption accounting ledger
GET    /v1/donors/{node_id}/logs    # Structured job execution logs for this node
```

CLI equivalent:
```
worldcompute donor join [--cpu-cap 50%] [--gpu-cap 30%] [--schedule "22:00-08:00"]
worldcompute donor status
worldcompute donor pause
worldcompute donor withdraw
```

### Submitter API

```
POST   /v1/jobs                     # Submit a job; body: job.yaml or job.json spec
GET    /v1/jobs/{job_id}            # Job status, progress, estimated completion
GET    /v1/jobs/{job_id}/logs       # Streaming log output (SSE or gRPC stream)
POST   /v1/jobs/{job_id}/cancel     # Cancel in-progress job; partial results retained
GET    /v1/jobs/{job_id}/results    # Download results (presigned URL or streaming)
GET    /v1/jobs                     # List submitter's jobs with filter/pagination
POST   /v1/jobs/{job_id}/retry      # Resubmit failed job (inherits original spec)
```

Job spec (language-agnostic YAML):
```yaml
apiVersion: worldcompute/v1
kind: Job
metadata:
  name: protein-fold-batch-001
spec:
  image: oci://registry.worldcompute.org/science/alphafold:2.3.2
  resources:
    cpu: "4"
    memory: "16Gi"
    gpu: optional
  job_class: scientific          # donor opt-in classes
  input:
    source: s3://my-bucket/inputs/
  output:
    destination: s3://my-bucket/outputs/
  checkpointing: enabled
  max_wall_time: 24h
```

CLI equivalent:
```
worldcompute job submit job.yaml
worldcompute job status <job_id>
worldcompute job logs <job_id> --follow
worldcompute job results <job_id> --output ./results/
worldcompute job cancel <job_id>
```

### Admin / Governance API

```
POST   /v1/governance/proposals          # Submit a constitution amendment or policy proposal
GET    /v1/governance/proposals          # List open proposals
POST   /v1/governance/proposals/{id}/vote # Cast a vote (TSC or board member)
GET    /v1/governance/reports            # List published compliance/financial reports
POST   /v1/governance/reports            # Publish a new quarterly report (board auth)
GET    /v1/governance/incidents          # List security incidents and their disclosure status
POST   /v1/admin/halt                    # Emergency cluster halt (on-call responder auth)
POST   /v1/admin/resume                  # Resume after halt (board auth)
GET    /v1/admin/nodes                   # Cluster-wide node registry (paginated)
POST   /v1/admin/nodes/{node_id}/ban     # Ban a node for policy violation
```

CLI equivalent:
```
worldcompute admin propose amendment.md
worldcompute admin vote <proposal_id> --approve
worldcompute admin report publish report.pdf
worldcompute admin halt --reason "active P0 sandbox incident"
```

---

## Limitations and Open Questions

[LIMITATION] The governance model recommendations assume a US-based founding team. European, APAC, or distributed founding teams should evaluate Stichting (Netherlands) or Verein (Switzerland) as alternatives with lower US-legal-system dependency.

[LIMITATION] The staged testing phase timelines are not specified in calendar time because they depend on team size and available hardware. A two-person founding team with 3 machines will spend longer in Phase 0–1 than a ten-person team with a university compute cluster available.

[LIMITATION] The 501(c)(3) recommendation assumes the project is not primarily a trade association for corporate members. If corporate membership dues become the primary revenue model, 501(c)(6) status is more appropriate — but this fundamentally changes the governance dynamics and conflicts with Principle III (donor sovereignty over paying institutional users).

[LIMITATION] Browser-based compute donation (WASM + WebGPU) is described as a stretch goal, but the security model must be independently audited before enabling it. The browser sandbox is not equivalent to the OS-level + hypervisor sandbox required by Principle I, and treating them as equivalent would be a constitutionviolation.

[LIMITATION] The API sketch above is a design target, not an implementation specification. Actual endpoint signatures, authentication schemes (mTLS for node-to-node, OAuth2/JWT for user-facing), rate limits, and versioning policy require a full API design document.

[LIMITATION] Mobile donor participation has not been fully analyzed for regulatory implications — some jurisdictions may classify sustained background compute donation as a form of contracted service, with employment or tax implications for the donor. Legal review is needed before enabling mobile donation.

---

*Document classification: Research / Pre-implementation. All findings are design recommendations subject to revision during implementation.*
