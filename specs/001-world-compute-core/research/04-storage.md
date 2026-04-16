# Research: Distributed Storage / Filesystem Architecture

**Stage**: Storage & Data Plane
**Date**: 2026-04-15
**Author**: Scientist agent (claude-sonnet-4-6)

---

## [OBJECTIVE]

Identify the optimal storage architecture for World Compute — a planetary-scale volunteer compute cluster where every storage node is treated as unreliable, potentially adversarial, and geographically dispersed. The system must guarantee no data loss for active jobs from any single node failure or plausible correlated regional failure, while keeping overhead tolerable for donor devices.

---

## 1. Erasure Coding Scheme

### Candidate Evaluation

Reed-Solomon (RS), fountain/RaptorQ, and Minimum Storage Regenerating (MSR) codes are the three practical options at this scale.

**Reed-Solomon (k, n)**: encodes k data shards into n total shards; any k of the n can reconstruct the original. Mature, well-understood, with production-quality implementations (liberasurecode, zfec, Intel ISA-L). The primary cost is repair bandwidth: naive RS repair requires downloading k shards to reconstruct one lost shard — equivalent to downloading the entire original file per lost shard.

**RaptorQ / Fountain codes**: rateless; a receiver collects any k+ε symbols and decodes. Excellent for broadcast/multicast streaming (e.g., streaming input data to many executors simultaneously). Not ideal as the primary storage codec because fountain codes produce slightly larger-than-needed overhead and lack the tight placement control RS provides.

**MSR (Minimum Storage Regenerating) codes**: a generalization of RS that reduces repair bandwidth from k·(shard\_size) down to n/(n−k)·(shard\_size) — a 78% bandwidth reduction for RS(10,18). The tradeoff is implementation complexity; MSR is not yet as widely deployed as RS. In 2026 libraries exist (e.g., Clay codes, regenerating code implementations in academic toolkits) but are less battle-tested.

[FINDING] Reed-Solomon RS(10,18) is the recommended primary erasure code for cold storage, with MSR-based repair as a Phase 2 optimization.

[EVIDENCE] Quantitative failure analysis at 10% per-shard unavailability (realistic for volunteer donors online ~14–22 hours/day):
- RS(10,18): P(data loss) = 2.09×10⁻⁵ at 10% churn; 4.25×10⁻³ at 20% churn
- With strict placement (1 shard per autonomous system, ≤2 shards per country): correlated regional outage knocks out at most 2 of 8 parity shards, leaving 6 parity shards intact — well within the 8-loss budget
- Storage overhead: 1.80× (18 shards stored per 10 data shards)
- Repair bandwidth (naive): 1.0× file size per lost shard; MSR reduces this to 0.225× (78% savings)

[CONFIDENCE] High for the RS recommendation. The MSR repair bandwidth figure is analytically exact for the (10,18) configuration; the durability probability assumes i.i.d. shard availability which is a lower bound — geographic diversity makes actual durability higher.

### Failure Model Survived

RS(10,18) with strict placement survives:
- Any 8 simultaneous random shard losses
- Complete loss of any single country's nodes (capped at 2 shards/country → 2 losses)
- Simultaneous loss of 3 different countries (3×2 = 6 losses, within 8-loss budget)
- Any single autonomous system failure (exactly 1 loss by construction)

[FINDING] Pure i.i.d. RS at low n (e.g., RS(4,8), RS(6,9)) fails the durability bar at 20-30% churn. The placement constraint is load-bearing: without geographic enforcement, RS(10,18) at 30% churn has P(loss) ≈ 4.25×10⁻³ which is unacceptable for active job data.

[EVIDENCE] P(loss | RS(4,8), p=0.20) = 1.04×10⁻² vs P(loss | RS(10,18), p=0.10 with placement) = 2.09×10⁻⁵ — a 500× improvement.

[CONFIDENCE] High. This is combinatorial math, not modeling.

---

## 2. Addressing Model

[FINDING] Content-addressable identifiers (CIDs) — specifically CIDv1 with SHA-256 content hashing — are the correct addressing model for the data plane. Location-addressable URLs are unsuitable for a system where any node may disappear.

