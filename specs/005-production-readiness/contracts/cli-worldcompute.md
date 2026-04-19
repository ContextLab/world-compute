# Contract: `worldcompute` CLI

**Scope**: New and mutated CLI commands introduced by spec 005. Existing commands (from specs 001–004) retain their contracts; this document lists only deltas.

## New flags on existing commands

### `worldcompute donor join`

| Flag | Default | Purpose | Spec ref |
|-|-|-|-|
| `--allow-ssl-inspection` | off | Trust local root CA for WSS-443 middleboxes; marks connection tier `Inspected` | FR-003, Edge Case |
| `--wss-listen` | off (on for relays) | Listen on port 443 for inbound WSS circuits | FR-007a |
| `--doh-only` | off | Skip OS resolver; use bundled DoH directly | FR-005 |
| `--allow-experimental-backbone <MODEL>` | off | Override backbone allowlist for diffusion nodes | Data-model E.1 |

### `worldcompute job submit`

Adds new subflags for distributed-diffusion inference:

| Flag | Default | Purpose |
|-|-|-|
| `--diffusion` | false | Dispatch a diffusion-inference request instead of a WASM workload |
| `--backbone <model_id>` | `GSAI-ML/LLaDA-8B-Instruct` | Select backbone |
| `--experts <id1,id2,...>` | automatic | Explicit expert selection |
| `--denoising-steps <N>` | 64 | Number of denoising steps |
| `--paradigms-block-size <N>` | 4 | ParaDiGMS parallel-block size |
| `--staleness <N>` | 1 | DistriFusion staleness bound |
| `--clipping-tau <F>` | 10.0 | PCG clipping bound |

## New top-level commands

### `worldcompute admin firewall-diagnose`

Runs the diagnostic sequence from issue #60 (libp2p debug log, dial attempts, transport negotiation). Emits a structured report at `evidence/phase1/firewall-traversal/<ts>/`.

### `worldcompute admin drift-check`

Manually runs the pinned-constant drift check (normally runs on a CI schedule). Exits 0 if all pinned values match upstream, 1 otherwise.

### `worldcompute admin verify-release <binary> <sig>`

Verifies a release binary against its detached Ed25519 signature using the pinned release public key. Wraps `scripts/verify-release.sh`.

## Exit codes (new)

- `0` — success
- `1` — general error (unchanged)
- `64` — placeholder detected in production code (used by `verify-no-placeholders.sh`; not a CLI exit but documented for consistency)
- `65` — reservation acquisition failed after all transports exhausted
- `66` — diffusion request failed convergence (ParaDiGMS non-convergence with sequential-fallback also failed)
- `67` — attestation chain rejected (real root mismatch, not zero-bypass)

## Stability

All new flags are additive. No existing CLI contract is broken. The `--diffusion` path for `job submit` is additive — existing WASM workflows continue to work.
