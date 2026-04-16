# Prior Art: Distributed LLM Inference Across Volunteer Nodes

**Research Stage**: 3 — Prior Art Survey  
**Date**: 2026-04-15  
**Scope**: Systems for distributed/decentralized LLM inference, federated learning, autonomous agents, and Sybil resistance. Focus on what has been built and works vs. what is theoretical.

---

## Executive Summary

The honest picture: distributed LLM inference across untrusted volunteer nodes is **technically possible but operationally immature**. Petals proved the concept at modest scale (~100 active nodes). Exo works well on trusted LANs. SWARM Parallelism proved training viability on preemptible T4s with <400 Mbps. No system has demonstrated reliable, low-latency inference across fully untrusted, geographically dispersed volunteer hardware at scale. The hard problems — Byzantine node tolerance, sub-second yield to local users, layer-assignment rebalancing under churn — are unsolved in production. World Compute must solve them from first principles while borrowing infrastructure primitives from Petals/Hivemind and training insights from DisTrO/Nous Research.

---

## 1. Petals (BigScience)

**What it is**: BitTorrent-style distributed inference for large models (BLOOM 176B, LLaMA 3.1 up to 405B, Mixtral 8x22B, Falcon 40B+). Volunteer nodes each serve a contiguous slice of model layers; clients route tokens through the pipeline across nodes.

[FINDING] Petals achieves single-batch interactive inference at **4–6 tokens/sec** for 70B-class models across volunteer swarms.  
[EVIDENCE] "Single-batch inference runs at up to 6 tokens/sec for Llama 2 (70B) and up to 4 tokens/sec for Falcon (180B)" (Petals paper / petals.dev). Tested in two-continent real-world setup.  
[CONFIDENCE] High — from peer-reviewed ACL 2023 demo paper and project documentation.

[FINDING] Petals is **alive in 2026** but at modest scale — roughly tens to low hundreds of active nodes depending on the model.  
[EVIDENCE] health.petals.dev shows live node counts. The public swarm hosts Meta-Llama-3.1-405B-Instruct, Mixtral-8x22B-Instruct-v0.1, and bloom-560m. Community forks (Kwaai-AI-Lab/OpenAI-Petal, Agent Artificial) maintain compatible infrastructure.  
[CONFIDENCE] Medium — exact active-node counts at time of writing are not published in primary sources reviewed; the health monitor is live but exact numbers fluctuate.

**Failure modes on volunteer hardware**: Node dropout mid-sequence requires routing around the failed segment, causing latency spikes. The fault-tolerant autoregressive algorithm reassigns failed servers, but recovery is measured in seconds — unacceptable for streaming tokens. Stragglers in the pipeline bottleneck the entire sequence. No Byzantine-fault protection: a malicious node can silently corrupt activations.

**Relevance to World Compute**: Petals is the closest existence proof for the World Compute inference layer. Its layer-sharding, DHT-based peer discovery (via Hivemind), and fault-tolerant routing are directly adoptable. **Critical gaps**: no sandbox isolation, no donor-yield preemption, no cryptographic attestation of computation, no economic incentives for nodes.

**License**: Apache 2.0. Compatible.

---

## 2. Hivemind (Learning@home)

**What it is**: PyTorch library for decentralized deep learning over the internet. Provides the P2P substrate (Kademlia DHT scaling to tens of thousands of peers with O(log n) lookups), distributed optimizer primitives, and the Decentralized Mixture of Experts (DMoE) layer type. Petals is built on top of Hivemind.

[FINDING] Hivemind's DHT can scale to **tens of thousands of peers** with logarithmic search complexity.  
[EVIDENCE] Library documentation and paper: "Kademlia-based DHT that can scale to tens of thousands of peers with logarithmic search complexity."  
[CONFIDENCE] High — well-established Kademlia properties; Hivemind's implementation has been validated in Petals production use.

[FINDING] Hivemind DMoE layers are **inherently fault-tolerant**: if chosen experts fail, the model averages the remaining ones (treating failure as dropout).  
[EVIDENCE] "If some of the chosen experts fail to respond, the model will simply average the remaining ones and call that dropout."  
[CONFIDENCE] High — by design; documented in the library.

**Is it alive?** Yes. GitHub shows active issues and PRs through 2025. The NeurIPS 2021 "Training Transformers Together" demonstration trained a collaborative text-to-image Transformer across volunteers. Used by Petals in production.

**Relevance to World Compute**: Hivemind's DHT is a strong candidate for World Compute's peer-discovery and node-registry layer. The DMoE fault-tolerance model is directly relevant to routing around dead volunteer nodes. Its optimizer primitives (gradient compression, asynchronous SGD) are relevant for any future distributed training workload.

