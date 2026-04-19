# Session 2026-04-18 — Production networking stack, dispatch protocols, and honest accounting

## What landed in spec 004 / PR #59

### Code

- **src/agent/daemon.rs**: full libp2p Swarm with the production NAT-traversal stack (TCP + QUIC + Noise + Yamux + mDNS + Kademlia + identify + ping + AutoNAT + Relay v2 server/client + DCUtR). AutoRelay reservations automatically triggered on `NatStatus::Private` by listening on `/<relay>/p2p-circuit`. Event loop handles peer connections, gossip messages, identify exchanges (feeding peer addresses into kademlia), DCUtR hole-punch results, relay reservation acceptance, and graceful shutdown on Ctrl-C.
- **src/network/discovery.rs**: `PUBLIC_LIBP2P_BOOTSTRAP_RELAYS` constant added with 5 Protocol Labs bootstrap addresses (each pinned by peer ID). `DiscoveryConfig::default()` merges these with worldcompute's own seeds so new daemons can rendezvous without operator infrastructure.
- **src/network/dispatch.rs**: two libp2p request-response protocols — `TaskOffer` (`/worldcompute/offer/1.0.0`) for lightweight capacity probing, `TaskDispatch` (`/worldcompute/dispatch/1.0.0`) for full job + result. CBOR serialization. 6 unit tests + 2 integration test suites.
- **src/cli/submitter.rs**: `execute_remote_submit()` — short-lived client swarm that dials an executor multiaddr, sends TaskDispatch, prints the result, exits. CLI flag: `worldcompute job submit --executor <multiaddr> --workload <wasm-path>`.
- **src/main.rs**: routes daemon-mode and remote-submit-mode to their async execution paths.

### Tests

- `tests/nat_traversal.rs` — 3-node in-process test: relay R + client A (NAT'd) + client B (NAT'd). A reserves a circuit at R; B dials A through R; B dispatches a real WASM job to A; A compiles + executes it with wasmtime; response returned to B as `TaskStatus::Succeeded`. Completes in ~5ms once connections are up. Proves every protocol step works.
- `tests/distributed_dispatch.rs` — 2-node direct dispatch over localhost.
- All 802 tests pass, zero failures, zero ignored.

### CI

PR #59 green across 7 checks: Tests (Linux), Tests (macOS), Tests (Windows), Sandbox (Linux KVM), Attestation (swtpm), Check / Test / Lint, Safety Audit Summary.

## What did NOT get verified

- **Real cross-machine WAN mesh formation** behind institutional firewalls. Attempted to run a daemon on tensor02.dartmouth.edu and dial public libp2p bootstrap relays (raw TCP probes succeeded, but the daemon got zero `ConnectionEstablished` events in 60+ seconds of observation, with no surfaced errors).
- **Real LLaMA inference** in the mesh LLM — `src/agent/mesh_llm/expert.rs::load_model()` is explicitly a placeholder. No candle-based inference, no real logit generation, no real tokens.
- **Pinned cryptographic values**: AMD ARK / Intel DCAP root CA fingerprints in `src/verification/attestation.rs` are `[0u8; 32]` → bypass mode. Rekor pinned public key in `src/ledger/transparency.rs` is also `[0u8; 32]` → signed tree head verification is skipped.
- **Firecracker rootfs** concatenates OCI layer bytes into a file; it does NOT run `mkfs.ext4` + loopback mount + tar extract, so real Firecracker boot would fail.
- **`admin_service::ban()`** returns `Ok(())` without updating the trust registry.
- **Platform adapters** (Slurm / K8s / Cloud) have parsers + scaffolds but no live-system integration tests.
- **Tauri GUI**, **Docker image**, **Helm chart**, **Apple VF helper binary** — files exist, none have been built or run.
- **REST gateway** routing logic exists but no HTTP listener is actually bound in the daemon.
- **Churn simulator** is a statistical model, not a real kill-rejoin harness over libp2p.

## Issue triage

Based on direct code verification:

**Closed (12 issues — fully addressed)**:
#31 Policy engine · #32 GPU passthrough · #35 Adversarial tests · #36 Integration test coverage · #44 Credit decay · #45 Preemption supervisor · #46 Confidential compute · #47 mTLS + rate limit · #48 Energy metering · #49 Storage GC + AU filter + residency · #50 Documentation · #55 Scheduler matchmaking

**Kept open with honest "partially addressed" comments (16 issues)**:
#28 Deep attestation · #29 Rekor Merkle proof · #30 Agent lifecycle (gossip wiring) · #33 Firecracker rootfs · #34 Incident containment (admin ban) · #37 Slurm adapter · #38 K8s adapter · #39 Cloud adapter · #40 Tauri GUI · #41 Docker/Helm · #42 LAN testnet · #43 REST gateway · #51 Churn simulator · #52 Apple VF Swift · #53 Reproducible builds · #56 Ledger Rekor anchor

**Kept open with "scaffolding in place; core deferred" comments (2 issues)**:
#27, #54 — mesh LLM (no real LLaMA inference)

**Opened new (1 issue)**:
#60 — cross-machine firewall traversal. This is the critical gap: the production NAT stack is validated in-process but not across real networks behind institutional / corporate firewalls. Will be the next spec.

## What changed in documentation

- CLAUDE.md: corrected overstated "Remaining Stubs: None" to list 16 known placeholder/scaffolding items with file pointers. Updated test count 784 → 802, date to 2026-04-18.
- README.md: rewrote the "Status notice" block. Removed "complete functional implementation" framing. Added honest lists of what's complete, what's scaffolded-with-placeholders, and what's the next critical issue (#60).
- whitepaper.md: added v0.4 entry correcting v0.3's overstatement. Lists scaffolded-but-placeholder items explicitly.

## Lessons

- Distinguishing "landed in PR" from "verified functional" matters. A passing test suite with mocked or localhost-only tests is NOT the same as verified production behavior.
- `cfg!(test)` + inline stubs that return `Ok(())` can make an issue LOOK addressed without actually addressing it.
- Institutional firewalls are a far bigger deployment blocker than any clever protocol work we ship. Solving #60 is worth more than any of the partially-addressed sub-issues.
- When writing issue close comments, it's worth reading the actual code of the function being claimed as complete — one `grep` for a function name isn't enough. Quick checks missed the `admin ban` stub, the `load_model` placeholder, and the zero-fingerprint bypass paths.