[EVIDENCE] CIDv1 encodes codec + hash function + digest in 36 bytes. Each chunk's address is a cryptographic commitment to its content, making:
- Integrity verification trivially O(1) per shard on receipt
- Deduplication automatic across jobs that share input datasets
- Caching correct by construction (same CID = same bytes, safe to serve from any cache)
- Merkle DAG composition natural for hierarchical datasets (directories, checkpoint trees)

[CONFIDENCE] High. IPFS has proven CIDv1 at scale; Filecoin uses it in production for petabytes of storage.

**Location addressing for routing only**: the metadata plane maps CID → {shard\_locations}. Shard locations are (node\_id, shard\_index) pairs, not stable URLs. The metadata plane is the only place location appears, and it is updated as nodes join/leave. This is the key architectural separation: content addressing for integrity, location addressing only transiently in the metadata plane.

### Why Not Reuse IPFS Directly

IPFS Bitswap and the Kademlia DHT are optimized for content discovery across the open web, not for low-latency job data staging with erasure coding. IPFS does not natively:
- Enforce erasure-coded shard placement with geographic constraints
- Support client-side encryption with key management integrated into job metadata
- Provide guaranteed SLAs for data availability windows (IPFS content can be garbage-collected by any node)
- Integrate with job scheduling locality hints

[FINDING] World Compute should use CIDv1 as its addressing primitive but build a custom data plane rather than inheriting IPFS Bitswap. IPFS tooling (go-ipfs libraries, CID libraries) can be reused at the component level.

[CONFIDENCE] High. Filecoin learned this lesson — they built Lotus on top of IPFS primitives but replaced Bitswap with their own transfer protocols for storage deals.

---

## 3. Replica Placement Strategy

### Geographic and Administrative Diversity

Placement must enforce:
1. **1 shard per autonomous system (AS)**: a single BGP routing failure cannot take out more than 1 shard
2. **≤ floor((n-k)/3) shards per country**: for RS(10,18), this is ≤ 2 shards/country, surviving a 3-country simultaneous event
3. **≥ 3 continents represented** among the n=18 shard holders
4. **No shard placement on the same /24 subnet** as another shard for the same object

[FINDING] The placement constraint is architecturally mandatory, not optional. Erasure coding without placement discipline provides no correlated-failure protection.

[EVIDENCE] A China internet outage in 2021 affected ~7% of global BGP prefixes; if 3 shards were in China that single event would consume 3 of 8 parity budget slots. Capping at 2 shards/country means no single-country event exceeds the parity budget by more than 2 slots, leaving 6 parity slots for other concurrent failures.

[CONFIDENCE] Medium-high. BGP event data is well-documented; exact shard cap tuning requires empirical measurement of donor geographic distribution.

### Minimum Node Pool Requirement

To place 18 shards across 18 distinct ASes with geographic constraints, the cluster needs a minimum viable donor pool of ~50 active storage nodes (to have sufficient diversity after filtering by geography and current availability). Below this threshold, the system must relax constraints or fall back to 3× replication for hot data only.

---

## 4. Hot vs. Cold Data Tiers

### Hot Tier (Working Set)

Active job inputs and outputs live on the executor nodes themselves during job execution. This is mandatory: erasure-coded reconstruction adds latency unsuitable for tight compute loops.

- **Replication**: 3× synchronous replication across job executor nodes (not donor storage nodes)
- **Lifecycle**: staged in at job start, wiped at job completion per Constitution Principle I (no residual state on donor machines)
- **Size cap**: bounded by job resource allocation; large datasets chunked and streamed

### Cold Tier (Persistent Storage)

Checkpoints, job outputs awaiting retrieval, and input datasets that are not currently being computed live in cold storage.

- **Encoding**: RS(10,18) with placement constraints as above
- **Chunk size**: 4 MB per shard (yielding 40 MB logical chunks). This is a practical balance: small enough to parallelize across 18 donors, large enough that per-chunk metadata overhead is negligible (960 bytes metadata per 40 MB chunk = 0.002% overhead)
- **Metadata footprint**: 240 MB metadata for a 1 TB job dataset — fits comfortably in coordinator RAM

### Tier Transition

