# Security Incident Disclosure Policy

Version: 1.0.0  
Status: Ratified  
Reference: FR-082, World Compute Constitution §7

## 1. Purpose

This policy establishes the process for discovering, triaging, remediating,
and publicly disclosing security vulnerabilities in the World Compute network.
It applies to all software components, network protocols, governance smart
contracts, and infrastructure operated by the World Compute Foundation.

## 2. Scope

- All software in the `world-compute` monorepo (agent daemon, CLI, coordinator, gateway)
- P2P network protocol (libp2p transport, gossip, DHT)
- On-chain governance contracts
- Hosted infrastructure (coordinators, bootstrappers, telemetry aggregators)

## 3. Reporting a Vulnerability

**Private disclosure (preferred):**  
Email `security@world-compute.org` with subject `[VULN] <one-line summary>`.  
PGP key: `https://world-compute.org/.well-known/security-pgp.asc`

**Bug bounty:**  
Critical and high-severity issues qualify for bounty rewards per the
Foundation's Bug Bounty Programme (`https://world-compute.org/security/bounty`).

**Response SLA:**

| Severity | Acknowledgement | Triage Complete | Patch Released |
|-|-|-|-|
| Critical | 24 h | 72 h | 7 days |
| High | 48 h | 7 days | 30 days |
| Medium | 7 days | 14 days | 90 days |
| Low | 14 days | 30 days | Next minor release |

## 4. Severity Classification

| Level | Criteria |
|-|-|
| Critical | Remote code execution, sandbox escape, key material exfiltration, ledger corruption |
| High | Privilege escalation, node impersonation, quorum manipulation, DoS of coordinator |
| Medium | Information disclosure, rate-limit bypass, degraded confidentiality |
| Low | Minor information leakage, cosmetic trust-score manipulation |

## 5. Coordinated Disclosure Timeline

1. **Day 0** — Reporter contacts `security@world-compute.org`.
2. **Day 0–3** — Foundation acknowledges receipt; assigns severity and triage lead.
3. **Day 0–7** — Triage lead reproduces and confirms vulnerability.
4. **Day 7–N** — Patch developed and reviewed (N per severity SLA above).
5. **Day N** — Patch deployed to all coordinators and bootstrappers.
6. **Day N+7** — Public advisory published (CVE requested if applicable).
7. **Day N+7** — Reporter credited in advisory unless anonymity requested.

The Foundation may accelerate this timeline for actively-exploited vulnerabilities
(0-day in the wild). In that case, a partial advisory may be published immediately
with full technical details withheld until patch is deployed.

## 6. Evidence Artifact Requirements

Every confirmed vulnerability must produce:

- A security evidence artifact conforming to `docs/security/evidence-schema.md`.
- A reproduction test committed to `tests/adversarial/` under `#[ignore]`.
- A linked entry in the governance ledger incident log.

## 7. Responsible Disclosure Expectations

Reporters are expected to:
- Not exploit the vulnerability beyond proof-of-concept demonstration.
- Not disclose to third parties until the coordinated disclosure date.
- Provide enough detail to reproduce the issue.

The Foundation commits to:
- Not pursue legal action against good-faith researchers.
- Credit reporters unless anonymity is requested.
- Provide bounty payments within 30 days of patch release.

## 8. Post-Incident Review

Within 14 days of patch release the triage lead must publish an internal
post-mortem covering: root cause, detection gap, remediation steps, and
process improvements. A redacted version is included in the public advisory.

## 9. Policy Updates

This policy is versioned in the `world-compute` repository. Material changes
require a governance vote per FR-090 (simple majority of active coordinators).
