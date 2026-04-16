# Research 09 — Distributed Mesh LLM for Self-Improvement

**Stage**: 9 (Mesh LLM / Collective Intelligence) of the World Compute core research series
**Date**: 2026-04-15
**Author**: Scientist agent
**Status**: Draft for synthesis review
**Constitutional anchors**: Principle IV (Efficiency & Self-Improvement), Principle I (Safety), Principle II (Robustness)
**Spec reference**: FR-033 (5–10% self-improvement capacity slice)

---

## 1. Executive Summary

**Viability assessment: PARTIALLY FEASIBLE — the ensemble-of-experts approach is viable today for non-interactive self-improvement workloads; the original "each node runs one transformer layer" vision is not viable over volunteer internet links.**

World Compute's self-improvement capacity (FR-033's 5–10% reserved slice) can be realized as a **distributed Mixture-of-Experts (MoE) ensemble** where each participating node runs a **complete small language model** and a lightweight **router** selects K-of-N models per token, aggregating their output distributions. This is architecturally distinct from — and significantly more practical than — pipeline-parallel distributed inference (e.g., Petals), because it requires only **one parallel network round-trip per token** rather than N sequential hops.

At 100ms inter-node latency with K=4 experts, the system achieves approximately **3.2 tokens/second** — too slow for interactive chat, but adequate for the intended use case: autonomous self-improvement agents that generate and evaluate scheduling policies, configuration changes, security analyses, and governance proposals on timescales of minutes to hours.

The minimum viable mesh requires approximately **280 total cluster nodes** (at 5% SI budget with 30% GPU donors) to sustain a single inference stream, or **140 nodes** at 10%. At 1,000+ nodes, multiple parallel agent streams become feasible. The system should standardize on the **LLaMA-3 tokenizer** (128K vocab) to maximize compatibility with the open-source ecosystem.

Key risks: straggler latency in heterogeneous networks, safety of self-modifying autonomous systems, and the cold-start problem (the mesh LLM cannot exist until the cluster is large enough to support it). A phased rollout — starting with a centralized small model, graduating to local ensemble, then full distributed mesh — is recommended.

[FINDING:F1] The ensemble-of-complete-models approach achieves 2–5x better latency than pipeline parallelism over volunteer internet links, because it replaces N sequential network hops with 1 parallel round-trip. [EVIDENCE:F1] Quantitative model: at 100ms network latency, K=4 ensemble = 314ms/token (3.2 tok/s) vs. 8-node pipeline = 864ms/token (1.2 tok/s) for equivalent model quality. Petals reports 0.3–1.0 tok/s for cross-internet pipeline inference, consistent with our model. [CONFIDENCE:HIGH]

[FINDING:F2] The minimum viable distributed mesh requires ~140–280 cluster nodes to sustain one inference stream within the 5–10% self-improvement budget. [EVIDENCE:F2] At 30% GPU donor rate, 5% SI budget: need 4 GPU experts / 0.30 / 0.05 = 267 nodes. At 10%: 134 nodes. [CONFIDENCE:MEDIUM — depends on actual GPU donor fraction, which is unknown pre-launch.]

---

## 2. Architecture Recommendation

### 2.1 Rejected: Per-Layer Pipeline Parallelism

The original vision — "each node runs one transformer layer" — suffers from a fatal latency problem over volunteer internet links. In pipeline parallelism, each token generation requires activation tensors to traverse **all N nodes sequentially**. With N=8 nodes at 100ms inter-node latency:

- Per-token latency: 8 × (100ms network + ~8ms compute) = **864ms** (1.16 tok/s)
- Per-token latency at N=32: **3,264ms** (0.31 tok/s)

This matches Petals' reported cross-internet performance of 0.3–1.0 tok/s. More critically, pipeline parallelism creates a **chain-of-dependencies**: if any single node in the pipeline is slow or fails, the entire inference stalls. In a volunteer compute environment with high churn (median donor session ~90 minutes per Research 01), this is unacceptable.

[FINDING:F3] Pipeline parallelism is architecturally incompatible with volunteer compute's churn and latency characteristics. A single slow or departing node in an N-node pipeline halts all inference. [EVIDENCE:F3] Pipeline requires N serial network hops; at 100ms/hop, N=8 already exceeds 800ms/token. Petals mitigates this with intra-datacenter deployment, which contradicts World Compute's volunteer model. [CONFIDENCE:HIGH]

Additionally, pipeline parallelism requires **all nodes to run layers from the same model architecture** with matching hidden dimensions. Node A's layer-12 from a 7B model (hidden_dim=4096) cannot feed into Node B's layer-45 from a 70B model (hidden_dim=8192). This eliminates heterogeneous participation, which is the entire point of volunteer compute.