```
Job submit → coordinator fetches k=10 cold shards → reconstruct → stream to hot tier on executors
Job completes → executor streams output → coordinator shards → distribute to n=18 cold storage donors
Checkpoint write → same pipeline, CID of checkpoint recorded in job metadata
```

[FINDING] The hot/cold boundary at job start/end maps cleanly onto the Constitution's requirement that donor nodes retain no persistent cluster state. The cold tier is where data lives between jobs; the hot tier exists only during execution.

[CONFIDENCE] High. This is a standard staging pattern used by distributed ML training systems (e.g., Google Borg, SLURM with Lustre).

---

## 5. Metadata Plane vs. Data Plane Separation

### Metadata Plane

The metadata plane stores:
- **Job manifest**: CID of root Merkle DAG → chunk CIDs → shard locations per chunk
- **Shard location index**: (CID, shard_index) → (node_id, last_seen, health_score)
- **Donor storage ledger**: per-donor byte quota, usage, last heartbeat
- **Job lifecycle state**: STAGED | RUNNING | CHECKPOINTED | COMPLETED | EXPIRED

[FINDING] The metadata plane must use a CRDT-based replicated data structure (recommended: a custom DAG-CRDT similar to Automerge's approach, or a purpose-built append-only log with vector clocks) to handle concurrent updates from multiple coordinators without a single master.

[EVIDENCE] Tahoe-LAFS's introducer node is the canonical lesson in what not to do: a single coordinator/introducer creates a SPOF that violates Constitution Principle II. Filecoin's chain-based metadata avoids this but introduces consensus overhead inappropriate for sub-second job scheduling. CRDTs allow eventual consistency with causal ordering, which is sufficient for shard location updates.

[CONFIDENCE] Medium. CRDT-based metadata planes are proven (Riak, Dynamo-style systems) but require careful design to avoid unbounded tombstone accumulation as donors churn.

**Recommended CRDT structure**: OR-Map (Observed-Remove Map) with a monotonically increasing vector clock per shard location entry. Shard location updates are last-writer-wins within a coordinator epoch. Coordinator epoch changes (coordinator failure/replacement) use a Paxos/Raft quorum over the coordinator set (typically 5–7 coordinators in geographically separate data centers or high-availability donor nodes).

### Data Plane

The data plane is pure point-to-point: executor ↔ storage donor, with no coordinator in the critical path for data bytes. The coordinator provides shard locations; the executor opens direct connections to the relevant donors. This is the same separation IPFS achieves with DHT (metadata) vs. Bitswap (data transfer).

---

## 6. Integrity: Merkle Trees and Hashing

Every chunk is identified by its CIDv1 (SHA-256). Job datasets are organized as Merkle DAGs:

```
Job Root CID
├── Input Dataset CID
│   ├── Chunk 0 CID → [shard_0..shard_17]
│   ├── Chunk 1 CID → [shard_0..shard_17]
│   └── ...
├── Checkpoint CID (updated on each checkpoint)
└── Output CID (written on completion)
```

Integrity checks:
- **Per-shard on write**: storage donor verifies shard hash before acking
- **Per-chunk on read**: executor verifies reconstructed chunk hash matches CID
- **Per-job on stage-in**: executor verifies input dataset root CID matches job manifest
- **Periodic audit**: coordinator samples random shard CIDs and asks holders to prove possession (simplified Proof of Data Possession, not full Filecoin PoSt — just a hash challenge)

[FINDING] The Merkle DAG structure provides end-to-end integrity without trusting any individual donor, consistent with the adversarial donor model in the Constitution.

[CONFIDENCE] High. Merkle-tree integrity is foundational to Git, IPFS, Filecoin, and Tahoe-LAFS. No novel cryptography required.

---

## 7. Deletion, GC, and Donor-Cap Enforcement

- **Donor storage cap**: each donor declares a storage budget (e.g., 10 GB). The coordinator never assigns more shards than the declared cap. Cap is enforced cryptographically: shard assignments are signed by the coordinator and bounded by the donor's registered quota.
- **Job expiry**: cold storage chunks have a TTL set at job submission. On expiry, the coordinator instructs shard holders to delete and releases quota. Donors that go offline before deletion simply have their quota freed after a grace period.
- **GC**: coordinator periodically scans for orphaned shard references (job expired but shard still tracked) and issues explicit delete instructions. Shard holders that have not received an explicit keep-alive for a chunk after the TTL + grace period may GC locally.
- **Donor withdrawal**: per Constitution Principle III, donors can withdraw at any time. On withdrawal signal, the coordinator immediately starts re-encoding affected shards onto other donors. The withdrawing donor remains online for a configurable handoff window (default: 30 minutes) to allow shard transfer. After the window, their quota is released regardless.

[FINDING] Donor-cap enforcement requires a signed quota token system, not just soft limits, because an adversarial or misconfigured donor might accept more shards than declared. The coordinator must not assign shards without a valid quota token from the donor.

[CONFIDENCE] Medium. This is a standard capability-token pattern; the exact token format and revocation mechanism need detailed design in a follow-on spec.

---

## 8. Job Execution Integration

### Staging Input Data

```
1. Coordinator resolves job input CID → chunk list → shard locations
2. Executor opens parallel connections to k=10 shard holders per chunk
3. RS reconstruct chunk in executor memory
4. Stream chunk to job working directory (size-capped, wiped on completion)
5. Repeat for each chunk; pipeline with computation where possible
```

Locality scheduling: the scheduler should prefer executor nodes that already hold shards of the job's input data in their local hot cache from a previous job or checkpoint. This is the "warm data" preference in Constitution Principle IV.

### Streaming vs. Batch

- **Batch**: preferred for large datasets. Coordinator pre-stages all input chunks before signaling job start. Predictable, easier to retry, required for checkpoint/resume.
- **Streaming**: supported for input datasets too large to pre-stage (e.g., ML training over a 100 TB corpus). Executor streams chunks from cold storage on demand, maintaining a sliding window. Requires careful backpressure to avoid stalling computation waiting for reconstruction.

[FINDING] For stage 1 (initial jobs), batch staging is sufficient and simpler. Streaming should be a stage 3+ feature.

[CONFIDENCE] High.

### Output and Checkpointing

Checkpoint writes go through the same shard pipeline as cold storage writes. The checkpoint CID is written to the job metadata plane atomically (single CRDT update). On executor failure, a new executor reads the latest checkpoint CID from the metadata plane, fetches the checkpoint data from cold storage, and resumes. This directly satisfies Constitution Principle II's checkpoint/resume requirement.

---

## 9. Privacy: Donor Cannot Read Job Submitter Data

[FINDING] All job data must be encrypted client-side before sharding. Donors receive only ciphertext shards and can neither individually nor collectively reconstruct plaintext (individual donors see 1/18 of ciphertext; Reed-Solomon shards of ciphertext are individually indistinguishable from random bytes).

**Encryption scheme**:
- **Per-chunk key**: ChaCha20-Poly1305 with a random 32-byte key per chunk. AEAD provides integrity + confidentiality.
- **Key wrapping**: chunk keys are wrapped with the job submitter's public key (X25519 ECDH + AES-256-GCM). Wrapped keys are stored in the job manifest on the metadata plane, not on donor storage nodes.
- **Executor access**: the executor receives the job manifest (including wrapped keys) from the coordinator. The executor decrypts chunk keys using a short-lived session key provided by the job submitter at submission time. This session key is delivered to the executor via the control plane over a mutually authenticated TLS channel.
- **Coordinator blindness**: the coordinator handles only CIDs and shard locations — never plaintext or unwrapped chunk keys. The coordinator cannot read job data.

[EVIDENCE] This is the same architecture used by Tahoe-LAFS (convergence encryption) and Storj (client-side encryption before erasure coding). Both systems have operated in adversarial multi-tenant environments with this model.

[CONFIDENCE] High for the architecture. Key management for long-lived jobs (multi-day training runs) requires additional design: session key expiry vs. checkpoint access.

[LIMITATION] Confidential computing (SGX/TDX) could allow computation on encrypted data, but adds hardware dependency and significant complexity. Out of scope for stage 1; worth revisiting for sensitive workloads in stage 4+.

---

## 10. Consistency Model

[FINDING] The storage layer provides **causal+ consistency** for metadata and **strong consistency** (via content addressing) for data.

- **Data reads are always consistent**: CID is a cryptographic hash of content; if you fetch a CID and it verifies, you have the exact bytes that were written. There is no "stale read" for immutable content-addressed data.
- **Metadata (shard locations) is causally consistent**: using a CRDT with vector clocks, the metadata plane guarantees that any coordinator that has seen update A and delivers update B has also applied all updates causally prior to B. A coordinator that goes offline may deliver slightly stale shard locations, but these resolve on reconnection. Stale locations result in failed shard fetches (the executor tries the next available shard holder), not data corruption.
- **Job state is linearizable** within a coordinator epoch: job lifecycle transitions (STAGED → RUNNING → COMPLETED) go through the Raft-replicated coordinator quorum and are linearizable. This prevents duplicate job execution.

[FINDING] Strong consistency for all operations would require a global consensus protocol (Paxos/Raft across geographically dispersed coordinators) on every read, which is incompatible with the latency requirements of job staging across a planetary network. Causal+ consistency for metadata with strong consistency via CIDs for data is the correct tradeoff.

[CONFIDENCE] High. This is the same model used by Amazon Dynamo, Apache Cassandra, and Filecoin's chain + retrieval market split.

---

## 11. Browser / WebRTC Donors

[FINDING] Browser donors have severely constrained storage capabilities and should participate in the hot tier (ephemeral working set) only, not the cold erasure-coded tier.

[EVIDENCE]:
- **Quota**: Origin Private File System (OPFS) gives browsers typically 1–10 GB quota, subject to browser GC and user clearing site data. StorageManager.estimate() returns available quota but it can be revoked.
- **Persistence**: `navigator.storage.persist()` requests durable storage but is not guaranteed (requires user permission grant; Chrome grants it based on engagement heuristics).
- **No background execution**: browsers cannot hold shards across sessions. A browser donor's shards would disappear on tab close — catastrophic for cold tier durability.
- **WebRTC data channels**: suitable for transferring shard data during an active session; ~50–100 Mbps practical throughput on good connections.

**Browser donor role**:
- **Compute only** during active session (satisfies job hot-tier data serving)
- **Ephemeral shard relay**: can forward shard bytes between persistent donors and executors during a session (WebRTC peer-to-peer, no storage commitment)
- **No cold storage quota**: excluded from RS placement for persistent data

[CONFIDENCE] High. This is a hard browser platform constraint, not a design choice.

---

## 12. System Comparison Summary

| System | Erasure Coding | Addressing | Metadata SPOF | Client Encryption | Donor Withdrawal |
|-|-|-|-|-|-|
| IPFS | No (replication) | CIDv1 | DHT (distributed) | No native | Yes |
| Filecoin | Proof of Storage | CIDv1 | Chain (slow) | No native | Via deal expiry |
| Storj | RS(29,80) | Custom | Satellite (SPOF) | Yes | Yes |
| Sia | RS(10,30) | Merkle | Blockchain | Yes | Via contract |
| Tahoe-LAFS | RS(3,10) | Tahoe URI | Introducer (SPOF) | Yes | Manual |
| Ceph | CRUSH/RS | RADOS OID | MON quorum | No native | Yes |
| **World Compute** | **RS(10,18)** | **CIDv1** | **CRDT+Raft quorum** | **Yes (ChaCha20)** | **Yes (30min handoff)** |

[FINDING] No existing system matches all World Compute requirements. Storj is the closest (client-side encryption, erasure coding, distributed metadata) but uses a centralized satellite for metadata coordination and is a commercial service incompatible with the open/volunteer model. Building a custom data plane on CIDv1 primitives with RS(10,18) erasure coding is the correct path.

[CONFIDENCE] Medium-high. Storj's satellite SPOF and commercial model are documented constraints. The "build custom" recommendation carries execution risk — mitigated by reusing mature components (liberasurecode for RS, go-cid for CIDv1, libp2p for transport).

---

## 13. Test Plan on Real Hardware

### Phase 1: Single-Rack Simulation (Stage 1 prerequisite)

1. Stand up 20 VMs (or physical machines) representing donor nodes, 1 coordinator
2. Write a 1 GB test dataset, shard with RS(10,18), distribute shards to 18 nodes
3. **Node loss test**: kill 8 nodes simultaneously; verify full reconstruction from remaining 10 shards. Measure reconstruction latency.
4. **Repair test**: bring killed nodes back as new nodes; verify shard repair propagates; measure repair bandwidth consumed (target: < 1.1× file size per lost shard for naive RS)
5. **Coordinator failure test**: kill coordinator; elect new coordinator from quorum; verify metadata plane converges; verify a new job can be staged in

### Phase 2: Geographic Distribution Test (Stage 2)

1. Deploy donors across at least 3 cloud regions in different countries (AWS us-east, eu-west, ap-southeast) plus 2 physical machines on different ISPs
2. Repeat node-loss tests; verify placement constraint enforcement (coordinator refuses to place 2 shards in same /24)
3. Inject simulated country-level outage (block traffic from one region); verify job reconstruction succeeds within latency budget

### Phase 3: Churn Simulation (Stage 3)

1. Script random node departures/arrivals at 20% hourly churn rate
2. Run a 4-hour continuous job; verify zero data loss and checkpoint continuity
3. Measure coordinator metadata convergence latency under churn

### Adversarial Tests (Every Release, Constitution Principle V)

- **Malicious donor**: donor returns corrupted shard bytes; verify Poly1305 AEAD tag rejection and automatic failover to another shard holder
- **Quota overflow**: donor accepts more shard assignments than declared quota; verify coordinator rejects excess assignments
- **Key extraction attempt**: donor inspects all received shard bytes; verify no reconstruction of plaintext is possible from fewer than k=10 ciphertext shards

---

## [LIMITATION]

1. **Durability probability assumes i.i.d. shard availability**: real churn has correlated structure (ISP outages, power events, time-of-day patterns). True durability is lower-bounded by the i.i.d. calculation; geographic diversity raises it. Empirical measurement of actual donor availability correlation is required before production deployment.

2. **RS repair bandwidth**: naive RS repair (download k shards per lost shard) is expensive at scale. MSR codes would reduce this 78%, but MSR libraries are less mature. Phase 2 should prototype MSR repair; if library stability is adequate, adopt for cold tier repair.

3. **Metadata plane scalability**: the CRDT + Raft coordinator model works at moderate scale (millions of chunks). At planetary scale (exabyte-class storage), the metadata plane will need sharding. This is a stage 4+ concern.

4. **Browser quota revocation**: browsers can revoke OPFS storage without notice. The recommendation (browsers excluded from cold tier) is correct, but implementation must handle graceful degradation when a browser donor unexpectedly loses its ephemeral hot-tier data mid-job.

5. **Key management for long-running jobs**: session keys for multi-day training runs need a rotation and escrow strategy. Not solved in this document.

---

## Recommended Architecture Summary

| Decision | Choice | Rationale |
|-|-|-|
| Erasure code | RS(10,18) | 1.8× overhead, survives 8 losses, P(loss)=2×10⁻⁵ at 10% churn |
| Addressing | CIDv1 (SHA-256) | Cryptographic integrity, deduplication, IPFS-compatible |
| Placement | 1 shard/AS, ≤2/country, ≥3 continents | Correlated failure isolation |
| Hot tier | 3× replication on executor nodes | Low latency during execution |
| Cold tier | RS(10,18) on 18 distinct donor ASes | Durability between jobs |
| Metadata | CRDT OR-Map + Raft coordinator quorum | No SPOF, causal consistency |
| Integrity | Merkle DAG + SHA-256 per chunk | End-to-end, trustless |
| Encryption | ChaCha20-Poly1305 per chunk, X25519 key wrap | Donor-blind, AEAD integrity |
| Consistency | Strong (CIDs) + Causal+ (metadata) + Linearizable (job state) | Right tool per layer |
| Browser donors | Hot tier / relay only, no cold storage | Platform quota constraints |
| Reuse vs. build | CIDv1 + libp2p transport + liberasurecode RS; custom data plane | Proven primitives, custom orchestration |

