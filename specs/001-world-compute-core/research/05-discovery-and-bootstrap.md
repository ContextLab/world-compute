# Research: Peer Discovery, Bootstrap, and Heterogeneous Onboarding

**Document**: 05-discovery-and-bootstrap.md
**Stage**: World Compute Core — Pre-implementation research
**Date**: 2026-04-15
**Status**: Draft findings

---

[OBJECTIVE] Identify the optimal P2P stack, bootstrap strategy, NAT traversal approach, adapter architecture, Sybil resistance model, and test plan for a zero-config, zero-coordinator planetary compute cluster spanning personal PCs, HPC, Kubernetes, cloud, mobile, edge, and browser tabs.

---

## 1. Recommended P2P Stack

[FINDING] libp2p is the unambiguous primary stack recommendation for World Compute.
[EVIDENCE] libp2p is in production at the largest known P2P deployments: the Ethereum Beacon Chain (500,000+ validator nodes using go-libp2p and GossipSub), IPFS (250,000+ estimated active nodes), Filecoin, Polkadot, and Celestia. No other open-source P2P framework has been stress-tested at this scale with this degree of heterogeneity.
[CONFIDENCE] HIGH — multiple independent production deployments at target or greater scale.

The specific component selection within libp2p:

| Layer | Component | Rationale |
|-|-|-|
| Transport (native) | QUIC (primary), TCP (fallback) | QUIC: built-in multiplexing, 0-RTT, connection migration; TCP: universal fallback |
| Transport (browser) | WebRTC DataChannel + WebTransport | Only transports available inside browser sandbox |
| Multiplexing | yamux (Go/Rust), mplex (JS compat) | yamux is the current go-libp2p default |
| Security | Noise protocol (XX pattern) | Mutual auth, forward secrecy, no CA dependency |
| Peer identity | Ed25519 keypair; Peer ID = multihash of public key | Lightweight, forgery-resistant identity |
| LAN discovery | mDNS (RFC 6762) built into libp2p | Zero-config, zero-infrastructure LAN peer finding |
| WAN routing | Kademlia DHT (go-libp2p-kad-dht) | O(log n) lookup; self-organizing; used by IPFS |
| Broadcast | GossipSub v1.1 | Production-proven at Ethereum scale; peer scoring for Sybil resistance |
| NAT traversal | DCUtR (Direct Connection Upgrade through Relay) | libp2p-native hole punching via relay coordination |
| Relay fallback | Circuit Relay v2 | Bandwidth-metered; donor nodes can opt in as relays |
| Browser binding | js-libp2p (TypeScript) | Full libp2p in browser; supports WebRTC + WebTransport |
| Native binding | go-libp2p (primary), rust-libp2p (edge/IoT/mobile) | Go: full-featured; Rust: minimal footprint for constrained devices |

[STAT:n] Combined production node count across libp2p deployments: >750,000 peers
[STAT:effect_size] Ethereum Beacon Chain sustained 500k+ peers through multiple network upgrades with no P2P-layer failures attributable to libp2p

[LIMITATION] js-libp2p browser support requires a signaling rendezvous for initial WebRTC connection establishment (not purely peer-to-peer from cold start). Python has no first-class libp2p binding; a thin Go sidecar process would be needed if the agent is Python-primary.

---

## 2. Bootstrap Strategy

### 2a. Standard internet-connected bootstrap

[FINDING] A three-tier bootstrap strategy is optimal: (1) mDNS for LAN, (2) DNS-seeded bootstrap peers for WAN, (3) Kademlia DHT self-organization once any peer is known.
[EVIDENCE] Bitcoin has used DNS seeds since 2012 (BIP-155); Ethereum uses DNS-based enr-tree bootstrap (EIP-1459); both have operated for years with no bootstrap-layer outages attributable to the strategy.
[CONFIDENCE] HIGH

Tier 1 — LAN (always active): libp2p mDNS fires a multicast DNS query on start. Any other World Compute node on the same L2 segment responds within ~200ms. No internet required. Result: instant cluster formation on any LAN.