**License**: MIT. Compatible with Apache 2.0.

---

## 3. Exo (Exo Labs)

**What it is**: Open-source tool for running LLM inference across heterogeneous consumer devices (Macs, Linux boxes, Raspberry Pis, theoretically phones). Uses pipeline parallelism — splits model layers into shards assigned to devices. Auto-discovers peers via mDNS/Bonjour on local networks.

[FINDING] Exo achieves **99% latency reduction** between co-located devices with RDMA over Thunderbolt 5, and **3.2x speedup** with tensor parallelism across 4 homogeneous devices.  
[EVIDENCE] "RDMA over Thunderbolt 5 achieves 99% latency reduction... tensor parallelism delivers up to 3.2x speedup across 4 devices" (Exo Labs blog, "12 Days of EXO" benchmark series).  
[CONFIDENCE] High for LAN/Thunderbolt scenarios; these are controlled benchmarks, not volunteer-internet conditions.

[FINDING] Exo **degrades on heterogeneous clusters** — the lack of a sophisticated scheduler causes uneven device utilization.  
[EVIDENCE] "Heterogeneous engine clusters require careful optimization or homogeneous deployments for production use... requires a more sophisticated scheduler to ensure that device utilisation stays high."  
[CONFIDENCE] High — acknowledged by the Exo developers themselves.

[FINDING] Multi-request throughput scales **nearly linearly** with device count (2.2x for 3 devices), but single-request latency worsens when the model fits on one device.  
[EVIDENCE] Exo Labs benchmark blog: "multi-request throughput scales nearly linearly with the number of devices (2.2x for 3 devices). However, if a model fits on a single device, adding more devices will actually decrease single-request performance due to network overhead."  
[CONFIDENCE] High — direct benchmark data.

**Supported models**: LLaMA, Mistral, LlaVA, Qwen, DeepSeek.

**Failure modes**: Exo assumes trusted LAN nodes. No Byzantine tolerance. Discovery relies on mDNS (LAN-only by default). Adding internet routing requires manual configuration. No job checkpointing across node loss.

**Relevance to World Compute**: Exo's architecture and auto-discovery are excellent for the **intra-cluster** (LAN) portion of World Compute but insufficient for internet-scale volunteer federation. Its scheduler gap is precisely what World Compute must solve. Code is MIT-licensed and readable.

**License**: MIT. Compatible.

---

## 4. GPT4All / LocalAI / Ollama

**What they are**: Local inference runtimes for consumer hardware. Not distributed — each runs a full model on one machine. Relevant as the **single-node baseline** that World Compute volunteer nodes would use when serving a complete small model.

[FINDING] Ollama/LocalAI can serve 7B–70B models at **5–50 tokens/sec** on consumer hardware (M-series Macs, RTX 3090+).  
[EVIDENCE] Widely documented benchmarks across the community; local-llm-inference-tools-guide (blog.starmorph.com, 2026).  
[CONFIDENCE] High — consumer-reproducible.

**Relevance to World Compute**: These runtimes are the **inference backend** for individual World Compute nodes running complete small models. Ollama's REST API is the de facto standard interface; World Compute's agent could wrap it. Not relevant to layer-sharded distributed inference across multiple nodes.

**License**: MIT (Ollama), Apache 2.0 (LocalAI). Compatible.

---

## 5. Together.ai

**What it is**: Commercial distributed inference platform. Runs open-source models (LLaMA, Qwen, DeepSeek, Mixtral) on proprietary GPU clusters. Uses speculative decoding (ATLAS system), FP4/FP8 quantization, and custom CUDA kernels (Together Kernel Collection, built by FlashAttention author Tri Dao).

[FINDING] Together.ai's architecture is **closed-source proprietary infrastructure** built on owned/leased datacenter GPUs — not volunteer nodes.  
[EVIDENCE] "NVIDIA GPU Clusters: H100, H200, B200, GB200" (together.ai/instant-gpu-clusters). No public documentation of volunteer/federated architecture.  
[CONFIDENCE] High.

[FINDING] Together.ai's **open-source contributions** (FlashAttention, Together Kernel Collection) are more relevant to World Compute than their platform.  
[EVIDENCE] Chief Scientist Tri Dao created FlashAttention; Together Kernel Collection is open-sourced.  
[CONFIDENCE] High.