### 2.2 Recommended: Ensemble-of-Experts with Learned Router

The recommended architecture treats each participating node as an **independent expert** running a **complete small language model**. A distributed router selects K-of-N experts per token and aggregates their output probability distributions.

```
                    ┌─────────────────────────────────────┐
                    │          ROUTER (replicated)         │
                    │  - Receives input token sequence     │
                    │  - Computes gating weights g_1..g_K  │
                    │  - Selects top-K of N experts        │
                    │  - Sends input to K experts (parallel)│
                    │  - Receives K prob distributions      │
                    │  - Computes weighted average          │
                    │  - Samples next token                 │
                    └────────┬──────────┬──────────┬───────┘
                             │          │          │
                    parallel │ requests │          │
                             ▼          ▼          ▼
                    ┌────────┐ ┌────────┐ ┌────────┐
                    │Expert 1│ │Expert 2│ │Expert K│
                    │LLaMA-3 │ │Mistral │ │Qwen-2  │  ← heterogeneous OK
                    │ 8B-Q4  │ │ 7B-Q4  │ │ 7B-Q4  │
                    │(Node A)│ │(Node B)│ │(Node C)│
                    └────────┘ └────────┘ └────────┘
```

**Per-token latency model:**

| K (experts) | Network (100ms) | Straggler compute | Total | Tok/s |
|-|-|-|-|-|
| 2 | 200ms RTT | 75ms | 285ms | 3.51 |
| 4 | 200ms RTT | 104ms | 314ms | 3.18 |
| 8 | 200ms RTT | 136ms | 346ms | 2.89 |
| 16 | 200ms RTT | 169ms | 379ms | 2.64 |

The straggler effect (expected maximum of K exponentially distributed compute times) grows only logarithmically with K, making the approach scalable.

**Why this works for self-improvement:**

The mesh LLM's job is not interactive conversation — it is **autonomous reasoning about the cluster itself**. At 3 tok/s, a 500-token analysis takes ~2.7 minutes. A 2,000-token policy proposal takes ~11 minutes. These timescales are perfectly acceptable for self-improvement tasks that operate on cycles of minutes to hours.

### 2.3 Data Transfer Optimization

Transmitting full probability distributions over the vocabulary is expensive:

- Full cl100k (100K vocab) at fp16: **196 KB per expert per token**
- Full LLaMA-3 (128K vocab) at fp16: **250 KB per expert per token**
- With K=8: **1.5–2.0 MB per token** — unsustainable at scale

**Solution: sparse top-k logit transmission.** Each expert returns only its top-256 (token_id, logit) pairs:

- Top-256 at 6 bytes each: **1.5 KB per expert per token**
- With K=8: **12 KB per token** — a 99.2% reduction
- Bandwidth at 3 tok/s: **36 KB/s** — trivially manageable on any connection

The router reconstructs a weighted sparse distribution from the union of all experts' top-256 sets and samples from it. Tokens not in any expert's top-256 are effectively probability-zero, which is acceptable because the probability mass outside top-256 is typically <0.1% for any well-trained model.

[FINDING:F4] Sparse top-k logit transmission reduces per-token bandwidth by 99%+ while preserving output quality, making the ensemble approach bandwidth-feasible even on residential connections. [EVIDENCE:F4] Top-256 logits per expert = 1.5 KB vs. full 128K-vocab distribution = 250 KB. Probability mass outside top-256 is typically <0.1% for calibrated language models (see nucleus sampling literature: Holtzman et al., 2020). [CONFIDENCE:HIGH for bandwidth reduction; MEDIUM for quality preservation — needs empirical validation.]

---

## 3. The Tokenizer Question

### 3.1 Survey of Tokenizer Families

No universal tokenizer exists across major open-source model families:

| Family | Tokenizer | Vocab Size |
|-|-|-|
| LLaMA-3 / 3.1 / 3.2 | tiktoken-based BPE | 128,256 |
| LLaMA-2 / Mistral-7B / Mixtral | SentencePiece BPE | 32,000 |
| Qwen-2 / 2.5 | tiktoken-based BPE | 151,936 |
| Gemma / Gemma-2 | SentencePiece | 256,000 |
| DeepSeek-V2/V3/R1 | BPE | 129,280 |
| GPT-2 / GPT-Neo / GPT-J | GPT-2 BPE | 50,257 |
| GPT-4 / ChatGPT | cl100k_base | 100,256 |

### 3.2 Resolution: Standardize on LLaMA-3 Tokenizer, with Adapter Layer

**Primary recommendation**: Standardize on the **LLaMA-3 tokenizer** (128,256 vocab) as the mesh's lingua franca.