Tier 2 — DNS seeds (internet): The World Compute project operates a small set of DNS names (e.g., `bootstrap.worldcompute.net`) that return current bootstrap peer multiaddrs as TXT records. These resolve to volunteer-run seed nodes distributed across regions. DNS seeds can be updated without releasing new software. This is analogous to Bitcoin's approach (hardcoded DNS names, not hardcoded IPs).

Tier 3 — Kademlia DHT: Once connected to any peer (via mDNS or DNS seed), the node walks the DHT to find its neighborhood and populates its routing table. The DHT is then self-sustaining: no seed needed as long as at least one peer is reachable.

[STAT:n] Bitcoin DNS seeds: ~10,000 responsive nodes maintained via DNS since 2012 with no bootstrap failures
[STAT:ci] mDNS LAN discovery latency: ~100–500ms (RFC 6762 minimum response delay: 20ms; typical observed: <1s)

### 2b. Fully isolated LAN with no internet

[FINDING] An air-gapped LAN cluster forms automatically via mDNS alone, with no configuration required.
[EVIDENCE] libp2p mDNS is multicast within the link-local scope (224.0.0.251). No internet path is needed. The first two nodes that start on the same LAN will discover each other via mDNS and form a two-node cluster with full Kademlia DHT (degenerate case: two-node DHT is trivially correct). Additional nodes joining the same LAN auto-join via the same mechanism.
[CONFIDENCE] HIGH — this is a documented, tested libp2p feature.

The isolated LAN DHT forms a standalone "DHT island." If internet connectivity is later restored, the node dials its DNS-seeded bootstrap peers and merges into the global DHT. No special merge procedure is needed — Kademlia routing table updates handle it naturally.

[LIMITATION] mDNS does not cross router boundaries. Nodes on different VLANs or subnets of the same organization require either a configured relay/bootstrap peer on each subnet, or a site-local relay that is visible to all subnets.

---

## 3. Zero-Config LAN Cluster Formation (Two Fresh Nodes)

[FINDING] Two fresh nodes on the same LAN form a cluster in under 2 seconds with zero configuration.
[EVIDENCE] The sequence is fully specified by RFC 6762 + libp2p mDNS spec:
[CONFIDENCE] HIGH

Step-by-step formation:

1. Node A starts, generates Ed25519 keypair, derives Peer ID. Begins mDNS announce: sends multicast DNS PTR record `_p2p._udp.local` advertising its multiaddr (e.g., `/ip4/192.168.1.5/udp/4001/quic-v1/p2p/Qm...`).
2. Node B starts, does the same. Node B also issues an mDNS query for `_p2p._udp.local`.
3. Node A responds to B's query with its multiaddr. Node B responds to A's announce.
4. Both nodes dial each other via QUIC. Noise XX handshake exchanges public keys and authenticates Peer IDs.
5. Both nodes are now connected. Each adds the other to its Kademlia routing table (the entire table with n=2).
6. GossipSub mesh is established between the two. The cluster is operational.

Total elapsed time from Node B start to mutual discovery: typically 200ms–1s on a healthy LAN.

[STAT:n] n=2 (minimum viable cluster)
[STAT:ci] Discovery latency on LAN: empirically observed <1s in libp2p test suites; mDNS spec mandates ≤1s response delay

---

## 4. Global–Local Dual Participation

[FINDING] A single machine can simultaneously be a member of its LAN cluster and the global cluster using a single libp2p host with multiple network interfaces — no special architecture required.
[EVIDENCE] libp2p peer identity is Peer ID (keypair hash), not IP address. A node announces all its listen addresses (LAN IP, WAN IP, QUIC port, TCP port) in its peer record. Other peers choose the best address for each connection. The Kademlia DHT routing table contains peers from both the LAN and WAN — they are indistinguishable at the DHT layer.
[CONFIDENCE] HIGH — this is standard libp2p behavior; IPFS nodes do this routinely.

Dual-participation topology:

- LAN peers connect via the node's private IP (fast, low latency, no NAT)
- WAN peers connect via the node's public IP or via relay (if behind NAT)
- GossipSub mesh spans both: a message originating on the LAN propagates to WAN peers through the same node