**Relevance to World Compute**: Together.ai is a **competitor model** (centralized commercial), not a building block. Their open-source kernel contributions (FlashAttention, etc.) are directly usable. Their ATLAS speculative decoding system demonstrates that learned draft models can accelerate inference — a technique World Compute could apply per node.

**License**: Platform is proprietary. Kernel contributions are open-source. FlashAttention is BSD-3.

---

## 6. Nous Research / Psyche Network / DisTrO

**What it is**: Nous Research is building the **Psyche Network** — a Solana-blockchain-anchored decentralized AI training platform using DisTrO (Distributed Training Over-the-Internet) and DeMo (Decoupled Momentum Optimisation). Raised $50M from Paradigm in April 2025. Focus is currently on **training**, not inference.

[FINDING] DisTrO reduces inter-GPU communication bandwidth requirements by up to **10,000x** during pre-training, enabling training over connections as slow as 100 Mbps down / 10 Mbps up.  
[EVIDENCE] "DisTrO reduces inter-GPU communication bandwidth requirements by up to 10,000X during pre-training... potentially as low as 100Mbps download and 10Mbps upload speeds" (Nous Research / VentureBeat coverage, 2025).  
[CONFIDENCE] Medium-High — reported figures from Nous; independent replication not yet widespread.

[FINDING] Nous Research's roadmap includes **accessible inference** as a future phase, not a current capability.  
[EVIDENCE] "Development roadmap consists of two main stages: cooperative training beginning with a permissioned testnet and transitioning toward a fully decentralized environment, and accessible inference and advanced capabilities" (Nous Research Psyche documentation).  
[CONFIDENCE] High — per their own roadmap.

**Relevance to World Compute**: DeMo/DisTrO's bandwidth compression techniques are highly relevant if World Compute ever supports distributed fine-tuning or training. The Solana-based incentive/accounting model is worth studying for the credit/fairness layer. Psyche's permissioned-testnet-first approach mirrors a sensible bootstrapping strategy. Currently **vaporware for inference** — training system is in early testnet.

**License**: Nous Hermes model weights are Apache 2.0. Psyche infrastructure: unclear/not yet released.

---

## 7. SWARM Parallelism

**What it is**: A 2023 ICML paper (from the Petals/Hivemind team) introducing Stochastically Wired Adaptively Rebalanced Model Parallelism — a training-focused distributed algorithm designed for swarms of heterogeneous, unreliable, poorly-connected devices.

[FINDING] SWARM trained a **1.1B shared-parameter Transformer** (≈13B before weight sharing) on preemptible T4 GPUs with **<400 Mbps** network throughput.  
[EVIDENCE] "Trained a large Transformer language model with 1.1B shared parameters (approximately 13B before sharing) on a swarm of preemptible T4 GPUs with less than 400Mb/s network throughput" (SWARM paper, ICML 2023, arXiv:2301.11913).  
[CONFIDENCE] High — peer-reviewed and reproduced.

[FINDING] SWARM inference of BLOOM-176B on consumer GPUs achieves approximately **1 step per second** — interactive but slow.  
[EVIDENCE] "Running inference of BLOOM-176B on consumer GPUs with approximately 1 step per second" (SWARM paper results).  
[CONFIDENCE] High.

**Key mechanism**: Creates temporary randomized pipelines; rebalances dynamically when nodes fail; weights assignment probability by node throughput (faster nodes get more work). This is the theoretical grounding for the Petals inference network.

**Relevance to World Compute**: SWARM's dynamic pipeline rebalancing algorithm is the **right abstraction** for World Compute's scheduler under high churn. The key insight — route tokens through whoever is available, weight by throughput — maps directly to World Compute's Principle II (robustness/graceful degradation).

**License**: Apache 2.0 (paper code).

---

## 8. Mixture of Experts in Practice

**What it is**: Architecture where each FFN layer is replaced by N experts with a learned gating network selecting top-K experts per token. Mixtral 8x7B (2 of 8 experts per token), Switch Transformer (1 expert per token), DeepSeek-V3 (256 experts, fine-grained routing).

[FINDING] MoE gating is **not naturally decomposable across network boundaries** — the gating decision requires knowing all expert activations, creating synchronization overhead.  
[EVIDENCE] "Sparse expert activation introduces irregular memory access patterns and frequent cross-device communication, resulting in elevated inference latency and hardware underutilization, and the stochastic nature of routing leads to unstable batching, fragmented workloads, and poor reproducibility" (MoE survey, HAL/arXiv 2025).  
[CONFIDENCE] High — fundamental architectural property.