**Rationale:**
1. **Largest actively-developed open ecosystem** — LLaMA-3, LLaMA-3.1, LLaMA-3.2 all use it; most new fine-tunes and community models build on this family.
2. **128K vocab** provides good coverage of multilingual text, code, and special tokens.
3. **tiktoken-compatible** — efficient, well-maintained, battle-tested encoding/decoding library.

**The shared-tokenizer constraint is soft, not hard, in the ensemble architecture.** Unlike pipeline parallelism (where hidden dimensions must match exactly between layers), the ensemble approach only requires that experts produce probability distributions that can be meaningfully aggregated. Two approaches:

**Approach A (recommended for v1): Homogeneous tokenizer.** All experts use the LLaMA-3 tokenizer. This means experts must be LLaMA-3-family models (or models re-tokenized/fine-tuned for this vocab). This is restrictive but simple and correct.

**Approach B (future): Vocabulary mapping layer.** Experts with different tokenizers include a learned linear projection that maps their native logits to a shared vocabulary space. This is equivalent to the "adapter head" approach in multilingual NMT. It enables heterogeneous models but adds complexity and potential distribution mismatch. Research-stage only.

[FINDING:F5] The shared-tokenizer constraint can be satisfied in v1 by standardizing on LLaMA-3's 128K-vocab tokenizer, which covers the largest open-source model ecosystem. Heterogeneous tokenizer support via vocabulary mapping is feasible but requires additional research. [EVIDENCE:F5] LLaMA-3 family (8B, 70B, 3.2-1B, 3.2-3B) all share the same tokenizer. Mistral-Nemo (12B) uses a compatible 131K vocab. The vocabulary mapping approach is validated by multilingual NMT (Johnson et al., 2017) but has not been tested for MoE ensemble aggregation. [CONFIDENCE:HIGH for v1 standardization; LOW for cross-tokenizer mapping.]

---

## 4. Heterogeneous Node Compatibility

### 4.1 The Hidden Dimension Problem (Pipeline)

In pipeline parallelism, layers from different architectures cannot be combined:
- LLaMA-3-8B: hidden_dim=4096, 32 layers, GQA with 8 KV heads
- LLaMA-3-70B: hidden_dim=8192, 80 layers, GQA with 8 KV heads
- Mistral-7B: hidden_dim=4096, 32 layers, sliding window attention

Even within the same hidden dimension (LLaMA-3-8B and Mistral-7B are both 4096), the weight matrices, layer norms, and attention patterns are incompatible. You cannot route layer 12 of LLaMA through layer 13 of Mistral.

### 4.2 The Ensemble Solution

The ensemble approach eliminates this problem entirely. Each expert is a complete, self-contained model. Heterogeneity manifests as:

- **Different model sizes**: Node A runs LLaMA-3-8B-Q4 (4GB VRAM), Node B runs LLaMA-3-3B-Q4 (2GB VRAM). Both produce valid probability distributions over the same vocabulary. The router's learned gating function can weight the 8B model higher for complex reasoning and the 3B model higher for simple continuations.
- **Different fine-tunes**: Node A runs a code-specialized fine-tune, Node B runs a general-purpose model. The router learns to route code-generation tasks preferentially to Node A — this is exactly how MoE gating is supposed to work.
- **Different quantization levels**: Q4, Q8, fp16 — all produce distributions over the same vocabulary, just with different precision. Quality differences are absorbed by the router weights.
- **CPU vs. GPU**: CPU nodes running llama.cpp with Q4 quantization can participate (at lower throughput, ~15 tok/s for 3B models). The router accounts for node latency in its selection.

[FINDING:F6] The ensemble architecture naturally accommodates heterogeneous hardware and model variants, provided all experts share the same tokenizer/vocabulary. This is the key advantage over pipeline parallelism for volunteer compute. [CONFIDENCE:HIGH]

### 4.3 Minimum Node Requirements

| Tier | Hardware | Model | VRAM/RAM | Local tok/s |
|-|-|-|-|-|
| GPU (high) | RTX 3090/4090 | LLaMA-3-8B-Q4 | 6 GB VRAM | 40–80 |
| GPU (mid) | RTX 3060 / M1 Pro | LLaMA-3-8B-Q4 | 4–6 GB VRAM | 20–40 |
| GPU (low) | GTX 1060 / M1 | LLaMA-3-3B-Q4 | 3 GB VRAM | 15–30 |
| CPU (high) | 32GB RAM, 8-core | LLaMA-3-3B-Q4 | 2 GB RAM | 8–15 |
| CPU (low) | 8GB RAM, 4-core | LLaMA-3-1B-Q4 | 1 GB RAM | 3–8 |

Nodes below the CPU-low tier (e.g., browser tabs, phones) should not participate in the mesh LLM but can contribute to other self-improvement workloads (data processing, log analysis, testing).