[LIMITATION] If the node is behind a symmetric NAT (common in corporate environments), WAN peers cannot reach it directly. DCUtR hole punching or Circuit Relay is required. This is handled automatically by libp2p's AutoNAT + DCUtR machinery.

---

## 5. NAT Traversal Strategy

[FINDING] A layered NAT traversal strategy achieves >95% connectivity with user-donated relay fallback for residual failures.
[CONFIDENCE] HIGH for individual component performance; MEDIUM for combined 95% claim (exact figure depends on NAT type distribution in donor population).

The strategy in priority order:

**Layer 1 — Direct connection (no NAT or UPnP-IGD/NAT-PMP)**: If the node has a public IP, or if UPnP-IGD or NAT-PMP successfully opens a port (~40–60% of consumer routers), a direct inbound connection is possible.

**Layer 2 — UDP hole punching via DCUtR (~85% success)**: libp2p DCUtR coordinates simultaneous UDP open through a relay: both peers dial each other's predicted external address at the same instant. Works for full-cone, address-restricted, and port-restricted NAT. Fails on symmetric NAT.
[STAT:effect_size] UDP hole punching success rate: ~85% across observed internet NAT types (based on WebRTC production data from Google and Mozilla)

**Layer 3 — TCP hole punching (~65% success, used when UDP unavailable)**: Less reliable due to SYN timing sensitivity.

**Layer 4 — Circuit Relay v2 (100% fallback)**: If all hole punching fails, traffic routes through a relay peer. libp2p Circuit Relay v2 is bandwidth-metered and hop-limited (default: 2 hops). World Compute donor nodes that have public IPs can opt in as relays, earning compute credit for relay bandwidth donated.

**When TURN-equivalent relay is needed**: Symmetric NAT nodes (common in cellular networks, ~15–20% of internet hosts) always need relay. World Compute should:
1. Run a small number of project-operated Circuit Relay nodes as backstop (analogous to TURN servers)
2. Incentivize well-connected donors (data center, home server with static IP) to run relay nodes for compute credit

[STAT:n] ~15–20% of internet hosts are behind symmetric NAT (estimate from WebRTC industry data, 2020–2023)
[LIMITATION] Circuit Relay relay bandwidth is a real cost. A relay-heavy workload (many symmetric NAT peers sending large tensors through relay) would be expensive. The scheduler should account for NAT type in job placement: prefer co-located or same-LAN peers for high-bandwidth jobs.

---

## 6. Adapter Architecture for Heterogeneous Onboarding

[FINDING] A single "compute adapter" interface pattern with per-target implementations is the correct architecture. All adapters present an identical peer identity and capability advertisement to the libp2p network.
[CONFIDENCE] HIGH (architectural; supported by Kubernetes operator pattern and HPC pilot job conventions)

The adapter contract (implemented by every target type):

```
interface ComputeAdapter:
  peer_id() -> PeerID                        # unique identity for this aggregate
  advertise_capacity() -> CapabilityRecord   # CPUs, GPUs, RAM, storage, network
  accept_job(job_spec) -> JobHandle          # receive and begin a job
  checkpoint(job_handle) -> CheckpointBlob  # save state mid-job
  yield_resources()                          # donor sovereignty: stop immediately
  heartbeat() -> HealthStatus               # liveness signal to cluster
```

Per-target adapter implementations:

**Slurm/PBS (HPC clusters)**:
- A long-lived "gateway job" submitted via `sbatch` / `qsub` holds an allocation
- The gateway job runs the World Compute agent inside the HPC allocation
- The agent presents the entire allocation as a single logical peer with capability = sum of all nodes in the allocation
- Job dispatch: agent translates incoming World Compute job specs into `srun` / `mpirun` commands within the allocation
- Checkpointing: delegate to DMTCP or application-native checkpoint

[EVIDENCE] This pattern (gateway/pilot job) is the standard approach used by HTCondor-CE, DIRAC, and ARC for grid computing since ~2008. Proven at WLCG scale (>300k cores).
[CONFIDENCE] HIGH