[FINDING] The 2025 trend toward **many fine-grained experts** (DeepSeek-V3: 256 experts) makes cross-node MoE increasingly attractive — each node could host a subset of experts.  
[EVIDENCE] "Trend is shifting towards models with small parameters and many experts (e.g., DeepSeek-V3 with 256 experts), featuring fine-grained expert division and dynamic routing" (MoE survey 2025).  
[CONFIDENCE] High.

**Relevance to World Compute**: A "mesh LLM" where each volunteer node hosts a subset of MoE experts is architecturally plausible — World Compute nodes would function as expert servers, with the gating network routing tokens to the appropriate node. The central challenge is that gating introduces per-token network round-trips; batching and prefetching strategies can amortize this. This is the most architecturally natural fit for World Compute's heterogeneous node model.

---

## 9. Federated Learning (Flower, PySyft, Google FL)

**What it is**: Training paradigm where model updates (gradients or weights) are computed locally and aggregated by a coordinator, never sharing raw data. Flower (flwr) is the leading open-source framework; PySyft focuses on privacy-preserving ML.

[FINDING] Flower's **Photon** system (MLSys 2025) is the first rigorous system for federated end-to-end LLM pre-training, demonstrated on models up to tens of billions of parameters.  
[EVIDENCE] "Photon, Flower's extension for federated pre-training of large language models, was presented at MLSys 2025... After more than 12 months of public demonstrations... now establishes a new state-of-the-art in efficient and robust decentralized foundation model pre-training" (Flower blog, 2025).  
[CONFIDENCE] High.

[FINDING] Flower scores **84.75%** in comprehensive comparative FL framework analysis — the top performer across 15 frameworks.  
[EVIDENCE] Comparative analysis of open-source federated learning frameworks (Springer, 2024–2025).  
[CONFIDENCE] High.

**Failure modes**: Federated learning assumes nodes are honest (no Byzantine setting by default). Stragglers slow synchronous aggregation. Communication of gradients, even compressed, is non-trivial for billion-parameter models. PySyft requires manual strategy implementation (no built-in FedAvg).

**Relevance to World Compute**: Flower/Photon is directly relevant if World Compute supports **distributed fine-tuning** as a workload class. The framework handles straggler tolerance, client dropout, and aggregation. Not relevant to inference workloads. The Flower API is clean and adoptable.

**License**: Flower is Apache 2.0. PySyft is Apache 2.0. Compatible.

---

## 10. Autonomous Agent Frameworks (AutoGPT, BabyAGI, CrewAI, LangGraph)

**What they are**: Frameworks for self-prompting LLM loops: agents plan tasks, execute tool calls, observe results, and iterate.

[FINDING] In 2025, **no autonomous agent framework is production-reliable without significant human oversight** — all exhibit failure modes in unbounded loops.  
[EVIDENCE] "We are far from achieving truly autonomous, reliable, and cost-effective AI agents that can operate without significant human oversight" (DEV Community analysis, 2025). "When CrewAI prototypes are pushed into production environments, they often hit a 'complexity wall' with problems like unpredictable loops, cluttered context windows, and unclear inter-agent communication."  
[CONFIDENCE] High — consensus across multiple independent assessments.

[FINDING] **LangGraph** (graph-state machines) is the most production-ready framework for structured agent orchestration; **BabyAGI/AutoGPT** are research demonstrations not suitable for production.  
[EVIDENCE] "LangGraph is built for production from day one, using a structured, engineering-friendly model based on Graph Theory and State Machines." BabyAGI introduced a simple task loop "powered by GPT-4 and a vector database" — never hardened beyond prototype.  
[CONFIDENCE] High.

**Failure modes**: Infinite loops, context window overflow, hallucinated tool calls, compounding errors without recovery. Every additional capability is a new failure mode.

**Relevance to World Compute**: The World Compute self-improvement budget (Principle IV) — allocating cluster capacity to improve the scheduler, protocols, and research — is architecturally similar to an agentic loop. LangGraph's state-machine model is the right abstraction for building a **cluster self-management agent** that operates within explicit state boundaries. The failure modes of current agents are exactly what World Compute's direct-testing requirement (Principle V) would catch.

---

## 11. Proof of Personhood / Sybil Resistance

**What they are**: Mechanisms to ensure one human = one identity, preventing Sybil attacks (one adversary controlling many fake nodes).

[FINDING] **Worldcoin/World** has authenticated **>12 million unique individuals** using iris-scanning Orbs; 7,500 Orbs were planned for US deployment by end of 2025.  
[EVIDENCE] "Since World's inception less than two years ago, the protocol has collectively authenticated over 12 million unique individuals" (World.org blog). "7,500 Orbs across the United States alone by end of 2025" (World deployment plans).  
[CONFIDENCE] High for enrollment numbers; fraud rate not publicly disclosed.