---

## 5. Router Design

### 5.1 Architecture

The router is a small, lightweight model that every participating node runs locally. It serves two functions:

1. **Gating**: Given an input token sequence, compute a score for each of the N available experts and select the top-K.
2. **Aggregation**: Receive the K sparse logit vectors, compute the weighted average, and sample the next token.

The router model is intentionally tiny — a 2-layer transformer or even an MLP over the last few token embeddings — so it runs in negligible time (<5ms) on any hardware.

### 5.2 How the Router Learns

**Phase 1 (bootstrap)**: The router uses **uniform random selection** — pick K random experts from those currently available and online. Aggregate with equal weights. This requires no training and works immediately.

**Phase 2 (capability discovery)**: The router collects performance data — which experts contribute useful probability mass for which types of inputs. A simple learned gating function (linear layer over input embeddings → N-dimensional score vector → top-K selection) is trained via **distillation**: compare the ensemble's output against a known-good reference (e.g., a single high-quality model's output on a curated evaluation set).

**Phase 3 (continuous adaptation)**: The router gating weights are updated online via a lightweight federated learning procedure: each router instance collects local performance metrics, computes gradient updates to the gating function, and shares compressed updates via GossipSub. This enables the mesh to adapt to changing expert availability and quality over time.

### 5.3 Straggler Mitigation

If one of the K selected experts is slow, the entire token generation stalls. Mitigation strategies:

1. **Timeout-and-fallback**: If an expert hasn't responded within 2× the expected latency, drop it and proceed with K-1 responses. The aggregation re-normalizes weights over the remaining experts.
2. **Speculative over-selection**: Select K+2 experts, use the first K responses that arrive. This adds marginal bandwidth cost but eliminates straggler dependency.
3. **Latency-aware routing**: The router's gating function includes a latency penalty term — consistently slow nodes receive lower gate scores and are selected less often.