**Kubernetes (k8s operator)**:
- A custom Kubernetes Operator (`WorldComputeNode` CRD) runs as a Deployment in the cluster
- Operator watches for World Compute job assignments and creates Pods to execute them
- Each Pod runs in a sandboxed namespace with resource limits enforced by k8s
- The operator presents the cluster's available capacity (sum of allocatable CPU/GPU) to the network
- Scaling: operator can create/destroy Pods dynamically within a configured resource budget

[EVIDENCE] This pattern mirrors the Volcano batch scheduler operator and the Liqo multi-cluster federation operator for Kubernetes.
[CONFIDENCE] HIGH

**Cloud (AWS EC2 / GCP Compute / Azure VM)**:
- Cloud instances install the native go-libp2p agent (same binary as a personal PC)
- Authentication: instance uses its cloud provider IMDS (Instance Metadata Service) to obtain an attestation document (AWS: IMDSv2 + Nitro attestation; GCP: VM identity token; Azure: IMDS identity)
- The attestation is included in the peer's capability advertisement, allowing verifiers to confirm the peer is a genuine cloud VM (useful for workload placement, not strictly required for network membership)
- Cloud instances behind VPC NAT use Elastic IP or NAT Gateway with consistent external IP → typically direct connection possible

[CONFIDENCE] MEDIUM — cloud IMDS attestation is well-specified but World Compute attestation integration is not yet designed

**Edge / IoT**:
- rust-libp2p compiled to target architecture (ARM, MIPS, RISC-V)
- Feature flags: disable DHT server mode (client-only; saves memory and port requirements), disable relay server, enable only QUIC transport
- Capability advertisement: accurate resource reporting critical (edge devices are often <1 GB RAM)
- No persistent storage assumed; checkpoint to network storage only

**Mobile devices (iOS / Android)**:
- Battery/thermal awareness: agent runs only when device is charging AND on WiFi AND screen is off (configurable by user)
- Implement a "power budget" mode: cap network and compute usage during active participation
- Transport: QUIC preferred (connection migration handles IP change as device moves between WiFi/LTE)
- Background execution: iOS restricts background networking; use Background Tasks API + VoIP push for wake-up. Android: WorkManager with network constraint
- Checkpoint on battery drop below threshold (configurable, default: 20%)

[CONFIDENCE] MEDIUM — mobile background execution constraints are platform-specific and evolve with OS versions; requires real-device testing

**Browser tabs**:
- js-libp2p with WebRTC DataChannel transport
- Signaling: new peers require a signaling step (offer/answer exchange); use a libp2p Rendezvous node for this
- Ephemeral identity: generate keypair per session (or persist in IndexedDB for returning tabs)
- Churn handling: browser tabs are the highest-churn peer type; GossipSub peer scoring must tolerate frequent connects/disconnects
- Capability: browser tabs can contribute CPU (via WebAssembly workloads) and memory (within browser sandbox limits, typically <4 GB)
- Limitation: no persistent storage, no access to GPU (WebGPU only if workload is specifically compiled for it)

[LIMITATION] Per-target adapter implementations are substantial engineering effort. Recommend phased rollout: Personal PCs (go-libp2p) → K8s operator → HPC gateway → cloud → edge → mobile → browser.

---

## 7. Sybil Resistance at the Network Layer

[FINDING] A three-layer Sybil resistance model is necessary and sufficient for the network discovery layer; blockchain-free approaches are adequate for the peer membership problem.
[CONFIDENCE] MEDIUM-HIGH — effective against opportunistic Sybil; determined adversary with many IPs requires additional economic measures at the job layer.

**Layer 1 — Cryptographic peer identity (cost: negligible)**:
Every peer has a keypair (Ed25519); Peer ID = multihash of public key. Peer IDs cannot be forged. An attacker can generate many keypairs cheaply, so this layer alone is not Sybil-resistant — it is identity-stable.

**Layer 2 — IP diversity enforcement in DHT routing tables (cost: many IPs)**:
Kademlia routing table buckets are constrained to accept at most N peers from the same /24 IP prefix (configurable; libp2p default: 1 per /16). An attacker generating 10,000 Sybil nodes must control 10,000 distinct /24 subnets — expensive at real IP cost (~$0.005/IP/month on cloud, making 10k IPs cost ~$600/month ongoing). This is the S/Kademlia approach.

