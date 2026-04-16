# Legal Entity — World Compute Project

## Status

**Placeholder — entity formation in progress.**

This document describes the intended legal structure for the World Compute
Project.  No entity has been formally incorporated as of the date of this
writing (2026).  The structure below reflects the target state and should be
updated when formation is complete.

## Intended Structure

| Field | Value |
|-|-|
| Entity type | 501(c)(3) public charity (IRC §501(c)(3)) |
| Jurisdiction | State of Delaware, USA |
| Model | Internet Security Research Group (ISRG) — single-purpose nonprofit steward |
| Registered agent | TBD upon incorporation |
| Principal office | TBD |

## Rationale

The ISRG model (operator of Let's Encrypt and Prossimo) was selected because:

- It establishes a credible, auditable nonprofit with a narrow charter focused
  solely on operating public-benefit infrastructure.
- It has demonstrated the ability to scale globally while maintaining fiscal
  accountability.
- Its board structure separates technical governance from financial governance,
  mirroring the World Compute two-body model (TSC + Board).

## Export Administration Regulations (EAR)

World Compute nodes may be located in multiple jurisdictions.  The following
EAR considerations apply:

- Compute workloads involving cryptography must be reviewed for compliance with
  the Commerce Control List (CCL) and EAR Part 740 license exceptions.
- Exports of software or technology to countries subject to embargo (Cuba,
  Iran, North Korea, Syria, Crimea, DRNK) are prohibited.
- The coordinator MUST enforce geographic deny-lists derived from the current
  OFAC Specially Designated Nationals (SDN) list.
- An annual EAR self-classification review should be conducted with outside
  counsel once the entity is formed.

## Office of Foreign Assets Control (OFAC)

- No funds may be received from, or disbursed to, entities or individuals on
  the OFAC SDN list.
- Donations must be screened against the SDN list at the time of receipt.
- Cloud and HPC providers used by the coordinator infrastructure must be
  domiciled in OFAC-compliant jurisdictions.

## Next Steps

1. Engage Delaware registered agent and file Certificate of Incorporation.
2. Draft bylaws (see `docs/governance/bylaws.md`).
3. File IRS Form 1023 for 501(c)(3) recognition.
4. Obtain EIN and open organizational bank account.
5. Engage outside counsel for EAR/OFAC compliance review.