[FINDING:F7] Speculative over-selection (request K+2, use first K) eliminates straggler dependency at the cost of ~25% additional bandwidth, which is acceptable given the sparse logit optimization. [EVIDENCE:F7] This is the standard approach in distributed systems (Google's "tail at scale" paper, Dean & Barroso, 2013). With sparse top-256 logits at 1.5 KB/expert, the overhead of 2 extra experts is 3 KB/token. [CONFIDENCE:HIGH]

---

## 6. Self-Prompting and Autonomous Agency

### 6.1 What the Mesh LLM Works On

The mesh LLM does not generate arbitrary text — it is a **purpose-built autonomous agent for cluster self-improvement**. Its task queue is derived from:

1. **Observability data**: The cluster's metrics pipeline (latency, throughput, error rates, churn rates, resource utilization) feeds into structured prompts. "Analyze the following 24h of scheduler metrics and propose efficiency improvements."
2. **Governance proposals**: Human governance participants submit improvement requests in natural language. The mesh LLM drafts implementation plans.
3. **Automated audits**: Periodic security scans, configuration drift detection, and performance regression analysis.
4. **Research tasks**: Literature surveys, algorithm comparisons, and prototype evaluations for cluster subsystems.

### 6.2 Agent Loop Architecture

The self-prompting loop follows a **deliberate, slow cadence** — not the rapid-fire loop of AutoGPT, but a measured cycle aligned with cluster operational timescales:

```
┌──────────────────────────────────────────────────────────────┐
│                    MESH LLM AGENT LOOP                       │
│                                                              │
│  1. OBSERVE: Ingest cluster metrics, logs, alerts (hourly)   │
│  2. ANALYZE: Generate analysis of cluster state (mesh LLM)   │
│  3. PROPOSE: Draft improvement actions (mesh LLM)            │
│  4. VALIDATE: Run proposed changes in sandbox (cluster jobs)  │
│  5. REVIEW: Human governance review for high-impact changes   │
│  6. APPLY: Deploy validated, approved changes                 │
│  7. MEASURE: Observe impact, feed back into step 1            │
│                                                              │
│  Cycle time: 1–24 hours depending on action class             │
└──────────────────────────────────────────────────────────────┘
```

### 6.3 Parallel Agent Subsets

At sufficient cluster scale (1,000+ nodes, yielding 3–7 parallel inference streams at 5% SI budget), the mesh can partition into **independent agent streams**, each working on a different self-improvement domain:

- **Stream A**: Scheduler optimization (analyze job latency, propose routing improvements)
- **Stream B**: Security analysis (audit sandbox configurations, scan for vulnerabilities)
- **Stream C**: Storage efficiency (analyze erasure coding overhead, propose compaction strategies)
- **Stream D**: Network optimization (analyze gossip overhead, propose topology improvements)

Each stream runs its own router + expert selection independently. Streams share results via the cluster's GossipSub messaging layer.

### 6.4 Prior Art: Autonomous Agent Frameworks

| Framework | Relevance | What to steal | What to avoid |
|-|-|-|-|
| AutoGPT | Pioneer of LLM self-prompting loops | Task decomposition pattern | Unbounded loop without validation; hallucination amplification |
| BabyAGI | Task prioritization and chaining | Priority queue of improvement tasks | Simplistic priority without governance |
| CrewAI | Multi-agent role specialization | Specialized agent roles per domain | Centralized orchestration assumption |
| LangGraph | Stateful agent graphs with cycles | Graph-based control flow for agent loop | Complex framework dependency |
| DSPy | Programmatic LLM pipeline optimization | Self-optimizing prompt pipelines | Requires training data from target domain |
| ReAct | Reasoning + action interleaving | Think-act-observe pattern | Single-agent, no distributed consideration |

**Key lesson from prior art**: Every autonomous agent framework that has been deployed in practice has required **human-in-the-loop checkpoints** for high-consequence actions. The mesh LLM must follow this pattern.

---

## 7. Safety and Sandboxing for Self-Modification

This is the most critical section. A self-improving system that can modify its own cluster is a powerful tool and a dangerous one. The following constraints are **non-negotiable** per Principle I.

### 7.1 Action Classification

All mesh LLM outputs are classified into action tiers with different approval requirements:

| Tier | Examples | Approval | Rollback |
|-|-|-|-|
| **Read-only** | Analyze metrics, generate reports, survey literature | None (automatic) | N/A |
| **Suggest** | Propose config changes, draft governance motions | Human review required | N/A (no action taken) |
| **Sandbox-test** | Run A/B experiment on 1% of traffic, test new scheduler policy in simulation | Automated validation + human spot-check | Automatic on regression |
| **Deploy-minor** | Update non-critical config parameters within pre-approved bounds | 2-of-3 governance quorum | Automatic canary rollback |
| **Deploy-major** | Change scheduler algorithm, modify sandbox policy, update security rules | Full governance vote + 24h review period | Manual rollback with audit |

### 7.2 Sandboxing Self-Modification

The mesh LLM's proposed changes are **never applied directly to the production cluster**. Instead:

1. Proposed changes are written to a **staging environment** — a shadow copy of the cluster's configuration.
2. A **simulation harness** replays the last 24h of cluster traffic against the proposed configuration and compares outcomes (latency, throughput, fairness metrics, safety metrics).
3. If simulation passes, a **canary deployment** applies the change to 1% of broker regions for 1 hour.
4. If canary passes, the change is promoted to full deployment — but only after the required approval tier is met.

### 7.3 Kill Switch

The mesh LLM has a **hardware kill switch**: any governance participant can issue a `MESH_LLM_HALT` command via signed message on GossipSub that immediately:
- Stops all self-improvement inference streams
- Reverts the last N applied changes (configurable, default N=3)
- Enters read-only mode pending governance review

This kill switch cannot be disabled or overridden by the mesh LLM itself.

[FINDING:F8] Tiered action classification with mandatory sandboxed simulation before any cluster modification is the minimum viable safety architecture for autonomous self-improvement. All prior art (AutoGPT, BabyAGI, production ML deployment systems) confirms that unvalidated autonomous deployment leads to cascading failures. [EVIDENCE:F8] Google's Safe Reinforcement Learning literature; OpenAI's RLHF alignment work; Netflix's canary deployment system (Spinnaker); the universal pattern of staged rollout in SRE practice. [CONFIDENCE:HIGH for the principle; MEDIUM for the specific tier boundaries, which need empirical calibration.]

---

## 8. Training and Adaptation

### 8.1 Can the Mesh Evolve?

Yes, through three mechanisms:

**Mechanism 1: Router weight updates (lightweight, continuous).** The router's gating function is trained continuously via federated averaging — each node computes local gradient updates based on its observed expert performance, and updates are shared via GossipSub. This adapts expert selection to changing availability and capability.

**Mechanism 2: LoRA adapter fine-tuning (medium-weight, periodic).** Individual expert nodes can train LoRA adapters on cluster-operational data (anonymized logs, scheduling traces, configuration histories). LoRA adapters are small (typically 1–10MB) and can be distributed as CIDv1 objects through the storage plane. This specializes experts for World Compute-specific tasks without modifying base model weights.

**Mechanism 3: Full expert retraining (heavyweight, rare).** On a longer timescale (quarterly or annually), the cluster can allocate a larger burst of self-improvement capacity to retrain or replace expert models entirely — e.g., upgrading from LLaMA-3-8B to a newer base model, or training a purpose-built model from scratch on curated cluster-improvement data.

### 8.2 Privacy Constraints

Training on cluster operational data must respect donor privacy per Principle I:

- **No donor-identifiable information** in training data. All logs are anonymized: Peer IDs are hashed, IP addresses stripped, geographic information coarsened to region level.
- **Differential privacy** applied to gradient updates in federated learning — gradient clipping + calibrated noise (epsilon=8, delta=1e-5 as starting parameters, tightened based on empirical analysis).
- **Opt-out**: Donors can opt out of having their node's operational data used for mesh LLM training. This is a per-node configuration flag.

[FINDING:F9] Federated LoRA fine-tuning is the most practical adaptation mechanism for the mesh LLM, balancing specialization capability against bandwidth cost and privacy risk. [EVIDENCE:F9] LoRA adapters are 0.1–1% of base model size (Hu et al., 2022); federated averaging with differential privacy is well-established (McMahan et al., 2017; Abadi et al., 2016). Bandwidth for sharing a 5MB LoRA adapter via GossipSub is negligible. [CONFIDENCE:MEDIUM — federated LoRA has been demonstrated in research but not at volunteer-compute scale.]

---

## 9. Resource Budget and Minimum Viable Scale

### 9.1 Capacity Model

Assumptions:
- 30% of donors have GPUs capable of running a quantized 7B model (~4GB VRAM)
- CPU-only donors can run quantized 1B–3B models
- Self-improvement budget: 5–10% of total cluster capacity (FR-033)
- Each inference stream requires K=4 GPU experts

| Cluster Size | SI Budget | GPU Nodes in SI | CPU Nodes in SI | Parallel Streams (K=4) |
|-|-|-|-|-|
| 100 | 5% | 1 | 4 | 0 (insufficient) |
| 100 | 10% | 3 | 7 | 0 (insufficient) |
| 280 | 5% | 4 | 10 | 1 (minimum viable) |
| 500 | 10% | 15 | 35 | 3 |
| 1,000 | 5% | 15 | 35 | 3 |
| 1,000 | 10% | 30 | 70 | 7 |
| 5,000 | 10% | 150 | 350 | 37 |
| 10,000 | 10% | 300 | 700 | 75 |

### 9.2 Minimum Viable Mesh

- **Single inference stream**: 280 nodes at 5% SI, or 140 nodes at 10%
- **Useful multi-agent**: 1,000+ nodes (3–7 parallel streams)
- **Full autonomous self-improvement**: 5,000+ nodes (37+ parallel streams)

### 9.3 Bandwidth Budget

With sparse top-256 logit transmission:
- Per-stream bandwidth: ~36 KB/s (negligible)
- Router gossip overhead: ~1 KB/s per node
- LoRA adapter distribution (periodic): 5–10 MB per update, amortized over hours

Total mesh LLM network overhead is **well under 1% of a typical residential connection** (50 Mbps = 6.25 MB/s).

---

## 10. Prior Art Survey

### 10.1 Petals (BigScience, 2022–present)

BitTorrent-style distributed inference for large language models. Nodes each host a subset of a model's layers; inference requests are routed through a pipeline of nodes.

- **Steal**: The concept of distributed inference over consumer hardware; the DHT-based routing for finding which nodes host which layers; the fault-tolerance approach (re-route around failed nodes).
- **Avoid**: Pipeline parallelism architecture (sequential hops kill latency over WAN); homogeneous model assumption (all nodes must host layers of the same model); no MoE/ensemble mechanism.
- **Performance**: 1–6 tok/s intra-datacenter; 0.3–1.0 tok/s cross-internet. [CONFIDENCE:HIGH]

### 10.2 Hivemind (Learning@home, 2020–present)

Decentralized deep learning training framework. Nodes contribute gradient computation; aggregation via decentralized all-reduce over libp2p.

- **Steal**: libp2p-based peer discovery and communication (directly compatible with World Compute's network stack from Research 05); decentralized averaging for federated learning; the DHT-based matchmaking for training participants.
- **Avoid**: Focus on training rather than inference; assumption of relatively stable long-running participants.
- **Relevance**: Hivemind's federated averaging protocol can be adapted for router weight updates and LoRA adapter sharing. [CONFIDENCE:HIGH]

### 10.3 Together.ai Distributed Inference

Commercial distributed inference platform using disaggregated serving across multiple GPU nodes.

- **Steal**: Disaggregated prefill/decode architecture (prefill on high-memory nodes, decode on high-throughput nodes); speculative decoding for latency reduction.
- **Avoid**: Datacenter-grade networking assumption; commercial/proprietary architecture.
- **Relevance**: The prefill/decode split is interesting for the mesh — large-context prefill could be done by high-memory nodes while decode (which is the latency-critical part) uses the ensemble. [CONFIDENCE:MEDIUM]

### 10.4 Swarm (Nous Research, 2024)

Distributed inference on consumer GPUs with a focus on open-source models.

- **Steal**: Consumer GPU targeting; community-driven model hosting; the demonstration that useful inference is possible on heterogeneous consumer hardware.
- **Avoid**: Centralized coordination; limited fault tolerance.
- **Relevance**: Validates the premise that consumer GPUs can contribute meaningfully to LLM inference. [CONFIDENCE:MEDIUM]

### 10.5 FriendliAI

Commercial serving optimization platform focused on batching and memory efficiency.

- **Steal**: Continuous batching techniques (can be applied within each expert node to serve multiple inference streams efficiently); memory-efficient attention implementations.
- **Avoid**: Single-cluster assumption; commercial dependency.
- **Relevance**: Expert nodes in the mesh should use continuous batching internally. [CONFIDENCE:MEDIUM]

### 10.6 Mixture-of-Experts Literature (Switch Transformer, Mixtral, DeepSeek-MoE)

The academic and industrial MoE literature provides the theoretical foundation for the ensemble approach.

- **Switch Transformer** (Fedus et al., 2022): Demonstrated that sparse expert routing works at scale; introduced the concept of expert capacity factors.
- **Mixtral-8x7B** (Mistral AI, 2024): Production MoE with 8 experts per layer, 2 active. Demonstrated that MoE achieves better quality-per-FLOP than dense models.
- **DeepSeek-MoE** (DeepSeek, 2024): Fine-grained expert segmentation with shared experts + routed experts.

**Key difference**: All of these are **intra-model** MoE (experts are feed-forward sub-layers within a single model). The mesh LLM is **inter-model** MoE (experts are complete independent models). The gating principle is the same; the granularity is different.

[FINDING:F10] The mesh LLM's inter-model MoE is theoretically grounded in the same principles as intra-model MoE (sparse gating, load balancing, expert specialization), but operates at a coarser granularity. No prior system has deployed inter-model MoE at the scale proposed here. [EVIDENCE:F10] Switch Transformer and Mixtral validate the MoE gating principle; Petals and Hivemind validate distributed inference over consumer hardware; but the combination (ensemble MoE over volunteer internet) is novel. [CONFIDENCE:MEDIUM — the individual components are validated, but their composition is unproven.]

---

## 11. Phased Rollout

### Phase 0: Centralized Bootstrap (v1, launch through ~500 nodes)

The cluster is too small for distributed mesh LLM. Instead:
- Run a single LLaMA-3-8B instance on a project-operated server as the "self-improvement brain."
- This centralized model performs the same agent loop (observe, analyze, propose, validate) but is not distributed.
- Begin collecting operational data for future distributed training.
- Ship the router protocol specification and expert node SDK so early adopters can test.

**Exit criteria**: 280+ nodes with GPU capability, validated router protocol, at least 3 months of operational data collected.

### Phase 1: Local Ensemble (v1.5, ~280–1,000 nodes)

- Enable distributed ensemble with K=2–4 experts.
- Router uses uniform random selection (no learned gating).
- Mesh LLM operates in **read-only + suggest mode** only — no automated deployment.
- All outputs reviewed by human governance.
- Validate latency, quality, and reliability empirically.

**Exit criteria**: 1,000+ nodes, validated ensemble quality comparable to centralized model, router gating function trained and tested.

### Phase 2: Multi-Stream Autonomous (v2, 1,000–5,000 nodes)

- Enable learned router gating.
- Multiple parallel agent streams for different self-improvement domains.
- Introduce sandbox-test action tier: mesh LLM can propose AND simulate changes automatically.
- Deploy-minor tier with governance quorum approval.
- Begin federated LoRA fine-tuning.

**Exit criteria**: 5,000+ nodes, demonstrated self-improvement actions that measurably improved cluster performance, safety audit passed.

### Phase 3: Full Self-Improvement (v3, 5,000+ nodes)

- Full action tier hierarchy including deploy-major with governance vote.
- Continuous router adaptation.
- Expert retraining pipeline.
- Cross-tokenizer vocabulary mapping (if research matures).
- Mesh LLM can propose and execute its own architecture improvements (meta-self-improvement), subject to governance approval.

---

## 12. Open Questions and Hard Blockers

### Hard Blockers

1. **Quality of inter-model MoE aggregation**: No one has demonstrated that averaging probability distributions from independently trained models produces outputs competitive with a single model of equivalent total parameter count. This is the central research risk. It MUST be validated empirically before Phase 1.

2. **Router cold-start with uniform selection**: Random expert selection may produce incoherent outputs if experts have very different training distributions. The transition from uniform to learned routing needs careful design.

### Open Questions

3. **Optimal K**: Is K=4 the right number of experts per token? Too few limits diversity; too many increases latency and bandwidth. Needs empirical sweep.

4. **Expert diversity vs. homogeneity**: Should we encourage donors to run diverse models (different fine-tunes, different sizes) or converge on one model? Diversity helps MoE quality but complicates the tokenizer constraint.

5. **KV-cache sharing**: In the ensemble approach, each expert maintains its own KV cache independently. Can KV cache state be shared or transferred between experts to reduce redundant computation? Research-stage.

6. **Prefill distribution**: For long-context inputs, the prefill phase (processing the prompt) is expensive. Can prefill be distributed across experts, or must each expert process the full prompt independently? If independently, the compute cost scales linearly with K.

7. **Evaluation methodology**: How do you measure the mesh LLM's quality in a principled way? Standard LLM benchmarks may not apply because the system is optimized for cluster-improvement tasks, not general-purpose generation.

8. **Constitutional alignment**: The mesh LLM must understand and respect the World Compute constitution. How is this enforced? System prompt? Fine-tuning? Constitutional AI techniques?

9. **Coordinator election for mesh LLM**: Who decides which nodes participate in the mesh LLM vs. other self-improvement workloads? This ties into the scheduler priority hierarchy (Research 01, Section 4).

10. **Cross-region expert selection**: Should the router prefer geographically nearby experts (lower latency) or diverse experts (better coverage)? Latency vs. quality tradeoff.

---

## 13. Coherence with Sibling Research Stages

- **Stage 1 (Job Management)**: Mesh LLM inference requests are WCJob tasks with `priority_tier: SELF_IMPROVEMENT`. The router is a workflow DAG that generates tokens sequentially, each token being a "micro-task." Expert selection uses the ClassAd matchmaking system with `gpu: required, model: llama3-family, tokenizer: llama3-128k` constraints. **Coherent.**
- **Stage 3 (Sandboxing)**: Expert models run inside the standard sandbox (Tier 1/2 for GPU, Tier 3 WASM not applicable for LLM inference). The mesh LLM's proposed cluster modifications run in a separate simulation sandbox before deployment. **Coherent.**
- **Stage 4 (Storage)**: LoRA adapters, router weights, and model checkpoints are CIDv1 objects in the RS(10,18) storage plane. KV cache is ephemeral and not stored. **Coherent.**
- **Stage 5 (Discovery)**: Expert nodes are discovered via the same libp2p/Kademlia/GossipSub stack. Expert capability advertisements include model family, quantization level, and available VRAM. Router gossip uses GossipSub topics. **Coherent.**
- **Stage 6 (Fairness/Credits)**: Self-improvement inference consumes the SELF_IMPROVEMENT priority tier, which is the lowest in the hierarchy (LOCAL_USER > DONOR_REDEMPTION > PAID_SPONSORED > PUBLIC_GOOD > SELF_IMPROVEMENT). Nodes contributing as mesh LLM experts earn standard compute credits. **Coherent.**

[FINDING:F11] The ensemble mesh LLM architecture is fully compatible with the existing World Compute research stages (1–7) without requiring renegotiation of any primitives. It consumes the existing job model, sandbox, storage, discovery, and credit systems as designed. [CONFIDENCE:HIGH]

---

## 14. Summary Table of Tagged Findings

| Tag | Claim | Evidence | Confidence |
|-|-|-|-|
| F1 | Ensemble achieves 2–5x better latency than pipeline over WAN | Quantitative model + Petals benchmarks | HIGH |
| F2 | Minimum viable mesh: 140–280 cluster nodes | Resource budget model | MEDIUM |
| F3 | Pipeline parallelism incompatible with volunteer compute | N serial hops × 100ms; churn fragility | HIGH |
| F4 | Sparse top-k logits reduce bandwidth 99%+ | 1.5 KB vs 250 KB per expert per token | HIGH |
| F5 | LLaMA-3 tokenizer is the right v1 standard | Largest open ecosystem; 128K vocab | HIGH |
| F6 | Ensemble handles heterogeneous hardware naturally | Complete models = independent outputs | HIGH |
| F7 | Speculative over-selection eliminates straggler dependency | Dean & Barroso 2013; 3 KB/token overhead | HIGH |
| F8 | Tiered action classification is minimum viable safety | Universal SRE practice; all agent framework lessons | HIGH |
| F9 | Federated LoRA is the practical adaptation mechanism | Hu et al. 2022; McMahan et al. 2017 | MEDIUM |
| F10 | Inter-model MoE is novel; individual components validated | Switch Transformer + Petals + Hivemind | MEDIUM |
| F11 | Architecture coherent with stages 1–7 | Cross-stage analysis | HIGH |