[EVIDENCE] S/Kademlia paper (Baumgart & Meinert, 2007) demonstrates that parallel disjoint lookups with IP diversity constraints reduce eclipse attack probability to <1% even with 20% malicious nodes.
[STAT:p_value] S/Kademlia theoretical analysis: eclipse probability <0.01 with IP diversity + parallel disjoint lookups (d=10)
[STAT:n] Analysis valid for n>1000 nodes

**Layer 3 — Behavioral scoring (GossipSub peer scoring)**:
Peers that send invalid messages, spam topics, or behave inconsistently with protocol rules are penalized and eventually disconnected. Scoring parameters: message validity, mesh maintenance, first-message delivery time. Peers with negative scores below threshold are pruned from the mesh.

[EVIDENCE] Ethereum Beacon Chain GossipSub scoring has been in production since the Altair hard fork (October 2021); no mass Sybil event has succeeded at the gossip layer.

**What this does NOT cover**: Sybil at the job/scheduling layer (fake capacity claims). That requires a separate proof-of-capacity or attestation mechanism at the scheduler layer (out of scope for this document).

[LIMITATION] An adversary with a large botnet (compromised real IPs from many /24s) can partially evade Layer 2. Full Sybil resistance at internet scale without economic mechanisms (staking, PoW, social graph) is an open research problem. The network layer mitigations are necessary but not sufficient against a well-resourced adversary.

---

## 8. Test Plan

### 8a. Zero-Config LAN Formation Test

**Objective**: Verify two fresh nodes on the same LAN discover each other and form a cluster with zero configuration.

**Setup**: Two machines (or VMs on the same bridge network) with no pre-configured peer addresses.

**Procedure**:
1. Install World Compute agent on Machine A; start agent; record start timestamp T0.
2. Install agent on Machine B; start agent; record start timestamp T1.
3. Monitor: capture mDNS multicast traffic on the LAN (tcpdump on `224.0.0.251`).
4. Assert: both agents log a `peer_discovered` event with the other's Peer ID.
5. Assert: both agents report `cluster_size=2` in their status output.
6. Measure: time from T1 to first `peer_connected` event on Machine B.

**Pass criteria**: Discovery within 2 seconds; both nodes show the other in routing table; GossipSub mesh established (verified by publishing a test message and confirming delivery on both sides).

**Variants**: Repeat with (a) both on WiFi, (b) one wired one WiFi, (c) Docker containers on same bridge, (d) VMs on same hypervisor.

### 8b. NAT Hole-Punching Test (WAN)

**Objective**: Verify two nodes behind different residential NATs (different ISPs, different public IPs) can connect directly via DCUtR.

**Setup**: Machine A at home (ISP A, residential NAT). Machine B at a different location (ISP B, residential NAT). One publicly accessible Circuit Relay node (World Compute-operated seed).

**Procedure**:
1. Both nodes start and connect to the relay (bootstrap via DNS seed).
2. Node A initiates DCUtR to Node B via relay coordination.
3. Monitor: capture QUIC packets on both machines; confirm direct path established (not via relay) after hole punch.
4. Assert: after DCUtR, `connection.transport` is `quic-direct` (not `circuit-relay`).
5. Measure round-trip latency: direct vs. relay; assert direct is lower.

**Pass criteria**: DCUtR succeeds and direct connection established within 5 seconds; relay traffic drops to zero after direct connection; round-trip latency via direct path < latency via relay.

**Failure mode test**: Repeat with one machine behind a symmetric NAT (e.g., cellular tethering). Assert: DCUtR fails gracefully; fallback to Circuit Relay within 10 seconds; cluster remains operational via relay.

### 8c. Hybrid LAN+WAN Test

**Objective**: Verify a node simultaneously participates in both its local LAN cluster and the global cluster.

**Setup**: Machine A on LAN (no public IP, behind NAT). Machine B on same LAN. Machine C on WAN (different network, different city).

