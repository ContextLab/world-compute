# Quickstart — 15-minute path to a running donor behind a firewall

**Target**: A fresh Ubuntu 24.04 / macOS 14 / Windows 11 machine with zero prior World Compute setup. Success = the donor daemon is running, has one active relay reservation, and is reachable by dispatch from another peer. Deadline: 15 minutes wall clock (FR-042 / SC-008).

This quickstart corresponds to the spec-005 user journey from User Story 1 ("Cross-firewall mesh formation on real hardware"). It is also the operator path validated by `scripts/quickstart-timed.sh` in CI.

---

## Prerequisites

- 64-bit Linux/macOS/Windows machine with internet access.
- 4 GB free RAM, 5 GB free disk.
- `curl` and `tar` available.
- For Linux Firecracker support: KVM enabled (`ls /dev/kvm` returns without error).

No compiler or Rust toolchain required — the binary is prebuilt and signed.

## Step 1 — Download the signed release (1 min)

```bash
# Replace <ver> with the current release, e.g., 0.5.0
curl -fsSL https://github.com/ContextLab/world-compute/releases/download/v<ver>/worldcompute-$(uname -s | tr A-Z a-z)-$(uname -m).tar.gz -o wc.tgz
curl -fsSL https://github.com/ContextLab/world-compute/releases/download/v<ver>/worldcompute-$(uname -s | tr A-Z a-z)-$(uname -m).tar.gz.sig -o wc.tgz.sig
curl -fsSL https://raw.githubusercontent.com/ContextLab/world-compute/main/scripts/verify-release.sh -o verify-release.sh
chmod +x verify-release.sh
./verify-release.sh wc.tgz wc.tgz.sig       # verifies against pinned RELEASE_PUBLIC_KEY
tar xzf wc.tgz
```

Expected: `./worldcompute` binary exists and is executable.

## Step 2 — Create a donor identity (1 min)

```bash
./worldcompute donor enroll
```

Expected output:
```
Created donor identity: <peer_id>
Keystore: ~/.worldcompute/keys/
```

## Step 3 — Start the daemon (2 min)

```bash
./worldcompute donor join --daemon
```

Expected log lines within 60 seconds:
```
[info] peer_id=<...> listening on /ip4/0.0.0.0/tcp/19999
[info] peer_id=<...> listening on /ip4/0.0.0.0/udp/19999/quic-v1
[info] dialing bootstrap relay /dnsaddr/bootstrap.worldcompute.org/...
[info] connected to relay <peer_id>
[info] reservation_accepted: /p2p/<relay>/p2p-circuit/p2p/<self>
```

If the log reaches `reservation_accepted` within 60 s, Step 3 is done.

## Step 4 — If Step 3 failed (the firewall case) — enable WSS-443 fallback (2 min)

```bash
# Stop the daemon (Ctrl-C), then restart with automatic WSS-443 fallback enabled
./worldcompute donor join --daemon
# The daemon will automatically fall back to WSS-443 if TCP and QUIC are blocked.
# If you want to see the fallback happen, restart with:
RUST_LOG=info,libp2p_swarm=debug,libp2p_websocket=debug ./worldcompute donor join --daemon
```

Expected log additions:
```
[info] tcp dial to <...> failed: Connection refused (firewall?)
[info] quic dial to <...> failed: Connection refused (firewall?)
[info] falling back to wss/443 transport for bootstrap
[info] wss connection established, reservation_accepted
```

If your firewall also does SSL inspection, the daemon will refuse the connection by default. Opt in explicitly:
```bash
./worldcompute donor join --daemon --allow-ssl-inspection
```

Your connection will be marked `Inspected` and will run at a lower trust tier.

## Step 5 — Verify reachability from another peer (3 min)

From a different machine (colleague's laptop, home machine, cloud VM):

```bash
./worldcompute job submit --executor /p2p/<relay>/p2p-circuit/p2p/<your_peer_id> --workload https://example.com/hello.wasm
```

Expected output on the submitting machine:
```
job_id: <uuid>
status: Succeeded
result: "hello\n"
receipt_verified: true
```

## Step 6 — Check your donor status (2 min)

```bash
./worldcompute admin status
```

Expected:
```
peer_id:        <...>
connections:    3 (1 relay, 2 peers)
reservations:   1 active (expires in 55 min)
load:           cpu=0.12 gpu=0.00 mem=0.34
workloads:      1 completed (last 1h)
attestation:    TPM2-backed, tier=2
```

## Step 7 — Troubleshooting (4 min budget if needed)

| Symptom | Run | What to check |
|-|-|-|
| No `reservation_accepted` after 60 s | `./worldcompute admin firewall-diagnose` | Report written to `evidence/phase1/firewall-traversal/<ts>/`; share with project |
| `DialFailure` on every attempt | `./worldcompute admin status` | Verify `connections == 0` and outbound 443 is open |
| Attestation tier 0 | `./worldcompute admin status` | TPM2 may not be present; this is fine, just lower trust tier |
| Daemon exits immediately | Check logs | `production` cargo feature guard triggered — contact release engineer |

## Exit criteria (15-minute budget check)

- [ ] Binary downloaded, signature verified, extracted.
- [ ] Donor identity created.
- [ ] Daemon running with at least one reservation OR automatic WSS-443 fallback engaged.
- [ ] Remote peer successfully dispatched a real WASM workload and received a verified receipt.
- [ ] `./worldcompute admin status` reports green.

If all five boxes are checked in ≤ 15 minutes, SC-008 passes. The CI job `.github/workflows/quickstart-timed.yml` runs this exact script on fresh Ubuntu 24.04 / macOS 14 / Windows 11 images for each release candidate.