[FINDING] **BrightID** (social graph vouching) and **Idena** (synchronous cognitive CAPTCHA) are functional but **small-scale** — neither has published user counts above low hundreds of thousands.  
[EVIDENCE] BrightID uses video "verification parties"; Idena uses synchronous flip-tests. Both lack the scale of Worldcoin. No fraud rate data found in 2025–2026 sources.  
[CONFIDENCE] Medium — absence of scale data is itself informative.

[FINDING] **Human Passport** (formerly Gitcoin Passport, acquired by Holonym in Feb 2025) aggregates multiple identity signals; plans for 34.5 million ZK credentials with 2 million existing users.  
[EVIDENCE] "Holonym acquires Gitcoin Passport in proof-of-personhood expansion" (Biometric Update, Feb 2025). "Human.tech plans to roll out 34.5 million zero-knowledge credentials."  
[CONFIDENCE] High for acquisition/plans; ZK credential rollout not yet confirmed complete.

**Relevance to World Compute**: Sybil resistance is essential for World Compute's fairness ledger (Principle III) — preventing one entity from registering thousands of fake donor nodes to drain cluster credits. No single system is clearly dominant:
- **Worldcoin**: highest scale, but biometric privacy concerns conflict with Principle I (donor privacy).
- **BrightID/Idena**: privacy-preserving, but small scale and UX friction.
- **Human Passport / ZK credentials**: best privacy-compute tradeoff; ZK proofs of humanity without revealing biometrics. Most aligned with World Compute values.
- Hybrid approach likely needed: ZK credential gate for cluster participation, with economic stake as secondary Sybil deterrent.

---

## Cross-Cutting Latency Reality Check

[FINDING] Realistic end-to-end token latency for pipeline-parallel inference across **internet volunteer nodes** is **0.5–5 seconds per token** depending on network conditions and pipeline depth.  
[EVIDENCE] Petals: 4–6 tokens/sec (≈167–250ms/token) in favorable conditions. SWARM BLOOM-176B: ~1 step/sec (1000ms/token). These are best-case numbers with cooperative, reasonably-connected nodes. High-latency or straggling nodes multiply these figures.  
[CONFIDENCE] Medium-High — derived from published benchmarks under controlled but not worst-case conditions.

[FINDING] **Minimum practical bandwidth** for layer-sharded internet inference: ~1–10 Mbps sustained per pipeline stage for activation tensors; DisTrO's training compression achieves 10,000x reduction but inference activations cannot be similarly compressed without accuracy loss.  
[EVIDENCE] SWARM training: <400 Mbps cluster aggregate. DisTrO inference roadmap: not yet demonstrated. Petals inference activations are small (hidden-state vectors), making inference more bandwidth-friendly than training (no gradient sync).  
[CONFIDENCE] Medium — inference activation bandwidth is genuinely low; the bottleneck is latency, not bandwidth.

---

## Build-vs-Adopt Summary for World Compute

| System | Real or Vaporware | License | Adoptable? | Gap |
|---|---|---|---|---|
| Petals | Real (modest scale) | Apache 2.0 | Yes — P2P inference layer | No sandbox, no attestation, no preemption |
| Hivemind | Real | MIT | Yes — DHT substrate | None blocking |
| Exo | Real (LAN-only) | MIT | Partial — single-node inference backend | Not internet-scale |
| Flower/Photon | Real | Apache 2.0 | Yes — for training workloads | Inference not supported |
| SWARM | Real (research) | Apache 2.0 | Yes — scheduler algorithm | No production implementation |
| Nous/DisTrO | Early testnet | Unclear | Borrow ideas | Training only, not inference |
| Together.ai | Real (commercial) | Proprietary | No — closed platform | Competing product |
| MoE experts-as-nodes | Theoretical | N/A | Design pattern only | No implementation |
| LangGraph | Real | MIT | Yes — self-management agent | Not distributed-compute-aware |
| Human Passport ZK | Real (early) | Open | Yes — Sybil resistance | Scale unproven |
| Worldcoin | Real (12M users) | Mixed | Partial — at cost to privacy | Biometric conflicts with Principle I |

**Recommended starting point**: Petals + Hivemind for the inference substrate. Flower for any training workloads. SWARM's rebalancing algorithm as the scheduling model. Human Passport ZK credentials (or equivalent) for Sybil resistance. LangGraph for the self-management agent loop.