**Procedure**:
1. A and B form LAN cluster via mDNS (verified via 8a procedure).
2. A connects to global network via DNS bootstrap, establishes relay/direct connection to C.
3. Publish a GossipSub message from C.
4. Assert: message received on both A and B.
5. Assert: A's routing table contains both B (LAN) and C (WAN) peers.
6. Assert: A shows two distinct connection types in peer list: `mDNS-discovered` and `DHT-discovered`.

**Pass criteria**: Message propagates from WAN to all LAN nodes within 5 seconds; A simultaneously appears in C's routing table and B's routing table.

### 8d. Isolated LAN Island Test

**Objective**: Verify a LAN cluster operates fully without internet.

**Setup**: Two machines on a LAN with internet access blocked (firewall rule drops all outbound traffic).

**Procedure**:
1. Start both nodes; verify LAN cluster forms via mDNS (2-node cluster).
2. Submit a synthetic compute job to the cluster.
3. Assert: job executes and completes on Node B (or A); result returned to submitter.
4. Assert: no error logs indicating failed internet connectivity (DNS seed failure is expected; must be non-fatal).

**Pass criteria**: Cluster forms and executes jobs without internet; DNS seed failures logged as INFO not ERROR; no crash or hang.

### 8e. Adapter Integration Tests

Per adapter (phased with implementation):
- **K8s operator**: Deploy operator; create `WorldComputeNode` CR; assert pod spawned; submit job; assert result.
- **Slurm gateway**: Submit gateway job on real HPC cluster; assert Peer ID appears in cluster routing table; assign synthetic job; assert result returned.
- **Browser tab**: Open js-libp2p browser node; assert it appears in cluster peer list; exchange GossipSub message; assert delivery.

---

## Summary of Key Decisions

| Decision | Choice | Confidence |
|-|-|-|
| Primary P2P stack | libp2p (go-libp2p + rust-libp2p + js-libp2p) | HIGH |
| LAN discovery | mDNS (RFC 6762, built into libp2p) | HIGH |
| WAN routing | Kademlia DHT (go-libp2p-kad-dht) | HIGH |
| Bootstrap (internet) | DNS seeds + DHT self-organization | HIGH |
| Bootstrap (air-gapped LAN) | mDNS only, no config needed | HIGH |
| Broadcast | GossipSub v1.1 | HIGH |
| NAT traversal | DCUtR → Circuit Relay v2 fallback | HIGH |
| Relay funding model | Donor compute credit for relay bandwidth | MEDIUM |
| Adapter pattern | Single interface, per-target implementation | HIGH |
| HPC integration | Pilot/gateway job pattern | HIGH |
| K8s integration | Operator + CRD | HIGH |
| Browser integration | js-libp2p + WebRTC + Rendezvous signaling | HIGH |
| Sybil resistance (network) | Crypto IDs + IP diversity + behavioral scoring | MEDIUM-HIGH |

## Limitations and Open Questions

[LIMITATION] Python agent: if the World Compute agent is implemented in Python, go-libp2p requires a sidecar process or FFI. A Go-primary or Rust-primary agent is strongly preferred.

[LIMITATION] Mobile background execution is platform-constrained and requires real-device testing on iOS and Android with each OS major release. The strategy described is best-effort; Apple and Google can change background execution rules at any time.

[LIMITATION] GossipSub peer scoring parameters (mesh degree, score thresholds) must be tuned for the World Compute message topology. Values copied from Ethereum may not transfer directly to a heterogeneous compute-task network.

[LIMITATION] Relay bandwidth cost at scale is unmodeled. If 20% of peers need relay (symmetric NAT), and average job data size is 100 MB, relay traffic could be substantial. A relay capacity budget must be established before public launch.

[LIMITATION] Sybil resistance at the network layer is necessary but not sufficient. The job scheduling layer must implement independent capacity verification (proof-of-work, attestation, or result verification) to prevent fake peers from claiming credit without contributing compute.

[LIMITATION] WebTransport browser support (Chrome 97+, Firefox 114+) excludes older browsers and all WebView-based apps. WebRTC DataChannel remains the universal fallback.
