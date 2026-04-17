# Mesh LLM Contract

## gRPC Service: MeshLLMService

### RegisterExpert(ExpertRegistration) -> ExpertStatus
- Input: expert_id, model_name, tokenizer, vram_mb, max_batch_size
- Output: registered (bool), router_id, assigned_shard
- Constraint: tokenizer must be "llama3" (128K vocab)

### GetRouterStatus() -> RouterStatus
- Output: expert_count, active_streams, tokens_per_second, health

### SubmitSelfTask(SelfTask) -> TaskReceipt
- Input: task_description, domain (scheduler | security | storage | network), priority
- Output: task_id, action_tier, approval_status
- Constraint: action_tier determines approval flow

### HaltMesh(HaltRequest) -> HaltConfirmation
- Input: actor_id (governance participant), reason
- Output: halted (bool), streams_stopped, changes_reverted
- Constraint: any governance participant can trigger; cannot be overridden by mesh itself

## Token Generation Protocol

1. Router receives prompt
2. Router selects K experts (default K=4) based on health, latency, load
3. Router sends prompt to K experts in parallel
4. Each expert runs local inference, returns top-256 (token_id, logit) pairs (~1.5KB)
5. Router aggregates: weighted average of logit distributions
6. Router samples next token from aggregated distribution
7. Repeat until EOS or max_tokens

## Action Tiers

| Tier | Approval | Examples |
|-|-|-|
| ReadOnly | None | Analyze metrics, generate reports |
| Suggest | Human review | Draft config changes, governance motions |
| SandboxTest | Automated validation | A/B experiment on 1% of traffic |
| DeployMinor | 2-of-3 quorum | Update non-critical config |
| DeployMajor | Full vote + 24h review | Change scheduler algorithm |

## Kill Switch

- Triggered by any governance participant via signed GossipSub message
- Immediately halts all inference streams
- Reverts last N applied changes (default N=3)
- Enters read-only mode
- Cannot be disabled or overridden by the mesh LLM itself
